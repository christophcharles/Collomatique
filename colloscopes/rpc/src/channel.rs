//! The private channel that carries the RPC between a host process and its
//! `<exe> --rpc-engine` worker.
//!
//! The worker's stdin/stdout/stderr stay on the pty, where they belong: they are
//! for the human. The protocol gets this channel instead — a local socket, byte
//! clean, with nothing between the two ends that could reflow, wrap or re-encode
//! what is written to it.
//!
//! The two sides meet by rendezvous. The host binds a listener under a fresh
//! unique name, hands that name to the worker in the [`CHANNEL_ENV_VAR`]
//! environment variable, and the worker connects on startup. An inherited file
//! descriptor or handle would be simpler, but `portable-pty` offers no way to
//! pass one: its `CommandBuilder` has no `pre_exec` hook on unix, and the ConPTY
//! path spawns with an explicit handle list that would exclude anything extra.
//!
//! Frames are a 4-byte little-endian length followed by that many bytes of UTF-8.
//! No markers, no chunking, no escaping, no line discipline.

use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicUsize, Ordering};

use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{
    ListenerNonblockingMode, ListenerOptions, RecvHalf, SendHalf, Stream,
};

#[cfg(unix)]
use interprocess::local_socket::GenericFilePath;
#[cfg(windows)]
use interprocess::local_socket::GenericNamespaced;

/// The environment variable through which a host tells its worker where to connect.
pub const CHANNEL_ENV_VAR: &str = "COLLOMATIQUE_RPC_CHANNEL";

/// Largest frame body we will accept, in bytes. Well above any real message
/// (the biggest is a serialized ILP problem), and low enough that a corrupt
/// length header is a reported error rather than a doomed allocation.
const MAX_FRAME_LEN: usize = 256 * 1024 * 1024;

/// Longest socket path we will build on unix. `sockaddr_un::sun_path` holds 108
/// bytes on Linux but only 104 on macOS, and the address must be NUL-terminated.
#[cfg(unix)]
const MAX_SOCKET_PATH_LEN: usize = 100;

#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("Erreur d'entrée/sortie sur le canal RPC : {0}")]
    Io(#[from] std::io::Error),
    #[error("La variable d'environnement {CHANNEL_ENV_VAR} est absente")]
    MissingEnvVar,
    #[error("Chemin de canal RPC trop long ({0} octets, maximum {MAX_SOCKET_PATH_LEN})")]
    #[cfg(unix)]
    NameTooLong(usize),
    #[error("Trame RPC de {0} octets, au-delà de la limite de {MAX_FRAME_LEN} octets")]
    FrameTooLarge(u64),
    #[error("Trame RPC tronquée")]
    Truncated,
    #[error("Trame RPC non-UTF-8")]
    NotUtf8,
}

/// A name nobody else in this process, or in any other, is using.
///
/// `<n>` counts within the process, so a worker that itself spawns workers
/// never collides with its own earlier channels.
fn fresh_stem() -> String {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("collo-rpc-{}-{}", std::process::id(), n)
}

/// The host side: a bound listener waiting for its worker.
pub struct Listener {
    inner: interprocess::local_socket::Listener,
    name: OsString,
    /// The private directory holding the socket file, removed on drop.
    #[cfg(unix)]
    dir: std::path::PathBuf,
}

impl Listener {
    /// Bind a fresh, uniquely named listener. Non-blocking: see [`Listener::try_accept`].
    #[cfg(unix)]
    pub fn bind() -> Result<Listener, ChannelError> {
        use std::os::unix::fs::DirBuilderExt;

        let dir = std::env::temp_dir().join(fresh_stem());

        // Mode on the directory, not on the socket: a unix socket ignores its own
        // permission bits on some systems, a directory never does. Setting it in
        // `DirBuilder` rather than afterwards leaves no window where it is world
        // readable.
        let mut builder = std::fs::DirBuilder::new();
        builder.mode(0o700);
        if let Err(e) = builder.create(&dir) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(e.into());
            }
            // Our own pid, so any process that could have made this directory is
            // gone: pids are unique among the living. It is a leftover from a
            // crash and it is ours to remove.
            std::fs::remove_dir_all(&dir)?;
            builder.create(&dir)?;
        }

        let path = dir.join("s");
        let name = path.into_os_string();
        if name.len() > MAX_SOCKET_PATH_LEN {
            let _ = std::fs::remove_dir_all(&dir);
            return Err(ChannelError::NameTooLong(name.len()));
        }

        match bind_at(&name) {
            Ok(inner) => Ok(Listener { inner, name, dir }),
            Err(e) => {
                let _ = std::fs::remove_dir_all(&dir);
                Err(e)
            }
        }
    }

    /// Bind a fresh, uniquely named listener. Non-blocking: see [`Listener::try_accept`].
    #[cfg(windows)]
    pub fn bind() -> Result<Listener, ChannelError> {
        let name = OsString::from(fresh_stem());
        let inner = bind_at(&name)?;
        Ok(Listener { inner, name })
    }

    /// The value to hand the worker in [`CHANNEL_ENV_VAR`].
    pub fn channel_name(&self) -> OsString {
        self.name.clone()
    }

    /// Take the worker's connection if it has arrived. `Ok(None)` means nobody
    /// has connected yet, so the caller can poll and give up on its own terms.
    pub fn try_accept(&self) -> Result<Option<Endpoint>, ChannelError> {
        match self.inner.accept() {
            Ok(stream) => Ok(Some(Endpoint { stream })),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(unix)]
impl Drop for Listener {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn bind_at(name: &OsStr) -> Result<interprocess::local_socket::Listener, ChannelError> {
    let name = resolve_name(name)?;
    let listener = ListenerOptions::new().name(name).create_sync()?;
    // Only the accept is non-blocking. The accepted stream stays blocking, which
    // is what the reader threads on both sides want.
    listener.set_nonblocking(ListenerNonblockingMode::Accept)?;
    Ok(listener)
}

/// Turn a channel name into the form the platform's backend expects.
///
/// This is the one place the two platforms differ, and the split is forced
/// rather than chosen: `GenericFilePath` is unsupported on Windows and
/// `GenericNamespaced` is unsupported on macOS, so a filesystem path on unix and
/// a namespaced name on Windows is the only combination that works everywhere.
///
/// Access control comes with each: on unix from the `0700` directory the socket
/// sits in, on Windows from the default named-pipe DACL, which already admits
/// only the creating user.
#[cfg(unix)]
fn resolve_name(name: &OsStr) -> Result<interprocess::local_socket::Name<'_>, ChannelError> {
    Ok(name.to_fs_name::<GenericFilePath>()?)
}

#[cfg(windows)]
fn resolve_name(name: &OsStr) -> Result<interprocess::local_socket::Name<'_>, ChannelError> {
    Ok(name.to_ns_name::<GenericNamespaced>()?)
}

/// One end of a connected channel, before it is split for reading and writing.
pub struct Endpoint {
    stream: Stream,
}

impl Endpoint {
    /// The worker side: connect to the channel named in [`CHANNEL_ENV_VAR`].
    pub fn connect_from_env() -> Result<Endpoint, ChannelError> {
        let name = std::env::var_os(CHANNEL_ENV_VAR).ok_or(ChannelError::MissingEnvVar)?;
        Endpoint::connect(&name)
    }

    /// Connect to a channel by name, as returned by [`Listener::channel_name`].
    pub fn connect(name: &OsStr) -> Result<Endpoint, ChannelError> {
        Ok(Endpoint {
            stream: connect_raw(name)?,
        })
    }

    /// Split into the two halves, so a reader thread and a writer can hold one each.
    pub fn split(self) -> (FrameReader, FrameWriter) {
        let (recv, send) = self.stream.split();
        (FrameReader { inner: recv }, FrameWriter { inner: send })
    }
}

fn connect_raw(name: &OsStr) -> Result<Stream, ChannelError> {
    let name = resolve_name(name)?;
    Ok(Stream::connect(name)?)
}

/// The receiving half of a channel.
pub struct FrameReader {
    inner: RecvHalf,
}

impl FrameReader {
    /// Read one whole frame, blocking until it is there.
    ///
    /// `Ok(None)` is a clean close by the peer, between frames — the normal way a
    /// channel ends.
    pub fn recv(&mut self) -> Result<Option<String>, ChannelError> {
        let mut header = [0u8; 4];
        if !fill(&mut self.inner, &mut header)? {
            return Ok(None);
        }

        let len = u32::from_le_bytes(header) as usize;
        // Checked before the allocation, not after it.
        if len > MAX_FRAME_LEN {
            return Err(ChannelError::FrameTooLarge(len as u64));
        }

        let mut body = vec![0u8; len];
        if !fill(&mut self.inner, &mut body)? {
            // EOF where a body was announced: the peer died mid-frame.
            return Err(ChannelError::Truncated);
        }

        String::from_utf8(body)
            .map(Some)
            .map_err(|_| ChannelError::NotUtf8)
    }
}

/// The sending half of a channel.
pub struct FrameWriter {
    inner: SendHalf,
}

impl FrameWriter {
    /// Send one whole frame.
    pub fn send(&mut self, body: &str) -> Result<(), ChannelError> {
        let len = u32::try_from(body.len())
            .map_err(|_| ChannelError::FrameTooLarge(body.len() as u64))?;
        if body.len() > MAX_FRAME_LEN {
            return Err(ChannelError::FrameTooLarge(body.len() as u64));
        }

        // Header and body in one `write_all`, so a frame never leaves half
        // written if the peer goes away between the two.
        let mut buf = Vec::with_capacity(4 + body.len());
        buf.extend_from_slice(&len.to_le_bytes());
        buf.extend_from_slice(body.as_bytes());

        self.inner.write_all(&buf)?;
        self.inner.flush()?;
        Ok(())
    }
}

/// Read until `buf` is full. `Ok(false)` is EOF before the first byte;
/// EOF part-way through is [`ChannelError::Truncated`].
fn fill(reader: &mut impl Read, buf: &mut [u8]) -> Result<bool, ChannelError> {
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..]) {
            Ok(0) => {
                return if filled == 0 {
                    Ok(false)
                } else {
                    Err(ChannelError::Truncated)
                };
            }
            Ok(n) => filled += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Block until the worker side connects. Tests only: production code polls
    /// `try_accept` against a deadline and the process's liveness.
    fn accept_blocking(listener: &Listener) -> Endpoint {
        for _ in 0..1000 {
            if let Some(endpoint) = listener.try_accept().unwrap() {
                return endpoint;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("nobody connected");
    }

    /// Bind, connect from another thread, and hand back both ends split.
    fn connected_pair() -> ((FrameReader, FrameWriter), (FrameReader, FrameWriter)) {
        let listener = Listener::bind().unwrap();
        let name = listener.channel_name();

        let worker = std::thread::spawn(move || Endpoint::connect(&name).unwrap().split());
        let host = accept_blocking(&listener).split();

        (host, worker.join().unwrap())
    }

    #[test]
    fn round_trip_both_directions() {
        let ((mut host_r, mut host_w), (mut worker_r, mut worker_w)) = connected_pair();

        host_w.send("{\"init\":true}").unwrap();
        assert_eq!(worker_r.recv().unwrap().as_deref(), Some("{\"init\":true}"));

        worker_w.send("{\"cmd\":1}").unwrap();
        assert_eq!(host_r.recv().unwrap().as_deref(), Some("{\"cmd\":1}"));

        host_w.send("{\"answer\":1}").unwrap();
        assert_eq!(worker_r.recv().unwrap().as_deref(), Some("{\"answer\":1}"));
    }

    #[test]
    fn body_larger_than_any_socket_buffer() {
        let ((_host_r, mut host_w), (mut worker_r, _worker_w)) = connected_pair();

        // A serialized ILP problem is this size. It must not deadlock: the send
        // fills the socket buffer long before it finishes, so the reader has to
        // be draining it concurrently.
        let big: String = "0123456789abcdef".repeat(4 * 1024 * 1024 / 16);
        assert!(big.len() >= 1024 * 1024);

        let sender = std::thread::spawn(move || host_w.send(&big).map(|()| big));
        let received = worker_r.recv().unwrap().unwrap();
        let sent = sender.join().unwrap().unwrap();

        assert_eq!(received, sent);
    }

    #[test]
    fn hostile_body_arrives_byte_identical() {
        let ((_host_r, mut host_w), (mut worker_r, _worker_w)) = connected_pair();

        // Everything the old in-band framing could not survive: bare newlines,
        // carriage returns, lines that look exactly like a frame marker, lone
        // markers, non-ASCII, and a run long past the old 80-byte chunk size.
        let body = concat!(
            "%%COLLOMATIQUE-RPC-MSG%%not a frame\n",
            "%%COLLOMATIQUE-RPC-CON%%\r\n",
            "%%COLLOMATIQUE-RPC-END%%",
            "\r\rprogrès : 42 % — élèves : « Éloïse », 日本語, 🎓\n\n",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );

        host_w.send(body).unwrap();
        let received = worker_r.recv().unwrap().unwrap();

        assert_eq!(received.as_bytes(), body.as_bytes());
    }

    #[test]
    fn clean_close_reads_as_end_of_stream() {
        let ((host_r, mut host_w), (mut worker_r, _worker_w)) = connected_pair();

        host_w.send("last").unwrap();

        // Both halves, not just the writer: they share one socket, so dropping
        // the writer alone closes nothing and the peer would block forever. In
        // production the close is the worker process exiting, which takes
        // everything with it.
        drop(host_w);
        drop(host_r);

        assert_eq!(worker_r.recv().unwrap().as_deref(), Some("last"));
        assert!(worker_r.recv().unwrap().is_none());
    }

    #[test]
    fn oversized_length_header_is_rejected() {
        let listener = Listener::bind().unwrap();
        let name = listener.channel_name();

        // Raw, so we can announce a length no `FrameWriter` would ever produce.
        let worker = std::thread::spawn(move || {
            let mut raw = connect_raw(&name).unwrap();
            raw.write_all(&u32::MAX.to_le_bytes()).unwrap();
            raw.flush().unwrap();
            // Hold the connection open: the reader must reject on the header
            // alone, without waiting for 4 GiB of body.
            std::thread::sleep(std::time::Duration::from_millis(500));
        });

        let (mut host_r, _host_w) = accept_blocking(&listener).split();
        match host_r.recv() {
            Err(ChannelError::FrameTooLarge(len)) => assert_eq!(len, u32::MAX as u64),
            other => panic!("expected FrameTooLarge, got {other:?}"),
        }

        worker.join().unwrap();
    }
}
