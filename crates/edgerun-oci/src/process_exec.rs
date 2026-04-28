//! Shared child-process environment and exec helpers.

use crate::prelude::*;
use core::fmt;
use std::ffi::CString;
use std::io;

const DEFAULT_PATH: &str = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";

pub(crate) enum ProcessExecError {
    Setup(io::Error),
    Exec(io::Error),
}

impl ProcessExecError {
    pub(crate) fn exit_code(&self) -> i32 {
        match self {
            Self::Setup(_) => 126,
            Self::Exec(_) => 127,
        }
    }
}

impl fmt::Display for ProcessExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Setup(error) | Self::Exec(error) => error.fmt(f),
        }
    }
}

impl fmt::Debug for ProcessExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Setup(error) => f.debug_tuple("Setup").field(error).finish(),
            Self::Exec(error) => f.debug_tuple("Exec").field(error).finish(),
        }
    }
}

pub(crate) fn exec_with_env_and_cwd(
    args: &[String],
    env: &[String],
    cwd: &str,
) -> Result<(), ProcessExecError> {
    if args.is_empty() {
        return Err(ProcessExecError::Setup(io::Error::new(
            io::ErrorKind::InvalidInput,
            "command to execute required",
        )));
    }

    apply_process_env(env).map_err(ProcessExecError::Setup)?;
    chdir(cwd).map_err(ProcessExecError::Setup)?;

    let exe_path = resolve_executable_path(&args[0], env);
    let exe_cstr = c_string(exe_path.as_bytes()).map_err(ProcessExecError::Setup)?;
    let c_args: Vec<CString> = args
        .iter()
        .map(|arg| c_string(arg.as_bytes()))
        .collect::<io::Result<_>>()
        .map_err(ProcessExecError::Setup)?;
    let c_ptrs: Vec<*const libc::c_char> = c_args
        .iter()
        .map(|arg| arg.as_ptr())
        .chain(std::iter::once(std::ptr::null()))
        .collect();

    unsafe { libc::execvp(exe_cstr.as_ptr(), c_ptrs.as_ptr()) };
    Err(ProcessExecError::Exec(io::Error::last_os_error()))
}

fn apply_process_env(env: &[String]) -> io::Result<()> {
    unsafe { libc::clearenv() };
    for entry in env {
        if let Some((key, value)) = entry.split_once('=') {
            let key = c_string(key.as_bytes())?;
            let value = c_string(value.as_bytes())?;
            unsafe { libc::setenv(key.as_ptr(), value.as_ptr(), 1) };
        }
    }
    Ok(())
}

fn chdir(cwd: &str) -> io::Result<()> {
    let cwd = c_string(cwd.as_bytes())?;
    let result = unsafe { libc::chdir(cwd.as_ptr()) };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn resolve_executable_path(command: &str, env: &[String]) -> String {
    if command.starts_with('/') {
        return command.to_string();
    }

    let path_env = env
        .iter()
        .find_map(|entry| entry.strip_prefix("PATH="))
        .unwrap_or(DEFAULT_PATH);
    for dir in path_env.split(':') {
        let candidate = format!("{dir}/{command}");
        if std::path::Path::new(&candidate).exists() {
            return candidate;
        }
    }
    command.to_string()
}

fn c_string(bytes: &[u8]) -> io::Result<CString> {
    CString::new(bytes).map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_absolute_command_as_is() {
        assert_eq!(
            resolve_executable_path("/bin/sh", &["PATH=/nope".into()]),
            "/bin/sh"
        );
    }

    #[test]
    fn falls_back_to_command_name_when_path_misses() {
        assert_eq!(
            resolve_executable_path("definitely-not-edgerun-test-bin", &["PATH=/nope".into()]),
            "definitely-not-edgerun-test-bin"
        );
    }
}
