use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crate::program_io::{
    PROGRAM_STREAM_STDERR, PROGRAM_STREAM_STDOUT, ProgramClose, ProgramExit, ProgramIoAdapter,
    ProgramIoError, ProgramIoEvent, ProgramOpen, ProgramOutput, ProgramPoll, ProgramStdin,
};
use crate::protocol::Hash;

struct NativeProcessSession {
    child: Child,
    stdin: Option<ChildStdin>,
    events: Arc<Mutex<Vec<ProgramIoEvent>>>,
    stdout_thread: Option<JoinHandle<()>>,
    stderr_thread: Option<JoinHandle<()>>,
    exit_reported: bool,
}

#[derive(Default)]
pub struct NativeProcessAdapter {
    sessions: BTreeMap<Hash, NativeProcessSession>,
}

impl NativeProcessAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    fn drain_events(&mut self, session_id: Hash) -> Result<Vec<ProgramIoEvent>, ProgramIoError> {
        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or(ProgramIoError::SessionMissing)?;
        let mut events = session
            .events
            .lock()
            .map_err(|_| ProgramIoError::BackendFailed)?
            .drain(..)
            .collect::<Vec<_>>();
        if !session.exit_reported {
            if let Some(status) = session
                .child
                .try_wait()
                .map_err(|_| ProgramIoError::BackendFailed)?
            {
                session.exit_reported = true;
                events.push(ProgramIoEvent::Exit(ProgramExit {
                    session_id,
                    code: status.code().unwrap_or(-1),
                }));
            }
        }
        Ok(events)
    }

    fn join_finished_readers(session: &mut NativeProcessSession) {
        if session.exit_reported {
            if let Some(handle) = session.stdout_thread.take() {
                let _ = handle.join();
            }
            if let Some(handle) = session.stderr_thread.take() {
                let _ = handle.join();
            }
        }
    }
}

impl Drop for NativeProcessAdapter {
    fn drop(&mut self) {
        for (_, mut session) in core::mem::take(&mut self.sessions) {
            let _ = session.child.kill();
            let _ = session.child.wait();
            if let Some(handle) = session.stdout_thread.take() {
                let _ = handle.join();
            }
            if let Some(handle) = session.stderr_thread.take() {
                let _ = handle.join();
            }
        }
    }
}

impl ProgramIoAdapter for NativeProcessAdapter {
    fn open(&mut self, request: ProgramOpen) -> Result<Vec<ProgramIoEvent>, ProgramIoError> {
        if self.sessions.contains_key(&request.session_id) {
            return Err(ProgramIoError::SessionExists);
        }
        if request.program.is_empty() {
            return Err(ProgramIoError::ProgramRejected);
        }
        let mut command = Command::new(&request.program);
        command.args(&request.args);
        if !request.cwd.is_empty() {
            command.current_dir(&request.cwd);
        }
        for env in &request.env {
            command.env(&env.key, String::from_utf8_lossy(&env.value).as_ref());
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(|_| ProgramIoError::BackendFailed)?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let events = Arc::new(Mutex::new(Vec::new()));
        let stdout_thread = stdout.map(|stdout| {
            spawn_reader(
                request.session_id,
                PROGRAM_STREAM_STDOUT,
                stdout,
                Arc::clone(&events),
            )
        });
        let stderr_thread = stderr.map(|stderr| {
            spawn_reader(
                request.session_id,
                PROGRAM_STREAM_STDERR,
                stderr,
                Arc::clone(&events),
            )
        });
        self.sessions.insert(
            request.session_id,
            NativeProcessSession {
                child,
                stdin,
                events,
                stdout_thread,
                stderr_thread,
                exit_reported: false,
            },
        );
        Ok(Vec::new())
    }

    fn stdin(&mut self, input: ProgramStdin) -> Result<Vec<ProgramIoEvent>, ProgramIoError> {
        {
            let session = self
                .sessions
                .get_mut(&input.session_id)
                .ok_or(ProgramIoError::SessionMissing)?;
            if let Some(stdin) = &mut session.stdin {
                stdin
                    .write_all(&input.bytes)
                    .and_then(|_| stdin.flush())
                    .map_err(|_| ProgramIoError::BackendFailed)?;
            } else if !input.bytes.is_empty() {
                return Err(ProgramIoError::BackendFailed);
            }
            if input.eof {
                session.stdin.take();
            }
        }
        self.drain_events(input.session_id)
    }

    fn close(&mut self, close: ProgramClose) -> Result<Vec<ProgramIoEvent>, ProgramIoError> {
        let mut session = self
            .sessions
            .remove(&close.session_id)
            .ok_or(ProgramIoError::SessionMissing)?;
        if !session.exit_reported {
            let _ = session.child.kill();
        }
        let status = session
            .child
            .wait()
            .map_err(|_| ProgramIoError::BackendFailed)?;
        let mut events = session
            .events
            .lock()
            .map_err(|_| ProgramIoError::BackendFailed)?
            .drain(..)
            .collect::<Vec<_>>();
        if !session.exit_reported {
            events.push(ProgramIoEvent::Exit(ProgramExit {
                session_id: close.session_id,
                code: status.code().unwrap_or(-1),
            }));
        }
        if let Some(handle) = session.stdout_thread.take() {
            let _ = handle.join();
        }
        if let Some(handle) = session.stderr_thread.take() {
            let _ = handle.join();
        }
        Ok(events)
    }

    fn poll(&mut self, poll: ProgramPoll) -> Result<Vec<ProgramIoEvent>, ProgramIoError> {
        let events = self.drain_events(poll.session_id)?;
        if let Some(session) = self.sessions.get_mut(&poll.session_id) {
            Self::join_finished_readers(session);
        }
        Ok(events)
    }
}

fn spawn_reader<R: Read + Send + 'static>(
    session_id: Hash,
    stream: u16,
    mut reader: R,
    events: Arc<Mutex<Vec<ProgramIoEvent>>>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    push_event(
                        &events,
                        ProgramIoEvent::Output(ProgramOutput {
                            session_id,
                            stream,
                            bytes: Vec::new(),
                            eof: true,
                        }),
                    );
                    break;
                }
                Ok(n) => {
                    push_event(
                        &events,
                        ProgramIoEvent::Output(ProgramOutput {
                            session_id,
                            stream,
                            bytes: buf[..n].to_vec(),
                            eof: false,
                        }),
                    );
                }
                Err(_) => break,
            }
        }
    })
}

fn push_event(events: &Arc<Mutex<Vec<ProgramIoEvent>>>, event: ProgramIoEvent) {
    if let Ok(mut events) = events.lock() {
        events.push(event);
    }
}
