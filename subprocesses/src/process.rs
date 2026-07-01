use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputData {
    Utf8(String),
    Raw(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessEvent {
    Stdout(OutputData),
    Stderr(OutputData),
    ProcessExited(Option<u32>),
}

/// One of a process's standard streams, used to report which one could not be acquired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdStream {
    Stdin,
    Stdout,
    Stderr,
}

impl std::fmt::Display for StdStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            StdStream::Stdin => "stdin",
            StdStream::Stdout => "stdout",
            StdStream::Stderr => "stderr",
        };
        f.write_str(s)
    }
}

/// Failure to spawn a child process.
///
/// The PTY-backed variants carry a formatted message because `portable_pty` reports failures as
/// `anyhow::Error` (not a concrete `std::error::Error`); the pipe-backed variant keeps the real
/// `std::io::Error` source.
#[derive(Debug, thiserror::Error)]
pub enum SpawnError {
    #[error("Erreur à la création du PTY : {0}")]
    PtyCreation(String),
    #[error("Erreur à l'exécution du sous-processus : {0}")]
    PtySpawn(String),
    #[error("Erreur à l'acquisition du reader PTY : {0}")]
    PtyReader(String),
    #[error("Erreur à l'acquisition de l'entrée standard : {0}")]
    PtyWriter(String),
    #[error("Erreur à l'exécution du sous-processus : {0}")]
    PipeSpawn(#[source] std::io::Error),
    #[error("Impossible d'acquérir {0}")]
    StreamUnavailable(StdStream),
}

/// Outcome of sending data to a process's stdin.
#[derive(Debug, thiserror::Error)]
pub enum SendError {
    /// The process no longer accepts input: it has exited or been killed (its stdin slot
    /// is closed, or the write hit a broken pipe).
    #[error("Le sous-processus est terminé")]
    Finished,
    /// A genuine I/O error occurred while writing.
    #[error("Erreur d'écriture stdin : {0}")]
    Io(#[from] std::io::Error),
}

/// Failure to terminate a child process.
#[derive(Debug, thiserror::Error)]
pub enum KillError {
    #[error("Erreur lors du test d'état du processus : {0}")]
    Status(#[source] std::io::Error),
    #[error("Erreur à l'arrêt du processus : {0}")]
    Kill(#[source] std::io::Error),
    #[error("Erreur à l'attente du processus : {0}")]
    Wait(#[source] std::io::Error),
}

enum ChildHandle {
    Pty {
        child: Box<dyn portable_pty::Child + Send + Sync>,
        _master: Box<dyn portable_pty::MasterPty + Send>,
    },
    Pipe {
        child: Child,
    },
}

impl ChildHandle {
    /// Returns Ok(None) if still running, Ok(Some(code)) if exited.
    /// code is None if the exit code couldn't be determined, Some(n) otherwise.
    fn try_wait(&mut self) -> std::io::Result<Option<Option<u32>>> {
        match self {
            ChildHandle::Pty { child, .. } => match child.try_wait() {
                Ok(Some(status)) => Ok(Some(Some(status.exit_code()))),
                Ok(None) => Ok(None),
                Err(e) => Err(std::io::Error::other(e.to_string())),
            },
            ChildHandle::Pipe { child } => match child.try_wait() {
                Ok(Some(status)) => Ok(Some(status.code().map(|c| c as u32))),
                Ok(None) => Ok(None),
                Err(e) => Err(e),
            },
        }
    }

    fn kill(&mut self) -> std::io::Result<()> {
        match self {
            ChildHandle::Pty { child, .. } => child
                .kill()
                .map_err(|e| std::io::Error::other(e.to_string())),
            ChildHandle::Pipe { child } => child.kill(),
        }
    }

    /// Returns the exit code, or None if it couldn't be determined.
    fn wait(&mut self) -> std::io::Result<Option<u32>> {
        match self {
            ChildHandle::Pty { child, .. } => match child.wait() {
                Ok(status) => Ok(Some(status.exit_code())),
                Err(e) => Err(std::io::Error::other(e.to_string())),
            },
            ChildHandle::Pipe { child } => {
                let status = child.wait()?;
                Ok(status.code().map(|c| c as u32))
            }
        }
    }
}

pub type StdinWriter = Arc<Mutex<Option<Box<dyn Write + Send>>>>;

/// Owned RAII handle to a child process.
///
/// Dropping a `Process` kills the child if it is still running (see [`Process::kill`]),
/// frees its PTY master / child handle, and lets the detached reader thread(s) finish.
/// All teardown is idempotent: explicit `kill`, `Drop`, and a natural exit observed by the
/// reader thread all converge to the same terminal state without double-killing.
pub struct Process {
    child: Arc<Mutex<ChildHandle>>,
    stdin: StdinWriter,
    /// Set once the child has been killed or observed to exit. Makes `kill`/`Drop`
    /// idempotent and turns a post-exit `kill` into a no-op.
    terminated: Arc<AtomicBool>,
    _reader_handles: Vec<JoinHandle<()>>,
}

impl Process {
    pub fn spawn_pty<F>(command: &str, args: &[&str], callback: F) -> Result<Self, SpawnError>
    where
        F: Fn(ProcessEvent) + Send + 'static,
    {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows: 36,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| SpawnError::PtyCreation(e.to_string()))?;

        #[cfg(unix)]
        {
            let fd = pair
                .master
                .as_raw_fd()
                .expect("Should have a raw fd on UNIX platform");
            unsafe {
                let mut termios: libc::termios = std::mem::zeroed();
                if libc::tcgetattr(fd, &mut termios) == 0 {
                    libc::cfmakeraw(&mut termios);
                    libc::tcsetattr(fd, libc::TCSANOW, &termios);
                }
            }
        }

        let mut cmd = CommandBuilder::new(command);
        for arg in args {
            cmd.arg(*arg);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| SpawnError::PtySpawn(e.to_string()))?;

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| SpawnError::PtyReader(e.to_string()))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| SpawnError::PtyWriter(e.to_string()))?;

        drop(pair.slave);

        let child_handle = ChildHandle::Pty {
            child,
            _master: pair.master,
        };
        let child_arc = Arc::new(Mutex::new(child_handle));
        let stdin: StdinWriter = Arc::new(Mutex::new(Some(writer)));
        let terminated = Arc::new(AtomicBool::new(false));

        let exit_emitted = Arc::new(AtomicBool::new(false));
        let reader_handle = Self::spawn_reader_thread(
            reader,
            Arc::clone(&child_arc),
            exit_emitted,
            Arc::clone(&terminated),
            Arc::clone(&stdin),
            callback,
            false,
        );

        Ok(Process {
            child: child_arc,
            stdin,
            terminated,
            _reader_handles: vec![reader_handle],
        })
    }

    pub fn spawn_pipes<F>(command: &str, args: &[&str], callback: F) -> Result<Self, SpawnError>
    where
        F: Fn(ProcessEvent) + Send + Clone + 'static,
    {
        let mut child = Command::new(command)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()
            .map_err(SpawnError::PipeSpawn)?;

        let stdout = child
            .stdout
            .take()
            .ok_or(SpawnError::StreamUnavailable(StdStream::Stdout))?;
        let stderr = child
            .stderr
            .take()
            .ok_or(SpawnError::StreamUnavailable(StdStream::Stderr))?;
        let stdin_pipe = child
            .stdin
            .take()
            .ok_or(SpawnError::StreamUnavailable(StdStream::Stdin))?;

        let child_handle = ChildHandle::Pipe { child };
        let child_arc = Arc::new(Mutex::new(child_handle));
        let stdin: StdinWriter = Arc::new(Mutex::new(Some(Box::new(stdin_pipe))));
        let terminated = Arc::new(AtomicBool::new(false));

        let exit_emitted = Arc::new(AtomicBool::new(false));
        let stdout_callback = callback.clone();
        let stdout_handle = Self::spawn_reader_thread(
            stdout,
            Arc::clone(&child_arc),
            Arc::clone(&exit_emitted),
            Arc::clone(&terminated),
            Arc::clone(&stdin),
            stdout_callback,
            false,
        );

        let stderr_handle = Self::spawn_reader_thread(
            stderr,
            Arc::clone(&child_arc),
            exit_emitted,
            Arc::clone(&terminated),
            Arc::clone(&stdin),
            callback,
            true,
        );

        Ok(Process {
            child: child_arc,
            stdin,
            terminated,
            _reader_handles: vec![stdout_handle, stderr_handle],
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_reader_thread<R, F>(
        reader: R,
        child: Arc<Mutex<ChildHandle>>,
        exit_emitted: Arc<AtomicBool>,
        terminated: Arc<AtomicBool>,
        stdin: StdinWriter,
        callback: F,
        is_stderr: bool,
    ) -> JoinHandle<()>
    where
        R: std::io::Read + Send + 'static,
        F: Fn(ProcessEvent) + Send + 'static,
    {
        std::thread::spawn(move || {
            let mut buf_reader = BufReader::new(reader);
            // Run exactly once, when the child is first observed to have exited: mark the
            // process terminated, close its stdin (so further sends report `Finished` and
            // `kill`/`Drop` become a no-op), then emit the exit event.
            let on_exit = |code| {
                if !exit_emitted.swap(true, Ordering::AcqRel) {
                    terminated.store(true, Ordering::Release);
                    *stdin.lock().unwrap() = None;
                    callback(ProcessEvent::ProcessExited(code));
                }
            };
            loop {
                let mut buf = Vec::new();
                match buf_reader.read_until(b'\n', &mut buf) {
                    Ok(0) => {
                        let exit_code = match child.lock().unwrap().wait() {
                            Ok(code) => code,
                            Err(_) => None,
                        };
                        on_exit(exit_code);
                        break;
                    }
                    Ok(_) => {
                        let data = match String::from_utf8(buf) {
                            Ok(s) => OutputData::Utf8(s),
                            Err(e) => OutputData::Raw(e.into_bytes()),
                        };
                        let event = if is_stderr {
                            ProcessEvent::Stderr(data)
                        } else {
                            ProcessEvent::Stdout(data)
                        };
                        callback(event);
                    }
                    Err(_) => match child.lock().unwrap().try_wait() {
                        Ok(Some(code)) => {
                            on_exit(code);
                            break;
                        }
                        _ => {
                            continue;
                        }
                    },
                }
            }
        })
    }

    pub fn get_stdin_writer(&self) -> StdinWriter {
        self.stdin.clone()
    }

    pub fn send_stdin(&self, data: &[u8]) -> Result<(), SendError> {
        let mut guard = self.stdin.lock().unwrap();
        let Some(writer) = guard.as_mut() else {
            return Err(SendError::Finished);
        };
        match writer.write_all(data).and_then(|()| writer.flush()) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::BrokenPipe => Err(SendError::Finished),
            Err(e) => Err(SendError::Io(e)),
        }
    }

    /// Terminate the child if it is still running. Idempotent and safe to call on an
    /// already-killed or already-exited process: the first caller (explicit `kill`, `Drop`,
    /// or the reader thread observing exit) wins, the rest are no-ops.
    pub fn kill(&self) -> Result<(), KillError> {
        if self.terminated.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        // Close stdin first so the child sees EOF.
        {
            let mut guard = self.stdin.lock().unwrap();
            *guard = None;
        }
        let mut child = self.child.lock().unwrap();
        match child.try_wait() {
            Ok(Some(_)) => Ok(()),
            Ok(None) => {
                child.kill().map_err(KillError::Kill)?;
                child.wait().map_err(KillError::Wait)?;
                Ok(())
            }
            Err(e) => Err(KillError::Status(e)),
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        // Idempotent: a no-op if the child already exited or was killed.
        let _ = self.kill();
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    /// Block until the spawned child is observed to have exited (the reader thread emits
    /// `ProcessExited` only after it has nulled stdin and flipped `terminated`).
    fn spawn_and_wait_for_exit(command: &str, args: &[&str]) -> Process {
        let (tx, rx) = mpsc::channel();
        let process = Process::spawn_pipes(command, args, move |event| {
            if let ProcessEvent::ProcessExited(_) = event {
                let _ = tx.send(());
            }
        })
        .expect("spawn");
        rx.recv_timeout(Duration::from_secs(5))
            .expect("child should exit promptly");
        process
    }

    #[test]
    fn kill_is_idempotent_on_a_running_child() {
        let process = Process::spawn_pipes("sleep", &["10"], |_| {}).expect("spawn");
        assert!(process.kill().is_ok());
        // A second explicit kill on an already-killed process is a clean no-op.
        assert!(process.kill().is_ok());
        // Dropping afterwards must not double-kill / panic either.
    }

    #[test]
    fn kill_after_natural_exit_is_a_noop() {
        let process = spawn_and_wait_for_exit("true", &[]);
        // The child already exited; kill sees `terminated`/`try_wait` and does nothing.
        assert!(process.kill().is_ok());
    }

    #[test]
    fn send_after_exit_reports_finished() {
        let process = spawn_and_wait_for_exit("true", &[]);
        // The reader thread nulled stdin when the child exited, so the send is rejected
        // deterministically rather than racing a broken pipe.
        match process.send_stdin(b"hello\n") {
            Err(SendError::Finished) => {}
            other => panic!("expected SendError::Finished, got {:?}", other),
        }
    }

    #[test]
    fn send_after_kill_reports_finished() {
        let process = Process::spawn_pipes("sleep", &["10"], |_| {}).expect("spawn");
        assert!(process.kill().is_ok());
        match process.send_stdin(b"hello\n") {
            Err(SendError::Finished) => {}
            other => panic!("expected SendError::Finished, got {:?}", other),
        }
    }
}
