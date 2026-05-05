//! Logs command implementation.

use crate::libc;
use crate::prelude::*;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::time::Duration;

use crate::cli::{invalid_input, parse_cli_args, required_positional};
use crate::state::{container_state_dir, load_state};
use edgerun_clap::cli::Action;
use edgerun_clap::{Arg, Command};

pub fn cmd_logs(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    crate::cli::apply_global_opts(opts)?;

    let opts = parse_logs_args(args)?;
    let id = opts.id;
    let _ = load_state(&id)?;
    let state_dir = container_state_dir(&id);
    let stdout = state_dir.join("stdout.log");
    let stderr = state_dir.join("stderr.log");

    if opts.follow {
        follow_logs(&id, &stdout, &stderr, opts.tail)
    } else {
        print_file(&stdout, io::stdout(), opts.tail)?;
        print_file(&stderr, io::stderr(), opts.tail)
    }
}

#[derive(Clone, Debug)]
struct LogsArgs {
    follow: bool,
    tail: Option<usize>,
    id: String,
}

fn parse_logs_args(args: &[String]) -> io::Result<LogsArgs> {
    const USAGE: &str = "Usage: ert logs [-f] [--tail N] <container-id>";
    let matches = parse_cli_args(
        Command::new("logs")
            .arg(
                Arg::new("follow")
                    .short('f')
                    .long("follow")
                    .action(Action::StoreTrue),
            )
            .arg(Arg::new("tail").long("tail")),
        args,
        USAGE,
    )?;
    if matches.positional_count() > 1 {
        return Err(invalid_input(USAGE));
    }
    let id = required_positional(&matches, 0, USAGE)?.to_string();
    crate::cli::validate_container_id(&id)?;
    let tail = matches
        .get_one::<String>("tail")
        .map(|value| parse_tail(&value))
        .transpose()?;
    Ok(LogsArgs {
        follow: matches.get_flag("follow"),
        tail,
        id,
    })
}

fn parse_tail(value: &str) -> io::Result<usize> {
    value
        .parse::<usize>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "--tail must be a number"))
}

fn print_file<W: Write>(path: &PathBuf, mut writer: W, tail: Option<usize>) -> io::Result<()> {
    match File::open(path) {
        Ok(mut file) => {
            if let Some(lines) = tail {
                let mut data = Vec::new();
                file.read_to_end(&mut data)?;
                writer.write_all(tail_lines(&data, lines))?;
            } else {
                io::copy(&mut file, &mut writer)?;
            }
            Ok(())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn follow_logs(
    id: &str,
    stdout: &PathBuf,
    stderr: &PathBuf,
    tail: Option<usize>,
) -> io::Result<()> {
    let out_start = tail
        .map(|lines| tail_start_offset(stdout, lines))
        .transpose()?
        .unwrap_or(0);
    let err_start = tail
        .map(|lines| tail_start_offset(stderr, lines))
        .transpose()?
        .unwrap_or(0);
    let mut out_pos = print_file_from(stdout, out_start, libc::STDOUT_FILENO)?;
    let mut err_pos = print_file_from(stderr, err_start, libc::STDERR_FILENO)?;
    loop {
        let next_out = print_file_from(stdout, out_pos, libc::STDOUT_FILENO)?;
        let next_err = print_file_from(stderr, err_pos, libc::STDERR_FILENO)?;
        let changed = next_out != out_pos || next_err != err_pos;
        out_pos = next_out;
        err_pos = next_err;
        if !changed && !container_running(id) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn container_running(id: &str) -> bool {
    let Ok(state) = load_state(id) else {
        return false;
    };
    state
        .pid
        .is_some_and(|pid| state.status == "running" && crate::cli::is_process_alive(pid))
}

fn tail_start_offset(path: &PathBuf, lines: usize) -> io::Result<u64> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
    };
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    let tail = tail_lines(&data, lines);
    Ok((data.len() - tail.len()) as u64)
}

fn tail_lines(data: &[u8], lines: usize) -> &[u8] {
    if lines == 0 || data.is_empty() {
        return &data[data.len()..];
    }

    let mut seen = 0usize;
    let mut index = data.len();
    if data.last() == Some(&b'\n') {
        index = index.saturating_sub(1);
    }
    while index > 0 {
        index -= 1;
        if data[index] == b'\n' {
            seen += 1;
            if seen == lines {
                return &data[index + 1..];
            }
        }
    }
    data
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
        crate::cli::write_all_fd(fd, &buf[..n]);
        pos += n as u64;
    }
    Ok(pos)
}
