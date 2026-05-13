//! Owned Server-Sent Events stream parser.

use core::fmt;
use core::pin::Pin;
use core::task::Context;
use core::task::Poll;

use edgerun_futures::Stream;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub event: String,
    pub data: String,
    pub id: String,
    pub retry: Option<u64>,
}

#[derive(Debug)]
pub enum EventStreamError<E> {
    Stream(E),
    Utf8,
}

impl<E: fmt::Display> fmt::Display for EventStreamError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stream(error) => write!(f, "{error}"),
            Self::Utf8 => f.write_str("SSE stream contained invalid UTF-8"),
        }
    }
}

pub trait Eventsource: Stream + Sized {
    fn eventsource<B, E>(self) -> EventStream<Self>
    where
        Self: Stream<Item = Result<B, E>>,
        B: AsRef<[u8]>,
    {
        EventStream {
            stream: self,
            buffer: String::new(),
            last_event_id: String::new(),
        }
    }
}

impl<S: Stream> Eventsource for S {}

pub struct EventStream<S> {
    stream: S,
    buffer: String,
    last_event_id: String,
}

impl<S, B, E> Stream for EventStream<S>
where
    S: Stream<Item = Result<B, E>> + Unpin,
    B: AsRef<[u8]>,
{
    type Item = Result<Event, EventStreamError<E>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            if let Some((frame, consumed)) = next_frame(&self.buffer) {
                let frame = frame.to_string();
                self.buffer.drain(..consumed);
                if let Some(event) = parse_frame(&frame, &mut self.last_event_id) {
                    return Poll::Ready(Some(Ok(event)));
                }
                continue;
            }

            match Pin::new(&mut self.stream).poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Some(Ok(bytes))) => match core::str::from_utf8(bytes.as_ref()) {
                    Ok(text) => self.buffer.push_str(text),
                    Err(_) => return Poll::Ready(Some(Err(EventStreamError::Utf8))),
                },
                Poll::Ready(Some(Err(error))) => {
                    return Poll::Ready(Some(Err(EventStreamError::Stream(error))));
                }
                Poll::Ready(None) => {
                    if self.buffer.trim().is_empty() {
                        return Poll::Ready(None);
                    }
                    let frame = core::mem::take(&mut self.buffer);
                    return Poll::Ready(parse_frame(&frame, &mut self.last_event_id).map(Ok));
                }
            }
        }
    }
}

fn next_frame(buffer: &str) -> Option<(&str, usize)> {
    if let Some(index) = buffer.find("\r\n\r\n") {
        return Some((&buffer[..index], index + 4));
    }
    buffer
        .find("\n\n")
        .map(|index| (&buffer[..index], index + 2))
}

fn parse_frame(frame: &str, last_event_id: &mut String) -> Option<Event> {
    let mut event_name = String::from("message");
    let mut data = String::new();
    let mut retry = None;
    let mut saw_data = false;

    for raw_line in frame.lines() {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.is_empty() || line.starts_with(':') {
            continue;
        }

        let (field, value) = line.split_once(':').map_or((line, ""), |(field, value)| {
            (field, value.strip_prefix(' ').unwrap_or(value))
        });
        match field {
            "event" => event_name = value.to_string(),
            "data" => {
                if saw_data {
                    data.push('\n');
                }
                data.push_str(value);
                saw_data = true;
            }
            "id" => {
                if !value.contains('\0') {
                    last_event_id.clear();
                    last_event_id.push_str(value);
                }
            }
            "retry" => retry = value.parse::<u64>().ok(),
            _ => {}
        }
    }

    saw_data.then(|| Event {
        event: event_name,
        data,
        id: last_event_id.clone(),
        retry,
    })
}
