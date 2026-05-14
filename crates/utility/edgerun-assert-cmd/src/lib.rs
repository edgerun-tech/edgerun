use std::ffi::OsStr;
use std::path::Path;
use std::process::Output;
use std::process::Stdio;

pub struct Command {
    inner: std::process::Command,
    stdin: Option<Vec<u8>>,
}

impl Command {
    pub fn new<S: AsRef<OsStr>>(program: S) -> Self {
        Self {
            inner: std::process::Command::new(program),
            stdin: None,
        }
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.inner.arg(arg);
        self
    }

    pub fn current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self {
        self.inner.current_dir(dir);
        self
    }

    pub fn write_stdin<S: Into<Vec<u8>>>(&mut self, input: S) -> &mut Self {
        self.stdin = Some(input.into());
        self
    }

    pub fn assert(&mut self) -> assert::Assert {
        if self.stdin.is_some() {
            self.inner.stdin(Stdio::piped());
        }
        self.inner.stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = self.inner.spawn().expect("spawn command");
        if let Some(input) = self.stdin.take() {
            use std::io::Write;
            child
                .stdin
                .as_mut()
                .expect("command stdin")
                .write_all(&input)
                .expect("write command stdin");
        }
        assert::Assert {
            output: child.wait_with_output().expect("wait for command"),
        }
    }
}

pub mod assert {
    use super::Output;

    pub struct Assert {
        pub(crate) output: Output,
    }

    impl Assert {
        pub fn success(self) -> Self {
            assert!(
                self.output.status.success(),
                "expected command to succeed, status: {:?}, stderr: {}",
                self.output.status.code(),
                String::from_utf8_lossy(&self.output.stderr)
            );
            self
        }

        pub fn failure(self) -> Self {
            assert!(
                !self.output.status.success(),
                "expected command to fail, status: {:?}, stdout: {}",
                self.output.status.code(),
                String::from_utf8_lossy(&self.output.stdout)
            );
            self
        }

        pub fn stdout<S: AsRef<str>>(self, expected: S) -> Self {
            assert_eq!(
                String::from_utf8_lossy(&self.output.stdout),
                expected.as_ref()
            );
            self
        }

        pub fn stderr<S: AsRef<str>>(self, expected: S) -> Self {
            assert_eq!(
                String::from_utf8_lossy(&self.output.stderr),
                expected.as_ref()
            );
            self
        }
    }
}
