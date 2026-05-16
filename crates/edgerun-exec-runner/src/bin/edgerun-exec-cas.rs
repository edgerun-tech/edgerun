use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const MAGIC: &[u8; 4] = b"ERXR";
const VERSION: u16 = 1;
const MAX_BLOB_SIZE: u64 = 512 * 1024 * 1024;

const MSG_HELLO: u16 = 1;
const MSG_STDOUT: u16 = 10;
const MSG_STDERR: u16 = 11;
const MSG_EXIT: u16 = 12;
const MSG_ERROR: u16 = 13;
const MSG_LOG: u16 = 14;
const MSG_PUT_BEGIN: u16 = 20;
const MSG_PUT_CHUNK: u16 = 21;
const MSG_PUT_END: u16 = 22;
const MSG_PUT_OK: u16 = 23;
const MSG_EXEC: u16 = 24;
const MSG_BLOB_EXISTS: u16 = 25;

#[derive(Debug)]
struct Frame {
    msg_type: u16,
    seq: u64,
    payload: Vec<u8>,
}

#[derive(Debug)]
struct PutBegin {
    hash: [u8; 32],
    size: u64,
    mode: u32,
}

#[derive(Debug)]
struct ExecRequest {
    hash: [u8; 32],
    timeout_ms: u64,
    argv: Vec<String>,
    env: Vec<(String, String)>,
}

fn main() {
    if let Err(err) = real_main() {
        let _ = writeln!(io::stderr(), "edgerun-exec-cas fatal: {err}");
        std::process::exit(1);
    }
}

fn real_main() -> io::Result<()> {
    let root = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/run/edgerun"));

    bootstrap_linux_mounts();
    let blobs = root.join("blobs/sha256");
    let jobs = root.join("jobs");
    fs::create_dir_all(&blobs)?;
    fs::create_dir_all(&jobs)?;

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();

    send_frame(&mut output, MSG_HELLO, 0, b"edgerun-exec-cas v1")?;
    output.flush()?;

    loop {
        match read_frame(&mut input) {
            Ok(frame) => match frame.msg_type {
                MSG_PUT_BEGIN => {
                    if let Err(err) = handle_put(frame, &mut input, &mut output, &blobs) {
                        send_error(&mut output, 0, &format!("put failed: {err}"))?;
                        output.flush()?;
                    }
                }
                MSG_EXEC => {
                    if let Err(err) = handle_exec(frame, &mut output, &blobs, &jobs) {
                        send_error(&mut output, 0, &format!("exec failed: {err}"))?;
                        output.flush()?;
                    }
                }
                _ => {
                    send_error(&mut output, frame.seq, "expected PUT_BEGIN or EXEC")?;
                    output.flush()?;
                }
            },
            Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(err) => return Err(err),
        }
    }
}

fn bootstrap_linux_mounts() {
    run_quiet_mount(["-t", "proc", "proc", "/proc"]);
    run_quiet_mount(["-t", "sysfs", "sysfs", "/sys"]);
    run_quiet_mount(["-t", "devtmpfs", "devtmpfs", "/dev"]);
    run_quiet_mount(["-t", "tmpfs", "tmpfs", "/run"]);
}

fn run_quiet_mount<const N: usize>(args: [&str; N]) {
    let _ = Command::new("/bin/mount")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn handle_put<R: Read, W: Write>(
    first: Frame,
    input: &mut R,
    output: &mut W,
    blobs: &Path,
) -> io::Result<()> {
    let put = parse_put_begin(&first.payload)?;
    if put.size > MAX_BLOB_SIZE {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "blob too large"));
    }

    let path = blob_path(blobs, &put.hash);
    if path.exists() {
        drain_put_chunks(input, put.size)?;
        send_frame(output, MSG_BLOB_EXISTS, first.seq, hex32_bytes(&put.hash).as_bytes())?;
        output.flush()?;
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
    let mut file = OpenOptions::new().create(true).truncate(true).write(true).open(&tmp)?;
    let mut received = 0u64;

    while received < put.size {
        let frame = read_frame(input)?;
        if frame.msg_type != MSG_PUT_CHUNK {
            let _ = fs::remove_file(&tmp);
            return Err(io::Error::new(io::ErrorKind::InvalidData, "expected PUT_CHUNK"));
        }
        if frame.payload.len() > (put.size - received) as usize {
            let _ = fs::remove_file(&tmp);
            return Err(io::Error::new(io::ErrorKind::InvalidData, "chunk exceeds declared size"));
        }
        file.write_all(&frame.payload)?;
        received += frame.payload.len() as u64;
    }

    let end = read_frame(input)?;
    if end.msg_type != MSG_PUT_END {
        let _ = fs::remove_file(&tmp);
        return Err(io::Error::new(io::ErrorKind::InvalidData, "expected PUT_END"));
    }

    file.flush()?;
    drop(file);

    let got = sha256_file(&tmp)?;
    if got != put.hash {
        let _ = fs::remove_file(&tmp);
        send_error(output, first.seq, "sha256 mismatch")?;
        output.flush()?;
        return Ok(());
    }

    fs::set_permissions(&tmp, fs::Permissions::from_mode(put.mode & 0o777))?;
    fs::rename(&tmp, &path)?;
    send_frame(output, MSG_PUT_OK, first.seq, hex32_bytes(&put.hash).as_bytes())?;
    output.flush()?;
    Ok(())
}

fn drain_put_chunks<R: Read>(input: &mut R, size: u64) -> io::Result<()> {
    let mut received = 0u64;
    while received < size {
        let frame = read_frame(input)?;
        if frame.msg_type != MSG_PUT_CHUNK {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "expected PUT_CHUNK"));
        }
        received += frame.payload.len() as u64;
    }
    let end = read_frame(input)?;
    if end.msg_type != MSG_PUT_END {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "expected PUT_END"));
    }
    Ok(())
}

fn handle_exec<W: Write>(frame: Frame, output: &mut W, blobs: &Path, jobs: &Path) -> io::Result<()> {
    let exec = parse_exec_request(&frame.payload)?;
    let blob = blob_path(blobs, &exec.hash);
    if !blob.exists() {
        send_error(output, frame.seq, "blob not found")?;
        output.flush()?;
        return Ok(());
    }

    let job_dir = jobs.join(format!("{}.{}", hex32_bytes(&exec.hash), std::process::id()));
    if job_dir.exists() {
        fs::remove_dir_all(&job_dir)?;
    }
    fs::create_dir_all(&job_dir)?;

    send_log(output, frame.seq, &format!("executing sha256:{}", hex32_bytes(&exec.hash)))?;
    output.flush()?;

    let code = execute_and_stream(&blob, &job_dir, &exec, output)?;
    let mut payload = Vec::with_capacity(4);
    payload.extend_from_slice(&code.to_le_bytes());
    send_frame(output, MSG_EXIT, frame.seq, &payload)?;
    output.flush()?;
    let _ = fs::remove_dir_all(&job_dir);
    Ok(())
}

fn execute_and_stream<W: Write>(program: &Path, work_dir: &Path, exec: &ExecRequest, output: &mut W) -> io::Result<i32> {
    let mut command = Command::new(program);
    command.current_dir(work_dir);
    if exec.argv.len() > 1 {
        command.args(&exec.argv[1..]);
    }
    for (key, value) in &exec.env {
        command.env(key, value);
    }
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let mut child = command.spawn()?;
    let mut stdout = child.stdout.take().ok_or_else(|| io::Error::other("missing stdout pipe"))?;
    let mut stderr = child.stderr.take().ok_or_else(|| io::Error::other("missing stderr pipe"))?;

    let (tx, rx) = mpsc::channel::<(u16, Vec<u8>)>();
    let tx_out = tx.clone();
    thread::spawn(move || pipe_reader(MSG_STDOUT, &mut stdout, tx_out));
    let tx_err = tx.clone();
    thread::spawn(move || pipe_reader(MSG_STDERR, &mut stderr, tx_err));
    drop(tx);

    let deadline = if exec.timeout_ms == 0 { None } else { Some(Instant::now() + Duration::from_millis(exec.timeout_ms)) };
    let code = loop {
        while let Ok((msg_type, data)) = rx.try_recv() {
            send_frame(output, msg_type, 0, &data)?;
            output.flush()?;
        }
        if let Some(status) = child.try_wait()? {
            break status.code().unwrap_or(128 + status_signal_code(&status));
        }
        if let Some(deadline) = deadline {
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                break 124;
            }
        }
        thread::sleep(Duration::from_millis(5));
    };

    for (msg_type, data) in rx.try_iter() {
        send_frame(output, msg_type, 0, &data)?;
    }
    Ok(code)
}

#[cfg(unix)]
fn status_signal_code(status: &std::process::ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    status.signal().unwrap_or(0)
}

#[cfg(not(unix))]
fn status_signal_code(_status: &std::process::ExitStatus) -> i32 { 0 }

fn pipe_reader(msg_type: u16, reader: &mut dyn Read, tx: mpsc::Sender<(u16, Vec<u8>)>) {
    let mut buf = [0u8; 4096];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if tx.send((msg_type, buf[..n].to_vec())).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

fn blob_path(root: &Path, hash: &[u8; 32]) -> PathBuf {
    let hex = hex32_bytes(hash);
    root.join(&hex[0..2]).join(&hex[2..4]).join(hex)
}

fn sha256_file(path: &Path) -> io::Result<[u8; 32]> {
    let output = Command::new("sha256sum").arg(path).output()?;
    if !output.status.success() {
        return Err(io::Error::other("sha256sum failed"));
    }
    let text = std::str::from_utf8(&output.stdout).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "sha256sum utf8"))?;
    let hex = text.split_whitespace().next().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "sha256sum missing hash"))?;
    parse_hex32(hex)
}

fn parse_put_begin(payload: &[u8]) -> io::Result<PutBegin> {
    let mut c = Cursor::new(payload);
    let mut hash = [0u8; 32];
    c.read_exact(&mut hash)?;
    let size = c.read_u64()?;
    let mode = c.read_u32()?;
    Ok(PutBegin { hash, size, mode })
}

fn parse_exec_request(payload: &[u8]) -> io::Result<ExecRequest> {
    let mut c = Cursor::new(payload);
    let mut hash = [0u8; 32];
    c.read_exact(&mut hash)?;
    let timeout_ms = c.read_u64()?;
    let argc = c.read_u16()? as usize;
    let envc = c.read_u16()? as usize;
    let mut argv = Vec::with_capacity(argc.max(1));
    for _ in 0..argc {
        argv.push(c.read_string()?);
    }
    if argv.is_empty() {
        argv.push("./program".to_string());
    }
    let mut env = Vec::with_capacity(envc);
    for _ in 0..envc {
        let key = c.read_string()?;
        let value = c.read_string()?;
        env.push((key, value));
    }
    Ok(ExecRequest { hash, timeout_ms, argv, env })
}

fn read_frame<R: Read>(reader: &mut R) -> io::Result<Frame> {
    let mut header = [0u8; 24];
    reader.read_exact(&mut header)?;
    if &header[0..4] != MAGIC {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "bad magic"));
    }
    let version = u16::from_le_bytes([header[4], header[5]]);
    if version != VERSION {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "bad version"));
    }
    let msg_type = u16::from_le_bytes([header[6], header[7]]);
    let seq = u64::from_le_bytes(header[8..16].try_into().expect("header slice"));
    let len = u32::from_le_bytes(header[16..20].try_into().expect("header slice")) as usize;
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload)?;
    Ok(Frame { msg_type, seq, payload })
}

fn send_frame<W: Write>(writer: &mut W, msg_type: u16, seq: u64, payload: &[u8]) -> io::Result<()> {
    if payload.len() > u32::MAX as usize {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "payload too large"));
    }
    let mut header = [0u8; 24];
    header[0..4].copy_from_slice(MAGIC);
    header[4..6].copy_from_slice(&VERSION.to_le_bytes());
    header[6..8].copy_from_slice(&msg_type.to_le_bytes());
    header[8..16].copy_from_slice(&seq.to_le_bytes());
    header[16..20].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    writer.write_all(&header)?;
    writer.write_all(payload)?;
    Ok(())
}

fn send_error<W: Write>(writer: &mut W, seq: u64, message: &str) -> io::Result<()> {
    send_frame(writer, MSG_ERROR, seq, message.as_bytes())
}

fn send_log<W: Write>(writer: &mut W, seq: u64, message: &str) -> io::Result<()> {
    send_frame(writer, MSG_LOG, seq, message.as_bytes())
}

struct Cursor<'a> { data: &'a [u8], pos: usize }
impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self { Self { data, pos: 0 } }
    fn read_exact(&mut self, out: &mut [u8]) -> io::Result<()> {
        if out.len() > self.data.len().saturating_sub(self.pos) {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "payload truncated"));
        }
        out.copy_from_slice(&self.data[self.pos..self.pos + out.len()]);
        self.pos += out.len();
        Ok(())
    }
    fn read_u16(&mut self) -> io::Result<u16> {
        let mut b = [0; 2]; self.read_exact(&mut b)?; Ok(u16::from_le_bytes(b))
    }
    fn read_u32(&mut self) -> io::Result<u32> {
        let mut b = [0; 4]; self.read_exact(&mut b)?; Ok(u32::from_le_bytes(b))
    }
    fn read_u64(&mut self) -> io::Result<u64> {
        let mut b = [0; 8]; self.read_exact(&mut b)?; Ok(u64::from_le_bytes(b))
    }
    fn read_string(&mut self) -> io::Result<String> {
        let len = self.read_u16()? as usize;
        if len > self.data.len().saturating_sub(self.pos) {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "string truncated"));
        }
        let s = std::str::from_utf8(&self.data[self.pos..self.pos + len])
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid utf8"))?
            .to_string();
        self.pos += len;
        Ok(s)
    }
}

fn hex32_bytes(bytes: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in bytes {
        s.push(hex_digit(b >> 4));
        s.push(hex_digit(b & 0x0f));
    }
    s
}

fn hex_digit(n: u8) -> char {
    match n { 0..=9 => (b'0' + n) as char, _ => (b'a' + n - 10) as char }
}

fn parse_hex32(hex: &str) -> io::Result<[u8; 32]> {
    if hex.len() != 64 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "hash hex length"));
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = (hex_val(hex.as_bytes()[i * 2])? << 4) | hex_val(hex.as_bytes()[i * 2 + 1])?;
    }
    Ok(out)
}

fn hex_val(b: u8) -> io::Result<u8> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(io::Error::new(io::ErrorKind::InvalidData, "invalid hex")),
    }
}
