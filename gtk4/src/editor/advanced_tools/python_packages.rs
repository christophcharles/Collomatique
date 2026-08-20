//! Installing a Python module, where the packaging gives us somewhere to put
//! one.
//!
//! Two packagings ship their own interpreter and own a directory for what a
//! user installs into it: the flatpak, where the runtime points
//! `PYTHONUSERBASE` at the private data directory, and Windows, where
//! `collomatique_site.py` owns a directory under `%APPDATA%`. Each has a shell
//! script doing exactly this from a terminal --
//! `pkgs/flatpak/collomatique-pip` and `pkgs/windows/collomatique-pip.cmd` --
//! and the command built here is the same one, for whoever would rather press
//! a button.

use std::path::{Path, PathBuf};

/// Whether this process runs inside a Flatpak sandbox
///
/// `/.flatpak-info` is placed in the sandbox's own mount namespace, so a
/// process outside it does not see the file. The `FLATPAK_ID` variable says the
/// same thing but is inherited by children, including those that have left the
/// sandbox, so the file is the sounder test. GTK itself asks this way.
///
/// Linux rather than unix: Flatpak exists nowhere else, and a macOS build
/// asking this question is a mistake worth a compiler error rather than a
/// `false`.
#[cfg(target_os = "linux")]
pub fn in_flatpak() -> bool {
    std::path::Path::new("/.flatpak-info").exists()
}

/// Whether the Python library came with Collomatique
///
/// It did on Windows, where it is installed beside the executable, and in the
/// flatpak, where it is in `/app`. On any other Linux it is the machine's own,
/// shared with everything else installed there -- which is what makes
/// installing a module a different question.
///
/// There is deliberately no answer for a platform we do not package. A third
/// one has to say which of the two situations it is in; guessing here would put
/// a wrong sentence on screen instead of stopping the build.
#[cfg(windows)]
pub fn python_is_bundled() -> bool {
    true
}

#[cfg(target_os = "linux")]
pub fn python_is_bundled() -> bool {
    in_flatpak()
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn python_is_bundled() -> bool {
    compile_error!(
        "this platform has to say where its Python comes from: shipped with \
         Collomatique, or the machine's own"
    )
}

/// Why no install command could be built
///
/// Shown as it is in the install window's log, so the text is French. The
/// variants differ by platform because the ways of failing do: there is nothing
/// to look up in the flatpak, where the interpreter is at a path the runtime
/// fixes, and nothing to be outside on Windows.
#[derive(Debug)]
pub enum InstallCommandError {
    /// This build uses the machine's Python, so there is no directory of ours
    /// to install into. Only reachable if the button was shown where it should
    /// not have been.
    #[cfg(target_os = "linux")]
    NotBundled,
    /// `std::env::current_exe` failed, so there is no directory to look in.
    #[cfg(windows)]
    ExecutablePath(std::io::Error),
    /// No interpreter beside the executable.
    #[cfg(windows)]
    Interpreter(PathBuf),
    /// The interpreter would not say where to install. Carries what went wrong.
    #[cfg(windows)]
    Prefix(String),
}

impl std::fmt::Display for InstallCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(target_os = "linux")]
            InstallCommandError::NotBundled => write!(
                f,
                "Cette installation de Collomatique utilise le Python du système : \
                 les modules s'installent avec le gestionnaire de paquets de la distribution."
            ),
            #[cfg(windows)]
            InstallCommandError::ExecutablePath(e) => {
                write!(f, "Impossible de déterminer l'exécutable courant : {e}")
            }
            #[cfg(windows)]
            InstallCommandError::Interpreter(path) => {
                write!(f, "Interpréteur Python introuvable : {}", path.display())
            }
            #[cfg(windows)]
            InstallCommandError::Prefix(reason) => {
                write!(f, "Impossible de savoir où installer le module : {reason}")
            }
        }
    }
}

/// A pip invocation: what to spawn, and what to show for it.
pub struct InstallCommand {
    program: PathBuf,
    args: Vec<String>,
}

impl InstallCommand {
    /// The command that installs `packages` into this packaging's directory.
    ///
    /// Both packagings run the interpreter they ship; only the destination
    /// differs, and each one's reason is already written down beside the shell
    /// script that had to choose it first.
    ///
    /// In the flatpak the runtime points `PYTHONUSERBASE` at
    /// `/var/data/python`, the sandbox name of the private data directory, so
    /// `--user` is the whole mechanism and the path never has to be named. The
    /// interpreter is at a path the manifest fixes, which is why `/app/bin`
    /// appears here spelled exactly as `pkgs/flatpak/collomatique-pip` spells
    /// it.
    #[cfg(target_os = "linux")]
    pub fn build(packages: &[String]) -> Result<Self, InstallCommandError> {
        // Unreachable while the button is hidden outside the flatpak, but
        // asked rather than assumed: this is the one thing that makes the
        // command correct.
        if !in_flatpak() {
            return Err(InstallCommandError::NotBundled);
        }

        let mut args = pip_install_args();
        args.push("--user".to_string());
        args.extend(packages.iter().cloned());

        Ok(InstallCommand {
            program: PathBuf::from("/app/bin/python3"),
            args,
        })
    }

    /// On Windows the interpreter sits beside the executable, in a folder
    /// chosen at install time that cannot be written down here. The
    /// destination is asked of `collomatique_site.py`, which owns it, exactly
    /// as `pkgs/windows/collomatique-pip.cmd` asks -- so the version-carrying
    /// path stays in one place.
    #[cfg(windows)]
    pub fn build(packages: &[String]) -> Result<Self, InstallCommandError> {
        let program = interpreter_beside_executable()?;
        let prefix = ask_install_prefix(&program)?;

        let mut args = pip_install_args();
        args.push("--prefix".to_string());
        args.push(prefix);
        args.extend(packages.iter().cloned());

        Ok(InstallCommand { program, args })
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    pub fn build(_packages: &[String]) -> Result<Self, InstallCommandError> {
        compile_error!(
            "this platform has to say how a Python module is installed into the \
             Python it ships"
        )
    }

    pub fn program(&self) -> &Path {
        &self.program
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// The command as one line, quoted enough to be pasted into a terminal.
    ///
    /// Only the destination path can hold a space in practice ("C:\Program
    /// Files\..."), but the decision is made per argument rather than per
    /// platform.
    pub fn command_line(&self) -> String {
        let program = self.program.to_string_lossy();

        std::iter::once(quote(&program))
            .chain(self.args.iter().map(|arg| quote(arg)))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn quote(arg: &str) -> String {
    if arg.contains(' ') {
        format!("\"{arg}\"")
    } else {
        arg.to_string()
    }
}

/// The part of the pip line that does not depend on the packaging.
///
/// `--no-warn-script-location`: the scripts directory of that destination is
/// not on PATH and is not meant to be; only imports matter here. Both shipped
/// shell scripts pass it for the same reason.
///
/// `--progress-bar off`: pip draws its bar with carriage returns, which a
/// terminal redraws in place and a text view simply accumulates. The scripts do
/// not pass it because they run in a real terminal.
fn pip_install_args() -> Vec<String> {
    [
        "-m",
        "pip",
        "install",
        "--no-warn-script-location",
        "--progress-bar",
        "off",
    ]
    .iter()
    .map(|arg| arg.to_string())
    .collect()
}

/// `python.exe` from the folder Collomatique was installed into.
#[cfg(windows)]
fn interpreter_beside_executable() -> Result<PathBuf, InstallCommandError> {
    let exe = std::env::current_exe().map_err(InstallCommandError::ExecutablePath)?;

    let interpreter = match exe.parent() {
        Some(dir) => dir.join("python.exe"),
        None => return Err(InstallCommandError::Interpreter(exe)),
    };

    if !interpreter.is_file() {
        return Err(InstallCommandError::Interpreter(interpreter));
    }

    Ok(interpreter)
}

/// Where `collomatique_site.py` says a module goes.
///
/// Run with no console of its own: `python.exe` is a console program, so
/// without `CREATE_NO_WINDOW` this one-shot query flashes a window in front of
/// the application.
///
/// `None` is rejected rather than passed on. That is what the module prints
/// when `%APPDATA%` is unset, and `--prefix None` would quietly install into a
/// folder of that name.
#[cfg(windows)]
fn ask_install_prefix(interpreter: &Path) -> Result<String, InstallCommandError> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let output = std::process::Command::new(interpreter)
        .arg("-c")
        .arg("import collomatique_site; print(collomatique_site.prefix())")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| InstallCommandError::Prefix(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = stderr.trim();
        return Err(InstallCommandError::Prefix(if reason.is_empty() {
            "l'interpréteur a échoué".to_string()
        } else {
            reason.to_string()
        }));
    }

    let answer = String::from_utf8(output.stdout)
        .map_err(|_| InstallCommandError::Prefix("réponse illisible".to_string()))?;
    let prefix = answer.trim();

    if prefix.is_empty() || prefix == "None" {
        return Err(InstallCommandError::Prefix(
            "aucun profil utilisateur (APPDATA)".to_string(),
        ));
    }

    Ok(prefix.to_string())
}
