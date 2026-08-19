//! Giving a console-less executable somewhere to print, on Windows.
//!
//! The binary is built for the `windows` subsystem, so Windows never gives it a
//! console (see the note at the top of `main.rs`). That is what stops a console
//! window from opening beside the GUI, and it is right for the thing a teacher
//! double-clicks. It is wrong for `--help`, `--version` and `--debug`, which
//! exist to be read at a terminal.
//!
//! Two problems follow from having no console, not one:
//!
//!   - nothing printed goes anywhere;
//!   - a standard handle that is null is not a silent sink, it is an error.
//!     `std::io` reports `ERROR_INVALID_HANDLE`, and `println!` turns a failed
//!     write into a panic. So printing at all would abort the program.
//!
//! [`attach`] answers both. If the program was started from a terminal, it
//! joins that terminal's console and prints there. If it was not, it points the
//! streams at `NUL`, where a write succeeds and is discarded.
//!
//! There are also two records of where the streams go, not one, and repairing
//! only the first is what the initial version of this module got wrong. Windows
//! keeps the standard handles, which `std::io` re-reads on every write. The C
//! runtime keeps its own descriptor table, built once at startup from those
//! handles as they were before `main` ran -- when they were still null. Nothing
//! done afterwards revives it, so C code goes on printing into a closed
//! descriptor. That is where CBC's log goes: it prints through
//! `CoinMessageHandler` to C's `stdout`.

use std::fs::OpenOptions;
use std::os::windows::io::IntoRawHandle;

use libc::c_int;
use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Console::{
    ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE, STD_HANDLE,
    STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle,
};

/// Connect the standard streams to something that can be written to.
///
/// Call this first thing in `main`, before the arguments are parsed: a parse
/// error is one of the messages this exists to make visible.
///
/// One wart, and it belongs to the platform rather than to this code: a console
/// process does not wait for a `windows`-subsystem one, so `cmd` prints its next
/// prompt straight away and our output arrives underneath it.
///
/// The `--rpc-engine` child runs this too, since it is the same executable, and
/// nothing happens: the parent hands it pipes for all three streams, and a
/// stream that already has a handle is left alone.
pub fn attach() {
    // ATTACH_PARENT_PROCESS asks for the console of whoever started us. It
    // fails when there is none -- started from Explorer, from the Start menu,
    // by the shell association on a document -- and that failure is the normal
    // case, not an error to report.
    let attached = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } != 0;

    if attached {
        // Attaching gives the process a console. It does not fill in the
        // standard handles: those were decided when the process was created,
        // and for a windows-subsystem program the system left them null.
        bind_if_unset(STD_INPUT_HANDLE, 0, "CONIN$");
        bind_if_unset(STD_OUTPUT_HANDLE, 1, "CONOUT$");
        bind_if_unset(STD_ERROR_HANDLE, 2, "CONOUT$");
    } else {
        // No console to print to, so make printing harmless instead of fatal.
        // Only the two output streams: a read from a null stdin comes back as
        // an ordinary error, which no caller turns into a panic.
        bind_if_unset(STD_OUTPUT_HANDLE, 1, "NUL");
        bind_if_unset(STD_ERROR_HANDLE, 2, "NUL");
    }
}

/// Point one standard stream at `device`, unless it already points somewhere.
///
/// `crt_fd` is the C descriptor that goes with `id`: 0, 1 or 2.
///
/// The "unless" is the important half. A shell that redirects -- `collomatique
/// --debug ... > log.txt` -- passes the file as a real handle at creation time,
/// and that handle is what the user asked for. Overwriting it would send the
/// output to the console they redirected it away from. It is also why the C
/// runtime needs no repair in that case: a handle that was already there before
/// `main` is a handle the runtime read at startup, so its descriptor is live.
fn bind_if_unset(id: STD_HANDLE, crt_fd: c_int, device: &str) {
    let current = unsafe { GetStdHandle(id) };
    if !current.is_null() && current != INVALID_HANDLE_VALUE {
        return;
    }

    // `CONIN$`, `CONOUT$` and `NUL` are device names, opened like files. Going
    // through `OpenOptions` rather than `CreateFileW` keeps the share mode and
    // the creation disposition std's problem; read plus write is what the
    // console devices want, and `NUL` accepts anything.
    let Ok(file) = OpenOptions::new().read(true).write(true).open(device) else {
        return;
    };

    // Deliberately leaked. The handle has to stay open for as long as anything
    // might print, which is the whole life of the process, and a `File` dropped
    // here would close it behind `std::io`'s back.
    let handle = file.into_raw_handle() as HANDLE;
    unsafe { SetStdHandle(id, handle) };

    // And now the same thing again for C. `open_osfhandle` wraps the handle in
    // a fresh descriptor, and `dup2` moves it onto the well-known number, which
    // is what `stdout` and `stderr` are permanently tied to -- so `printf`
    // starts working without anyone reopening a stream.
    //
    // Flags are 0, meaning binary: no CRLF translation. That matches what Rust
    // writes, which never translates either, so a log carrying both has one
    // kind of line ending rather than two.
    //
    // The intermediate descriptor is left open on purpose, for the same reason
    // the handle is: closing it here is exactly what we do not want.
    let fd = unsafe { libc::open_osfhandle(handle as libc::intptr_t, 0) };
    if fd >= 0 {
        unsafe { libc::dup2(fd, crt_fd) };
    }
}
