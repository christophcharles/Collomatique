use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProcessId(pub(crate) u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessStatus {
    Running,
    Exited(Option<u32>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputData {
    Utf8(String),
    Raw(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputEntry {
    Stdout(OutputData),
    Stderr(OutputData),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessEvent {
    Stdout(OutputData),
    Stderr(OutputData),
    ProcessExited(Option<u32>),
    Error(String),
}

pub struct ProcessState {
    pub status: ProcessStatus,
    pub output_log: Vec<OutputEntry>,
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

pub struct Process {
    state: ProcessState,
    child: Arc<Mutex<ChildHandle>>,
    stdin: StdinWriter,
    _reader_handles: Vec<JoinHandle<()>>,
}

impl Process {
    pub(crate) fn spawn_pty<F>(command: &str, args: &[&str], callback: F) -> Result<Self, String>
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
            .map_err(|e| format!("Erreur à la création du PTY : {}", e))?;

        #[cfg(unix)]
        {
            let fd = pair
                .master
                .as_raw_fd()
                .expect("Should have a raw fd on UNIX platform");
            unsafe {
                let mut termios: libc::termios = std::mem::zeroed();
                libc::tcgetattr(fd, &mut termios);
                termios.c_lflag &= !libc::ECHO;
                termios.c_lflag &= !libc::ECHONL;
                libc::tcsetattr(fd, libc::TCSANOW, &termios);
            }
        }

        let mut cmd = CommandBuilder::new(command);
        for arg in args {
            cmd.arg(*arg);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Erreur à l'exécution du sous-processus : {}", e))?;

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("Erreur à l'acquisition du reader PTY : {}", e))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("Erreur à l'acquisition de l'entrée standard : {}", e))?;

        drop(pair.slave);

        let child_handle = ChildHandle::Pty {
            child,
            _master: pair.master,
        };
        let child_arc = Arc::new(Mutex::new(child_handle));
        let stdin: StdinWriter = Arc::new(Mutex::new(Some(writer)));

        let exit_emitted = Arc::new(AtomicBool::new(false));
        let reader_handle = Self::spawn_reader_thread(
            reader,
            Arc::clone(&child_arc),
            exit_emitted,
            callback,
            false,
        );

        Ok(Process {
            state: ProcessState {
                status: ProcessStatus::Running,
                output_log: Vec::new(),
            },
            child: child_arc,
            stdin,
            _reader_handles: vec![reader_handle],
        })
    }

    pub(crate) fn spawn_pipes<F>(command: &str, args: &[&str], callback: F) -> Result<Self, String>
    where
        F: Fn(ProcessEvent) + Send + Clone + 'static,
    {
        let mut child = Command::new(command)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Erreur à l'exécution du sous-processus : {}", e))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Impossible d'acquérir stdout".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "Impossible d'acquérir stderr".to_string())?;
        let stdin_pipe = child
            .stdin
            .take()
            .ok_or_else(|| "Impossible d'acquérir stdin".to_string())?;

        let child_handle = ChildHandle::Pipe { child };
        let child_arc = Arc::new(Mutex::new(child_handle));
        let stdin: StdinWriter = Arc::new(Mutex::new(Some(Box::new(stdin_pipe))));

        let exit_emitted = Arc::new(AtomicBool::new(false));
        let stdout_callback = callback.clone();
        let stdout_handle = Self::spawn_reader_thread(
            stdout,
            Arc::clone(&child_arc),
            Arc::clone(&exit_emitted),
            stdout_callback,
            false,
        );

        let stderr_handle =
            Self::spawn_reader_thread(stderr, Arc::clone(&child_arc), exit_emitted, callback, true);

        Ok(Process {
            state: ProcessState {
                status: ProcessStatus::Running,
                output_log: Vec::new(),
            },
            child: child_arc,
            stdin,
            _reader_handles: vec![stdout_handle, stderr_handle],
        })
    }

    fn spawn_reader_thread<R, F>(
        reader: R,
        child: Arc<Mutex<ChildHandle>>,
        exit_emitted: Arc<AtomicBool>,
        callback: F,
        is_stderr: bool,
    ) -> JoinHandle<()>
    where
        R: std::io::Read + Send + 'static,
        F: Fn(ProcessEvent) + Send + 'static,
    {
        std::thread::spawn(move || {
            let mut buf_reader = BufReader::new(reader);
            let emit_exit = |code| {
                if !exit_emitted.swap(true, Ordering::AcqRel) {
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
                        emit_exit(exit_code);
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
                            emit_exit(code);
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

    pub fn send_stdin(&self, data: &[u8]) -> Result<(), String> {
        let mut guard = self.stdin.lock().unwrap();
        let Some(writer) = guard.as_mut() else {
            return Err("Le processus n'accepte plus d'entrées".to_string());
        };
        match writer.write_all(data) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => return Ok(()),
            Err(e) => return Err(format!("Erreur d'écriture stdin : {}", e)),
        }
        match writer.flush() {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
            Err(e) => Err(format!("Erreur de flush stdin : {}", e)),
        }
    }

    pub fn kill(&self) -> Result<(), String> {
        // Close stdin first
        {
            let mut guard = self.stdin.lock().unwrap();
            *guard = None;
        }
        let mut child = self.child.lock().unwrap();
        child
            .kill()
            .map_err(|e| format!("Erreur à l'arrêt du processus : {}", e))?;
        child
            .wait()
            .map_err(|e| format!("Erreur à l'attente du processus : {}", e))?;
        Ok(())
    }

    pub fn state(&self) -> &ProcessState {
        &self.state
    }

    pub fn handle_event(&mut self, event: &ProcessEvent) {
        match event {
            ProcessEvent::Stdout(data) => {
                self.state
                    .output_log
                    .push(OutputEntry::Stdout(data.clone()));
            }
            ProcessEvent::Stderr(data) => {
                self.state
                    .output_log
                    .push(OutputEntry::Stderr(data.clone()));
            }
            ProcessEvent::ProcessExited(code) => {
                self.state.status = ProcessStatus::Exited(*code);
            }
            ProcessEvent::Error(_) => {}
        }
    }
}
