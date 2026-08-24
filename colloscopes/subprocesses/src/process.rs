use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

// Killing a parent must tear down its whole subprocess subtree. On unix this happens for free:
// each child is spawned with a controlling tty, so a parent's death closes the pty master and
// hangs up the child (SIGHUP → terminate). On windows a kill-on-close Job Object provides the
// equivalent guarantee (see `job` below). No other platform has a mechanism here, so refuse to
// build rather than silently orphan solver processes.
#[cfg(not(any(unix, windows)))]
compile_error!(
    "collomatique-subprocesses teardown requires a controlling-tty (unix) or job-object (windows) backend"
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputData {
    Utf8(String),
    Raw(Vec<u8>),
}

impl OutputData {
    /// The data as text, with any byte that is not valid UTF-8 replaced by U+FFFD.
    ///
    /// This is program output on its way to a log view, so there is nothing to be
    /// gained by refusing it: a Windows console under a legacy code page, or a C++
    /// library writing a raw byte, should still be readable.
    pub fn into_lossy_string(self) -> String {
        match self {
            OutputData::Utf8(s) => s,
            OutputData::Raw(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        }
    }
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
    #[cfg(windows)]
    #[error("Impossible d'obtenir le PID du sous-processus pour le Job Object")]
    NoPid,
    #[cfg(windows)]
    #[error("Erreur à la création du Job Object : {0}")]
    JobObject(#[source] std::io::Error),
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
    /// Windows kill-on-close Job Object owning the child (and its descendants). Held for its
    /// whole lifetime: when this process dies for any reason the OS closes the handle and kills
    /// everything in the job, which is what tears the subtree down. Killed explicitly in `kill`.
    #[cfg(windows)]
    _job: job::Job,
}

impl Process {
    pub fn spawn_pty<F>(
        command: &std::ffi::OsStr,
        args: &[&str],
        envs: &[(&str, &std::ffi::OsStr)],
        callback: F,
    ) -> Result<Self, SpawnError>
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
        // Added, not substituted: nothing here clears the environment, so the child
        // still inherits everything this process has — PATH, and on Windows the GTK
        // and Python prefixes it is started from.
        for (key, value) in envs {
            cmd.env(key, value);
        }
        // portable_pty otherwise starts the child in `$HOME` (a default meant for terminal
        // emulators opening a shell). The engine should inherit our working directory like
        // any other subprocess, as `spawn_pipes` already does. If our own cwd is gone,
        // `current_dir` fails and the library default is better than refusing to spawn.
        if let Ok(cwd) = std::env::current_dir() {
            cmd.cwd(cwd);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| SpawnError::PtySpawn(e.to_string()))?;

        // Bind the child to a kill-on-close job so its subtree dies when this process does.
        // Safe against the spawn/assign race here: the child (`--rpc-engine`) spawns nothing
        // until it has received its init message on the RPC channel, long after assignment.
        #[cfg(windows)]
        let job = {
            let pid = child.process_id().ok_or(SpawnError::NoPid)?;
            job::Job::kill_on_close_with(pid).map_err(SpawnError::JobObject)?
        };

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
            #[cfg(windows)]
            _job: job,
        })
    }

    pub fn spawn_pipes<F>(
        command: &std::ffi::OsStr,
        args: &[&str],
        envs: &[(&str, &std::ffi::OsStr)],
        callback: F,
    ) -> Result<Self, SpawnError>
    where
        F: Fn(ProcessEvent) + Send + Clone + 'static,
    {
        let mut command = Command::new(command);
        command
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped());
        // Added, not substituted: `Command` inherits our whole environment unless
        // asked to clear it, and nothing here asks.
        for (key, value) in envs {
            command.env(key, value);
        }

        // Start console programs without a console.
        //
        // A windows process that has no console of its own -- which the graphical
        // interface is, being built with `windows_subsystem = "windows"` -- makes
        // windows allocate a fresh console for any console-subsystem child it
        // starts. That console is a command window, popping up in front of the
        // application for as long as the child runs. CREATE_NO_WINDOW says to
        // create none: the child simply has no console.
        //
        // Nothing is lost by it. Every stream we care about is a pipe we handed
        // the child ourselves, and the flag only ever applies to a console
        // program -- an `--rpc-engine` child, being this same windows-subsystem
        // binary, had no console either way.
        //
        // `creation_flags` replaces the flag word rather than adding to it, but
        // the standard library ORs its own CREATE_UNICODE_ENVIRONMENT in
        // afterwards, so nothing it needs is lost.
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;

            const CREATE_NO_WINDOW: u32 = 0x0800_0000;

            command.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = command.spawn().map_err(SpawnError::PipeSpawn)?;

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

        // Bind the child to a kill-on-close job so its subtree dies when this process does.
        // This is the windows teardown mechanism, and `Worker` spawns through here on that
        // platform. Same race note as in `spawn_pty`: an `--rpc-engine` child spawns nothing
        // of its own until it has received its init message, long after assignment.
        #[cfg(windows)]
        let job = job::Job::kill_on_close_with(child.id()).map_err(SpawnError::JobObject)?;

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
            #[cfg(windows)]
            _job: job,
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
        // On windows, terminate the whole job (the child and any descendants) so an explicit
        // cancel reaps grandchildren too, matching the unix SIGHUP cascade. The reap below then
        // observes the now-dead child.
        #[cfg(windows)]
        self._job.terminate();
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

/// Windows kill-on-close Job Object backend for subtree teardown.
///
/// A job created with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` kills every process it contains the
/// moment the last handle to it is closed. Since the OS closes all of a process's handles when it
/// dies — for any reason, including a hard kill or a crash — holding the job handle in the parent
/// means the child and every descendant that inherited the job are killed when the parent dies.
/// This is the windows equivalent of the unix pty-hangup/SIGHUP cascade.
#[cfg(windows)]
mod job {
    use std::io;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
    };

    /// Owns a kill-on-close Job Object handle. Dropping it (or the owning process dying) closes
    /// the handle and terminates every process still in the job.
    pub struct Job {
        handle: HANDLE,
    }

    // A job object is a plain kernel handle with no thread affinity.
    unsafe impl Send for Job {}
    unsafe impl Sync for Job {}

    impl Job {
        /// Create a kill-on-close job and assign the process `pid` to it.
        pub fn kill_on_close_with(pid: u32) -> io::Result<Job> {
            unsafe {
                let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if handle.is_null() {
                    return Err(io::Error::last_os_error());
                }
                // `job` now owns `handle`; any early return closes it via `Drop`.
                let job = Job { handle };

                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                if SetInformationJobObject(
                    handle,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const core::ffi::c_void,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                ) == 0
                {
                    return Err(io::Error::last_os_error());
                }

                let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid);
                if process.is_null() {
                    return Err(io::Error::last_os_error());
                }
                let assigned = AssignProcessToJobObject(handle, process);
                CloseHandle(process);
                if assigned == 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(job)
            }
        }

        /// Immediately terminate every process in the job (the whole subtree).
        pub fn terminate(&self) {
            unsafe {
                TerminateJobObject(self.handle, 1);
            }
        }
    }

    impl Drop for Job {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use std::sync::mpsc;
    use std::time::Duration;

    /// Block until the spawned child is observed to have exited (the reader thread emits
    /// `ProcessExited` only after it has nulled stdin and flipped `terminated`).
    fn spawn_and_wait_for_exit(command: &str, args: &[&str]) -> Process {
        let (tx, rx) = mpsc::channel();
        let process = Process::spawn_pipes(OsStr::new(command), args, &[], move |event| {
            if let ProcessEvent::ProcessExited(_) = event {
                let _ = tx.send(());
            }
        })
        .expect("spawn");
        rx.recv_timeout(Duration::from_secs(5))
            .expect("child should exit promptly");
        process
    }

    /// The load-bearing unix teardown mechanism: a child spawned with a controlling tty dies
    /// when the pty master is closed, because the kernel hangs up its controlling terminal and
    /// delivers SIGHUP (default disposition: terminate). In production the master is closed by
    /// the OS when a parent process dies (for any reason, including a hard kill), which is what
    /// cascades the teardown down the subprocess tree. This asserts the primitive holds on the
    /// current platform (Linux and macOS).
    #[test]
    fn closing_the_pty_master_hangs_up_the_child() {
        use portable_pty::{PtySize, native_pty_system};

        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("openpty");

        let mut cmd = CommandBuilder::new("sleep");
        cmd.arg("300");
        // Without an explicit cwd, portable_pty starts the child in `$HOME`, and the
        // spawn fails with ENOENT if that directory does not exist (the nix build
        // sandbox sets `HOME=/homeless-shelter`). Any existing directory will do here.
        cmd.cwd(std::env::current_dir().expect("current dir"));
        let mut child = pair.slave.spawn_command(cmd).expect("spawn");
        drop(pair.slave);

        // Close the master: this hangs up the child's controlling terminal.
        drop(pair.master);

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break, // terminated by SIGHUP, as expected
                Ok(None) => {
                    if std::time::Instant::now() > deadline {
                        let _ = child.kill();
                        panic!("child survived pty master close — no SIGHUP hangup");
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(e) => panic!("try_wait failed: {e}"),
            }
        }
    }

    #[test]
    fn kill_is_idempotent_on_a_running_child() {
        let process =
            Process::spawn_pipes(OsStr::new("sleep"), &["10"], &[], |_| {}).expect("spawn");
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
        let process =
            Process::spawn_pipes(OsStr::new("sleep"), &["10"], &[], |_| {}).expect("spawn");
        assert!(process.kill().is_ok());
        match process.send_stdin(b"hello\n") {
            Err(SendError::Finished) => {}
            other => panic!("expected SendError::Finished, got {:?}", other),
        }
    }
}
