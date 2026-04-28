//! Logs command implementation.

use crate::prelude::*;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::time::Duration;

use crate::state::{container_state_dir, load_state};

pub fn cmd_logs(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    if let Some(ref root) = opts.root {
        crate::state::set_state_dir(root.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--root path is not valid UTF-8",
            )
        })?);
    }

    let (follow, id) = parse_logs_args(args)?;
    let _ = load_state(id)?;
    let state_dir = container_state_dir(id);
    let stdout = state_dir.join("stdout.log");
    let stderr = state_dir.join("stderr.log");

    if follow {
        follow_logs(&stdout, &stderr)
    } else {
        print_file(&stdout, io::stdout())?;
        print_file(&stderr, io::stderr())
    }
}

fn parse_logs_args(args: &[String]) -> io::Result<(bool, &str)> {
    let mut follow = false;
    let mut id = None;
    for arg in args {
        match arg.as_str() {
            "-f" | "--follow" => follow = true,
            _ if id.is_none() => id = Some(arg.as_str()),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Usage: ert logs [-f] <container-id>",
                ))
            }
        }
    }

    id.map(|id| (follow, id)).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Usage: ert logs [-f] <container-id>",
        )
    })
}

fn print_file<W: Write>(path: &PathBuf, mut writer: W) -> io::Result<()> {
    match File::open(path) {
        Ok(mut file) => {
            io::copy(&mut file, &mut writer)?;
            Ok(())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn follow_logs(stdout: &PathBuf, stderr: &PathBuf) -> io::Result<()> {
    let mut out_pos = print_file_from(stdout, 0, libc::STDOUT_FILENO)?;
    let mut err_pos = print_file_from(stderr, 0, libc::STDERR_FILENO)?;
    loop {
        let next_out = print_file_from(stdout, out_pos, libc::STDOUT_FILENO)?;
        let next_err = print_file_from(stderr, err_pos, libc::STDERR_FILENO)?;
        out_pos = next_out;
        err_pos = next_err;
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn print_file_from(path: &PathBuf, offset: u64, fd: i32) -> io::Result<u64> {
    use std::io::Seek;

    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(offset),
        Err(error) => return Err(error),
    };
    let size = file.metadata()?.len();
    let mut pos = offset.min(size);
    file.seek(io::SeekFrom::Start(pos))?;

    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        write_all_fd(fd, &buf[..n]);
        pos += n as u64;
    }
    Ok(pos)
}

fn write_all_fd(fd: i32, bytes: &[u8]) {
    let mut written = 0usize;
    while written < bytes.len() {
        let n = unsafe {
            libc::write(
                fd,
                bytes[written..].as_ptr() as *const libc::c_void,
                bytes.len() - written,
            )
        };
        if n <= 0 {
            break;
        }
        written += n as usize;
    }
}
