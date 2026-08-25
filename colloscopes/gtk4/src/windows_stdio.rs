//! Keeping a console-less executable from dying when it prints, on Windows.
//!
//! The binary is built for the `windows` subsystem, so that no console window
//! opens beside the GUI (see the note at the top of `main.rs`). Windows then
//! gives the process no standard handles at all, and a null handle is not a
//! silent sink: `std::io` reports `ERROR_INVALID_HANDLE`, and `println!` turns
//! a failed write into a panic. With no console, that panic is invisible too --
//! the program would simply vanish.
//!
//! So the output is thrown away on purpose. [`discard_output`] points the two
//! output streams at `NUL`, where a write succeeds and goes nowhere.
//!
//! **There is no command line on Windows.** `--help` and `--version` print
//! nothing there, and that is the accepted state rather than a gap to fill
//! later. Windows decides console-or-GUI from a single flag in the
//! executable, at build time, and a program cannot be both. The way back was
//! tried and abandoned: `AttachConsole(ATTACH_PARENT_PROCESS)` plus
//! `SetStdHandle`, plus `_open_osfhandle`/`_dup2` for the C runtime's separate
//! descriptor table. It restored Rust's output but never CBC's, and PowerShell
//! redirection stayed unreliable because it does not hand a
//! `windows`-subsystem program a real handle at all. Not worth the machinery
//! for a command line no teacher runs.

use std::fs::OpenOptions;
use std::os::windows::io::IntoRawHandle;

use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Console::{
    GetStdHandle, STD_ERROR_HANDLE, STD_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle,
};

/// Make printing harmless. Call it first thing in `main`, before anything can
/// print -- clap reports a bad argument by printing.
///
/// Only the two output streams. A read from a null stdin comes back as an
/// ordinary error, which no caller here turns into a panic.
pub fn discard_output() {
    silence_if_unset(STD_OUTPUT_HANDLE);
    silence_if_unset(STD_ERROR_HANDLE);
}

/// Point one standard stream at `NUL`, unless it already points somewhere.
///
/// The "unless" is what makes this safe to run unconditionally, and it is not a
/// detail: the `--rpc-engine` worker is this same executable, and its whole log
/// -- the engine's messages, a script's `print()`, CBC's solver output -- goes
/// out on a pipe the parent handed it at creation time. That is a real handle,
/// so it is left alone. Replacing it would throw away the one output stream
/// that anybody reads.
///
/// A shell redirection is the same case and survives for the same reason.
fn silence_if_unset(id: STD_HANDLE) {
    let current = unsafe { GetStdHandle(id) };
    if !current.is_null() && current != INVALID_HANDLE_VALUE {
        return;
    }

    // `NUL` is a device name, opened like a file. Going through `OpenOptions`
    // rather than `CreateFileW` keeps the share mode and the creation
    // disposition std's problem.
    let Ok(file) = OpenOptions::new().write(true).open("NUL") else {
        return;
    };

    // Deliberately leaked. The handle has to stay open for as long as anything
    // might print, which is the whole life of the process, and a `File` dropped
    // here would close it behind `std::io`'s back.
    let handle = file.into_raw_handle() as HANDLE;
    unsafe { SetStdHandle(id, handle) };
}
