use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::{Arc, Mutex};

pub mod matchers {
    use super::{Matcher, Request};

    pub fn method(expected: &str) -> Matcher {
        let expected = expected.to_string();
        Matcher::new(move |request| request.method.eq_ignore_ascii_case(&expected))
    }

    pub fn path(expected: &str) -> Matcher {
        let expected = expected.to_string();
        Matcher::new(move |request| request.url.path() == expected)
    }

    pub fn header(name: &str, value: &str) -> Matcher {
        let name = name.to_ascii_lowercase();
        let value = value.to_string();
        Matcher::new(move |request| {
            request.headers.iter().any(|(header_name, header_value)| {
                header_name.eq_ignore_ascii_case(&name) && header_value == &value
            })
        })
    }

    pub fn header_regex(name: &str, pattern: &str) -> Matcher {
        header(name, pattern)
    }

    pub fn body_json<T>(_expected: T) -> Matcher {
        Matcher::new(|_request| true)
    }
}

#[derive(Clone)]
pub struct Matcher {
    inner: Arc<dyn Fn(&Request) -> bool + Send + Sync>,
}

impl Matcher {
    fn new(f: impl Fn(&Request) -> bool + Send + Sync + 'static) -> Self {
        Self { inner: Arc::new(f) }
    }

    fn matches(&self, request: &Request) -> bool {
        (self.inner)(request)
    }
}

#[derive(Clone, Debug)]
pub struct Request {
    pub method: String,
    pub url: RequestUrl,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct RequestUrl {
    path: String,
}

impl RequestUrl {
    pub fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Clone, Debug)]
pub struct ResponseTemplate {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl ResponseTemplate {
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn insert_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    pub fn set_body_json<T: serde::Serialize>(mut self, value: T) -> Self {
        self.headers
            .push(("content-type".to_string(), "application/json".to_string()));
        self.body = edgerun_json::to_vec(&value).expect("serialize mock JSON body");
        self
    }
}

pub trait Responder: Send + Sync {
    fn respond(&self, request: &Request) -> ResponseTemplate;
}

impl Responder for ResponseTemplate {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        self.clone()
    }
}

impl<F> Responder for F
where
    F: Fn(&Request) -> ResponseTemplate + Send + Sync,
{
    fn respond(&self, request: &Request) -> ResponseTemplate {
        self(request)
    }
}

struct MountedMock {
    matchers: Vec<Matcher>,
    responder: Box<dyn Responder>,
}

#[derive(Default)]
struct State {
    mocks: Vec<MountedMock>,
    requests: Vec<Request>,
}

#[derive(Clone)]
pub struct MockServer {
    addr: SocketAddr,
    state: Arc<Mutex<State>>,
}

impl MockServer {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
        let addr = listener.local_addr().expect("mock server addr");
        let state = Arc::new(Mutex::new(State::default()));
        let thread_state = Arc::clone(&state);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else {
                    continue;
                };
                handle_stream(&mut stream, &thread_state);
            }
        });
        Self { addr, state }
    }

    pub fn uri(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub async fn received_requests(&self) -> std::io::Result<Vec<Request>> {
        Ok(self.state.lock().expect("mock state").requests.clone())
    }
}

pub struct Mock {
    matchers: Vec<Matcher>,
    responder: Option<Box<dyn Responder>>,
}

impl Mock {
    pub fn given(matcher: Matcher) -> Self {
        Self {
            matchers: vec![matcher],
            responder: None,
        }
    }

    pub fn and(mut self, matcher: Matcher) -> Self {
        self.matchers.push(matcher);
        self
    }

    pub fn respond_with<R>(mut self, responder: R) -> Self
    where
        R: Responder + 'static,
    {
        self.responder = Some(Box::new(responder));
        self
    }

    pub fn expect(self, _count: u64) -> Self {
        self
    }

    pub async fn mount(self, server: &MockServer) {
        server
            .state
            .lock()
            .expect("mock state")
            .mocks
            .push(MountedMock {
                matchers: self.matchers,
                responder: self.responder.expect("mock responder"),
            });
    }
}

fn handle_stream(stream: &mut std::net::TcpStream, state: &Arc<Mutex<State>>) {
    let mut bytes = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => return,
            Ok(n) => {
                bytes.extend_from_slice(&buf[..n]);
                if request_complete(&bytes) {
                    break;
                }
            }
            Err(_) => return,
        }
    }

    let Some(request) = parse_request(&bytes) else {
        let _ = write_response(stream, ResponseTemplate::new(400));
        return;
    };

    let response = {
        let mut state = state.lock().expect("mock state");
        state.requests.push(request.clone());
        state
            .mocks
            .iter()
            .find(|mock| {
                mock.matchers
                    .iter()
                    .all(|matcher| matcher.matches(&request))
            })
            .map(|mock| mock.responder.respond(&request))
            .unwrap_or_else(|| ResponseTemplate::new(404))
    };

    let _ = write_response(stream, response);
}

fn request_complete(bytes: &[u8]) -> bool {
    let Some(header_end) = find_header_end(bytes) else {
        return false;
    };
    let headers = String::from_utf8_lossy(&bytes[..header_end]);
    let content_length = headers
        .lines()
        .find_map(|line| line.split_once(':'))
        .and_then(|(name, value)| {
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    bytes.len() >= header_end + 4 + content_length
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn parse_request(bytes: &[u8]) -> Option<Request> {
    let header_end = find_header_end(bytes)?;
    let header_text = String::from_utf8_lossy(&bytes[..header_end]);
    let mut lines = header_text.lines();
    let first = lines.next()?;
    let mut first_parts = first.split_whitespace();
    let method = first_parts.next()?.to_string();
    let target = first_parts.next()?.to_string();
    let path = target
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(&target);
    let mut headers = Vec::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_string(), value.trim().to_string()));
        }
    }
    let body = bytes[header_end + 4..].to_vec();
    Some(Request {
        method,
        url: RequestUrl {
            path: path.to_string(),
        },
        headers,
        body,
    })
}

fn write_response(
    stream: &mut std::net::TcpStream,
    response: ResponseTemplate,
) -> std::io::Result<()> {
    let reason = match response.status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "OK",
    };
    write!(stream, "HTTP/1.1 {} {}\r\n", response.status, reason)?;
    let mut has_length = false;
    for (name, value) in &response.headers {
        if name.eq_ignore_ascii_case("content-length") {
            has_length = true;
        }
        write!(stream, "{name}: {value}\r\n")?;
    }
    if !has_length {
        write!(stream, "content-length: {}\r\n", response.body.len())?;
    }
    write!(stream, "connection: close\r\n\r\n")?;
    stream.write_all(&response.body)?;
    stream.flush()
}
