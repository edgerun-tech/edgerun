use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAGIC: &[u8; 4] = b"ERXR";
const VERSION: u16 = 1;
const MAX_PROGRAM_SIZE: u64 = 512 * 1024 * 1024;
const DEFAULT_TIMEOUT_MS: u64 = 0;

const MSG_HELLO: u16 = 1;
const MSG_EXEC_BEGIN: u16 = 2;
const MSG_EXEC_CHUNK: u16 = 3;
const MSG_EXEC_END: u16 = 4;
const MSG_STDOUT: u16 = 10;
const MSG_STDERR: u16 = 11;
const MSG_EXIT: u16 = 12;
const MSG_ERROR: u16 = 13;
const MSG_LOG: u16 = 14;

#[derive(Debug)]
struct Frame {
    msg_type: u16,
    seq: u64,
    payload: Vec<u8>,
}

#[derive(Debug)]
struct ExecBegin {
    job_id: [u8; 32],
    size: u64,
    sha256: [u8; 32],
    timeout_ms: u64,
    argv: Vec<String>,
    env: Vec<(String, String)>,
}

fn main() {
    if let Err(err) = real_main() {
        let _ = writeln!(io::stderr(), "edgerun-exec-runner fatal: {err}");
        std::process::exit(1);
    }
}

fn real_main() -> io::Result<()> {
    let mut args = env::args().skip(1);
    let work_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/run/edgerun/jobs"));

    bootstrap_linux_mounts();
    fs::create_dir_all(&work_dir)?;

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();

    send_frame(&mut output, MSG_HELLO, 0, b"edgerun-exec-runner v1")?;
    output.flush()?;

    loop {
        match read_frame(&mut input) {
            Ok(frame) => {
                if frame.msg_type == MSG_EXEC_BEGIN {
                    if let Err(err) = handle_job(frame, &mut input, &mut output, &work_dir) {
                        send_error(&mut output, 0, &format!("job failed: {err}"))?;
                        output.flush()?;
                    }
                } else {
                    send_error(&mut output, frame.seq, "expected EXEC_BEGIN")?;
                }
            }
            Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(err) => return Err(err),
        }
    }
}

fn bootstrap_linux_mounts() {
    let _ = Command::new("/bin/mount").args(["-t", "proc", "proc", "/proc"]).status();
    let _ = Command::new("/bin/mount").args(["-t", "sysfs", "sysfs", "/sys"]).status();
    let _ = Command::new("/bin/mount").args(["-t", "devtmpfs", "devtmpfs", "/dev"]).status();
    let _ = Command::new("/bin/mount").args(["-t", "tmpfs", "tmpfs", "/run"]).status();
}

fn handle_job<R: Read, W: Write>(
    first: Frame,
    input: &mut R,
    output: &mut W,
    work_root: &Path,
) -> io::Result<()> {
    let begin = parse_exec_begin(&first.payload)?;
    if begin.size > MAX_PROGRAM_SIZE {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "program too large"));
    }

    let job_dir = work_root.join(hex32(&begin.job_id));
    if job_dir.exists() {
        fs::remove_dir_all(&job_dir)?;
    }
    fs::create_dir_all(&job_dir)?;
    let program_path = job_dir.join("program");
    let mut file = OpenOptions::new().create_new(true).write(true).open(&program_path)?;

    let mut received = 0u64;
    let mut hasher = Sha256::new();

    while received < begin.size {
        let frame = read_frame(input)?;
        if frame.msg_type != MSG_EXEC_CHUNK {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "expected EXEC_CHUNK"));
        }
        if frame.payload.len() > (begin.size - received) as usize {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "chunk exceeds declared size"));
        }
        hasher.update(&frame.payload);
        file.write_all(&frame.payload)?;
        received += frame.payload.len() as u64;
    }

    let end = read_frame(input)?;
    if end.msg_type != MSG_EXEC_END {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "expected EXEC_END"));
    }

    file.flush()?;
    drop(file);

    let got_hash = hasher.finalize();
    if got_hash != begin.sha256 {
        let _ = fs::remove_dir_all(&job_dir);
        send_error(output, first.seq, "sha256 mismatch")?;
        return Ok(());
    }

    fs::set_permissions(&program_path, fs::Permissions::from_mode(0o700))?;
    send_log(output, first.seq, &format!("hash ok; executing {} bytes", begin.size))?;
    output.flush()?;

    let exit_code = execute_and_stream(&program_path, &job_dir, &begin, output)?;
    let mut payload = Vec::with_capacity(4);
    payload.extend_from_slice(&exit_code.to_le_bytes());
    send_frame(output, MSG_EXIT, first.seq, &payload)?;
    output.flush()?;

    let _ = fs::remove_dir_all(&job_dir);
    Ok(())
}

fn execute_and_stream<W: Write>(
    program_path: &Path,
    job_dir: &Path,
    begin: &ExecBegin,
    output: &mut W,
) -> io::Result<i32> {
    let mut command = Command::new(program_path);
    command.current_dir(job_dir);
    if begin.argv.len() > 1 {
        command.args(&begin.argv[1..]);
    }
    for (key, value) in &begin.env {
        command.env(key, value);
    }
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let mut child = command.spawn()?;
    let mut stdout = child.stdout.take().ok_or_else(|| io::Error::new(io::ErrorKind::Other, "missing stdout pipe"))?;
    let mut stderr = child.stderr.take().ok_or_else(|| io::Error::new(io::ErrorKind::Other, "missing stderr pipe"))?;

    let (tx, rx) = mpsc::channel::<(u16, Vec<u8>)>();
    let tx_out = tx.clone();
    thread::spawn(move || pipe_reader(MSG_STDOUT, &mut stdout, tx_out));
    let tx_err = tx.clone();
    thread::spawn(move || pipe_reader(MSG_STDERR, &mut stderr, tx_err));
    drop(tx);

    let deadline = if begin.timeout_ms == 0 { None } else { Some(Instant::now() + Duration::from_millis(begin.timeout_ms)) };
    let mut exit_code = None;

    loop {
        while let Ok((msg_type, data)) = rx.try_recv() {
            send_frame(output, msg_type, 0, &data)?;
            output.flush()?;
        }

        if let Some(status) = child.try_wait()? {
            exit_code = Some(status.code().unwrap_or(128 + status_signal_code(&status)));
            break;
        }

        if let Some(deadline) = deadline {
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                exit_code = Some(124);
                break;
            }
        }

        thread::sleep(Duration::from_millis(5));
    }

    for (msg_type, data) in rx.try_iter() {
        send_frame(output, msg_type, 0, &data)?;
    }

    Ok(exit_code.unwrap_or(127))
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
    let seq = u64::from_le_bytes(header[8..16].try_into().unwrap());
    let len = u32::from_le_bytes(header[16..20].try_into().unwrap()) as usize;
    let _reserved = u32::from_le_bytes(header[20..24].try_into().unwrap());
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

fn parse_exec_begin(payload: &[u8]) -> io::Result<ExecBegin> {
    let mut cursor = Cursor::new(payload);
    let mut job_id = [0u8; 32];
    cursor.read_exact(&mut job_id)?;
    let size = cursor.read_u64()?;
    let mut sha256 = [0u8; 32];
    cursor.read_exact(&mut sha256)?;
    let timeout_ms = cursor.read_u64()?;
    let argc = cursor.read_u16()? as usize;
    let envc = cursor.read_u16()? as usize;
    let mut argv = Vec::with_capacity(argc.max(1));
    for _ in 0..argc {
        argv.push(cursor.read_string()?);
    }
    if argv.is_empty() {
        argv.push("./program".to_string());
    }
    let mut env = Vec::with_capacity(envc);
    for _ in 0..envc {
        let key = cursor.read_string()?;
        let value = cursor.read_string()?;
        env.push((key, value));
    }
    Ok(ExecBegin { job_id, size, sha256, timeout_ms, argv, env })
}

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

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
        let mut b = [0u8; 2];
        self.read_exact(&mut b)?;
        Ok(u16::from_le_bytes(b))
    }
    fn read_u64(&mut self) -> io::Result<u64> {
        let mut b = [0u8; 8];
        self.read_exact(&mut b)?;
        Ok(u64::from_le_bytes(b))
    }
    fn read_string(&mut self) -> io::Result<String> {
        let len = self.read_u16()? as usize;
        if len > self.data.len().saturating_sub(self.pos) {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "string truncated"));
        }
        let s = std::str::from_utf8(&self.data[self.pos..self.pos + len])
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid utf8 string"))?
            .to_string();
        self.pos += len;
        Ok(s)
    }
}

fn hex32(bytes: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for byte in bytes {
        s.push(hex_digit(byte >> 4));
        s.push(hex_digit(byte & 0x0f));
    }
    s
}

fn hex_digit(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        _ => (b'a' + n - 10) as char,
    }
}

struct Sha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    buffer_len: usize,
    bit_len: u64,
}

impl Sha256 {
    fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
                0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
            ],
            buffer: [0; 64],
            buffer_len: 0,
            bit_len: 0,
        }
    }

    fn update(&mut self, mut data: &[u8]) {
        self.bit_len = self.bit_len.wrapping_add((data.len() as u64) * 8);
        if self.buffer_len > 0 {
            let take = (64 - self.buffer_len).min(data.len());
            self.buffer[self.buffer_len..self.buffer_len + take].copy_from_slice(&data[..take]);
            self.buffer_len += take;
            data = &data[take..];
            if self.buffer_len == 64 {
                let block = self.buffer;
                self.compress(&block);
                self.buffer_len = 0;
            }
        }
        while data.len() >= 64 {
            let mut block = [0u8; 64];
            block.copy_from_slice(&data[..64]);
            self.compress(&block);
            data = &data[64..];
        }
        if !data.is_empty() {
            self.buffer[..data.len()].copy_from_slice(data);
            self.buffer_len = data.len();
        }
    }

    fn finalize(mut self) -> [u8; 32] {
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;
        if self.buffer_len > 56 {
            for b in &mut self.buffer[self.buffer_len..] { *b = 0; }
            let block = self.buffer;
            self.compress(&block);
            self.buffer_len = 0;
        }
        for b in &mut self.buffer[self.buffer_len..56] { *b = 0; }
        self.buffer[56..64].copy_from_slice(&self.bit_len.to_be_bytes());
        let block = self.buffer;
        self.compress(&block);

        let mut out = [0u8; 32];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    fn compress(&mut self, block: &[u8; 64]) {
        const K: [u32; 64] = [
            0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
            0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
            0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
            0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
            0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
            0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
            0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
            0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2,
        ];
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(block[i*4..i*4+4].try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3);
            let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10);
            w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }
        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        let mut e = self.state[4];
        let mut f = self.state[5];
        let mut g = self.state[6];
        let mut h = self.state[7];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }
}
