use std::ffi::OsStr;
use std::io::ErrorKind;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use collomatique_rpc::channel::{
    CHANNEL_ENV_VAR, ChannelError, Endpoint, FrameWriter, Listener as ChannelListener,
};
use collomatique_rpc::{AppProtocol, CmdMsg, CompleteCmdMsg, InitMsg, ResultMsg, RpcDecodeError};

use crate::process::{KillError, Process, ProcessEvent, SendError, SpawnError};

/// How long to wait for a freshly spawned worker to join its channel. Generous:
/// a cold start on Windows loads a large binary and its whole GTK dependency
/// tree before `main` runs.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_POLL_INTERVAL: Duration = Duration::from_millis(5);

/// The worker's end of the RPC channel, once it has connected.
///
/// `None` before the connection lands and again after it closes, so a send made
/// outside that window reports [`SendError::Finished`] rather than failing oddly.
pub type RpcWriter = Arc<Mutex<Option<FrameWriter>>>;

/// A runtime error surfaced by a [`Worker`]'s channel thread (as opposed to a spawn-time failure).
///
/// Both carry their cause as text: the underlying errors are `std::io::Error`,
/// which a [`WorkerEvent`] cannot hold — events are cloned and compared.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum WorkerError {
    #[error("Le sous-processus n'a pas rejoint le canal RPC : {0}")]
    NoConnection(String),
    #[error("Erreur sur le canal RPC : {0}")]
    Channel(String),
}

/// Which executable to re-execute as `<exe> --rpc-engine`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum EngineExe {
    /// The running executable. Correct whenever the host *is* a collomatique binary:
    /// the GUI, and an engine process spawning nested workers.
    #[default]
    Current,
    /// An explicit path, for a host that is not collomatique — a standalone Python
    /// interpreter importing the module.
    Explicit(PathBuf),
}

impl EngineExe {
    fn resolve(&self) -> Result<PathBuf, WorkerSpawnError> {
        match self {
            EngineExe::Current => std::env::current_exe().map_err(WorkerSpawnError::CurrentExe),
            EngineExe::Explicit(path) => Ok(path.clone()),
        }
    }
}

/// Failure to spawn a [`Worker`] subprocess.
#[derive(Debug, thiserror::Error)]
pub enum WorkerSpawnError {
    #[error("Impossible de déterminer l'exécutable courant : {0}")]
    CurrentExe(#[source] std::io::Error),
    #[error(transparent)]
    Spawn(#[from] SpawnError),
    #[error("Impossible de créer le canal RPC : {0}")]
    Channel(#[from] ChannelError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerEvent<P: AppProtocol> {
    LogLine(String),
    RpcCommand(Result<CmdMsg<P>, RpcDecodeError>),
    GracefulExit,
    ProcessExited(Option<u32>),
    Error(WorkerError),
}

/// Owned RAII handle to an RPC worker subprocess (`<engine> --rpc-engine`, see [`EngineExe`]).
///
/// A `Worker` owns its backing [`Process`], so dropping it tears the subprocess down
/// (killing it if still running).
///
/// The worker talks on two channels at once and this type merges them into one
/// stream of [`WorkerEvent`]s. Its standard streams carry whatever the program
/// prints — the engine's own logs, a Python script's `print()`, CBC's C++ output
/// — and each line of it becomes a [`WorkerEvent::LogLine`]. Its private RPC
/// channel carries the protocol, and nothing else: the init message out, then
/// commands in and answers out.
///
/// Two streams means two threads reading, so a `LogLine` and an `RpcCommand`
/// have no fixed order relative to each other. Nothing needs one — the protocol
/// is strict request/response, so any command the worker expects an answer to is
/// delivered before it can do anything else.
///
/// `P` is the application half of the protocol this channel speaks; a channel
/// that only runs an ILP or a strategy uses [`collomatique_rpc::NoApp`]. It is
/// fixed at spawn time by the init message, so nothing but the marker itself
/// needs to be stored.
pub struct Worker<P: AppProtocol> {
    process: Process,
    rpc_writer: RpcWriter,
    protocol: PhantomData<P>,
}

// `'static` on the parameter itself, not just on its message types: the init
// message is moved into the channel thread, and `InitMsg<P>` mentions `P`.
impl<P: AppProtocol + 'static> Worker<P> {
    pub fn spawn<F>(
        engine: &EngineExe,
        init_msg: InitMsg<P>,
        callback: F,
    ) -> Result<Worker<P>, WorkerSpawnError>
    where
        F: Fn(WorkerEvent<P>) + Send + 'static,
    {
        let exe = engine.resolve()?;

        // Bound before the spawn, so the name in the environment is already
        // listening by the time the child looks it up.
        let listener = ChannelListener::bind()?;
        let channel_name = listener.channel_name();

        // Both threads report through the same callback, and it is only `Send`.
        let callback = Arc::new(Mutex::new(callback));

        // Raised by the output thread when the child dies, read by the accept loop
        // so a child that crashes on startup does not leave it waiting out the
        // full timeout.
        let exited = Arc::new(AtomicBool::new(false));

        let output_callback = {
            let callback = Arc::clone(&callback);
            let exited = Arc::clone(&exited);
            move |event: ProcessEvent| match event {
                ProcessEvent::Stdout(data) | ProcessEvent::Stderr(data) => {
                    (callback.lock().unwrap())(WorkerEvent::LogLine(data.into_lossy_string()));
                }
                ProcessEvent::ProcessExited(code) => {
                    exited.store(true, Ordering::Release);
                    (callback.lock().unwrap())(WorkerEvent::ProcessExited(code));
                }
            }
        };

        let envs = [
            (CHANNEL_ENV_VAR, channel_name.as_os_str()),
            // The engine embeds a Python interpreter, and a script's `print()`
            // is meant to be watched as it happens. Python block-buffers its
            // output whenever it is not a terminal, which behind a pipe means a
            // long script says nothing until it ends.
            ("PYTHONUNBUFFERED", OsStr::new("1")),
        ];

        // What the program prints — the engine's logs, a script's `print()`, CBC's
        // C++ log — travels on the child's standard streams. What kind of streams
        // those are is the one place the two platforms part ways.
        //
        // On unix the pty earns its keep. Closing its master hangs the child up,
        // and that SIGHUP cascade is how a subtree dies with its parent (see the
        // note at the top of `process.rs`). A C library also sees a terminal and
        // line-buffers, which is what makes a live log live.
        //
        // On windows it earns nothing. Teardown there is the job object's work,
        // the RPC has its own channel now, and ConPTY is a terminal emulator that
        // gets in the way: it reflows output at the console width, injects escape
        // sequences, and stalls at startup waiting for an answer to the cursor
        // position report it sends — an answer a log reader does not know how to
        // give, so the child hangs before printing a single line. Plain pipes have
        // none of those problems.
        #[cfg(unix)]
        let process =
            Process::spawn_pty(exe.as_os_str(), &["--rpc-engine"], &envs, output_callback)?;
        #[cfg(windows)]
        let process =
            Process::spawn_pipes(exe.as_os_str(), &["--rpc-engine"], &envs, output_callback)?;

        let rpc_writer: RpcWriter = Arc::new(Mutex::new(None));

        {
            let rpc_writer = Arc::clone(&rpc_writer);
            std::thread::spawn(move || {
                run_channel(listener, init_msg, rpc_writer, exited, |event| {
                    (callback.lock().unwrap())(event)
                });
            });
        }

        Ok(Worker {
            process,
            rpc_writer,
            protocol: PhantomData,
        })
    }

    /// A clone of the worker's RPC writer slot, for callbacks that need to answer
    /// the worker from a reader thread. Writes become [`SendError::Finished`] once
    /// the channel closes.
    pub fn get_rpc_writer(&self) -> RpcWriter {
        Arc::clone(&self.rpc_writer)
    }

    /// Send an RPC [`ResultMsg`] to the worker.
    ///
    /// Returns [`SendError::Finished`] if the worker has already exited or been killed,
    /// or [`SendError::Io`] on a genuine write failure.
    pub fn send_rpc_message(&self, msg: ResultMsg<P>) -> Result<(), SendError> {
        send_via_rpc(&self.rpc_writer, msg)
    }

    /// Kill the worker if it is still running. Idempotent (see [`Process::kill`]).
    pub fn kill(&self) -> Result<(), KillError> {
        self.process.kill()
    }
}

/// Write one answer to the worker.
///
/// The slot is untyped, so nothing here ties `P` to the worker it belongs to;
/// the two callers outside this module both hold a `NoApp` worker.
pub(crate) fn send_via_rpc<P: AppProtocol>(
    writer: &RpcWriter,
    msg: ResultMsg<P>,
) -> Result<(), SendError> {
    let mut guard = writer.lock().unwrap();
    let Some(writer) = guard.as_mut() else {
        return Err(SendError::Finished);
    };
    match writer.send(&msg.to_text_msg()) {
        Ok(()) => Ok(()),
        // The peer is gone. Which error says so depends on the platform and on
        // how far the write got, so treat the whole family as a clean ending.
        Err(ChannelError::Io(e))
            if matches!(
                e.kind(),
                ErrorKind::BrokenPipe
                    | ErrorKind::ConnectionReset
                    | ErrorKind::ConnectionAborted
                    | ErrorKind::NotConnected
            ) =>
        {
            Err(SendError::Finished)
        }
        Err(ChannelError::Io(e)) => Err(SendError::Io(e)),
        Err(e) => Err(SendError::Io(std::io::Error::other(e.to_string()))),
    }
}

/// The channel thread: wait for the worker, hand it its init message, then relay
/// everything it says until it stops saying anything.
fn run_channel<P: AppProtocol>(
    listener: ChannelListener,
    init_msg: InitMsg<P>,
    rpc_writer: RpcWriter,
    exited: Arc<AtomicBool>,
    emit: impl Fn(WorkerEvent<P>),
) {
    let endpoint = match accept_worker(&listener, &exited) {
        Ok(endpoint) => endpoint,
        Err(e) => {
            emit(WorkerEvent::Error(e));
            return;
        }
    };
    // One worker, one connection. Dropping the listener now also removes the
    // socket's directory on unix; the connection already made is unaffected.
    drop(listener);

    let (mut reader, mut writer) = endpoint.split();

    // The init message goes first, before the slot is published, so nothing can
    // slip a frame in ahead of it.
    if let Err(e) = writer.send(&init_msg.to_text_msg()) {
        emit(WorkerEvent::Error(WorkerError::Channel(e.to_string())));
        return;
    }
    *rpc_writer.lock().unwrap() = Some(writer);

    loop {
        match reader.recv() {
            Ok(Some(body)) => match CompleteCmdMsg::<P>::from_text_msg(&body) {
                Ok(CompleteCmdMsg::CmdMsg(cmd)) => emit(WorkerEvent::RpcCommand(Ok(cmd))),
                Ok(CompleteCmdMsg::GracefulExit) => emit(WorkerEvent::GracefulExit),
                Err(e) => emit(WorkerEvent::RpcCommand(Err(e))),
            },
            // The worker closed its end, which in practice means it exited.
            Ok(None) => break,
            Err(e) => {
                emit(WorkerEvent::Error(WorkerError::Channel(e.to_string())));
                break;
            }
        }
    }

    // Nothing will be read any more, so nothing can be answered any more.
    *rpc_writer.lock().unwrap() = None;
}

fn accept_worker(listener: &ChannelListener, exited: &AtomicBool) -> Result<Endpoint, WorkerError> {
    let deadline = Instant::now() + CONNECT_TIMEOUT;
    loop {
        match listener.try_accept() {
            Ok(Some(endpoint)) => return Ok(endpoint),
            Ok(None) => {}
            Err(e) => return Err(WorkerError::NoConnection(e.to_string())),
        }

        if exited.load(Ordering::Acquire) {
            // One last look before giving up: the worker may have connected and
            // died between the try above and this check, and that connection is
            // still sitting in the backlog.
            if let Ok(Some(endpoint)) = listener.try_accept() {
                return Ok(endpoint);
            }
            return Err(WorkerError::NoConnection(String::from(
                "il s'est terminé avant de se connecter",
            )));
        }

        if Instant::now() >= deadline {
            return Err(WorkerError::NoConnection(format!(
                "délai de {} s dépassé",
                CONNECT_TIMEOUT.as_secs()
            )));
        }

        std::thread::sleep(CONNECT_POLL_INTERVAL);
    }
}
