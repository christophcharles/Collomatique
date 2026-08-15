# Windows packaging roadmap

## Goal

A Windows installer with the same end-user experience as the flatpak: the user
double-clicks `Collomatique-Setup-<version>.exe`, possibly dismisses the
SmartScreen warning (the installer is unsigned for now), clicks through
next-next-next, and Collomatique is installed with `.collomatique` files
associated to it. Double-clicking a `.collomatique` file opens it in the app.

No code signing, no store distribution, no auto-update for now. Builds happen
manually in a Windows 11 VM. The flatpak is the ground truth for what the app
needs at runtime.

A second goal shapes every choice below: **long-term maintainability**. The
build must be one script that still works in six months or a year. When a
choice trades convenience today against that, maintainability wins.

## What the app needs (established by exploring the tree)

- **One binary.** `collomatique-gtk4` is a multiplexed exe: the GUI re-executes
  `current_exe()` with `--rpc-engine` for solver and Python workers
  (`subprocesses/src/worker.rs`, `EngineExe::Current`). `python-runner` is a
  library, not a binary. So the one exe links GTK4, libadwaita, CBC and
  libpython all at once.
- **GTK floor**: the crates pin the API floors — `gtk4 = "0.9"` with `v4_10`
  (GTK >= 4.10) and `libadwaita = "0.7"` with `v1_7` (adw >= 1.7)
  (`gtk4/Cargo.toml`). Any GTK 4.10+/adw 1.7+ build satisfies them.
- **CBC**: `collo-cbc/build.rs` finds CBC >= 2.10 through pkg-config (`cbc.pc`)
  and compiles `cpp/collo_cbc.cpp` as C++17 with the `cc` crate. The shim
  imports the *data symbol* `cbcPreProcessPointer` from libCbcSolver. MSVC
  cannot auto-import data symbols from DLLs (unlike mingw), and the COIN-OR
  headers carry no `dllimport` annotations, so on Windows CBC should be linked
  **statically**. That also removes the whole CBC DLL family from the bundle.
- **Python is mandatory** (no cargo feature turns it off): pyo3 0.29, no
  `abi3`, no `auto-initialize`; `python-runner/src/lib.rs` calls
  `Python::initialize()` explicitly. At runtime the exact `python3XX.dll` plus
  a matching stdlib must be present. Nothing in the repo sets
  `PYTHONHOME`/`PYTHONPATH` today.
- **User Python packages matter.** The flatpak documents how teachers install
  their own packages with pip. On Windows this means bundling the **official
  python.org interpreter**: PyPI binary wheels (`win_amd64`) target it.
  MSYS2's mingw Python is a patched build with its own platform tag and cannot
  install those wheels — that is the known caveat of plan C below.
- **Already Windows-aware**: `subprocesses/src/process.rs` has a real
  `#[cfg(windows)]` path (ConPTY through `portable-pty`, kill-on-close Job
  Objects through `windows-sys`); the worker exe lookup handles `EXE_SUFFIX`;
  `settings/` uses `directories` (maps to `%APPDATA%`); storage is plain JSON
  over `tokio::fs`; no sqlite, no openssl, no direct dbus. None of this has
  ever actually run on Windows.
- **rfd** is pinned workspace-wide to `default-features = false,
  features = ["xdg-portal", "tokio"]`. In rfd those features only select the
  Linux backend; the Win32 `IFileDialog` backend is not feature-gated, so this
  should compile and give native dialogs on Windows. Verify, don't assume.
- **gresources**: `gtk4/build.rs` needs `glib-compile-resources` on PATH at
  build time.
- Bundled Python package in the flatpak: **xlsxwriter** only.

## Toolchain plans, in order

Every project in the stack has its own blessed way of existing on Windows, so
instead of one bet the roadmap is a ladder. Try plan A; if it fails in a way
that is not worth fighting, drop to the next plan. Python comes from
python.org in plans A and B; only plan C compromises on that.

**Plan A — vcpkg + MSVC for everything.** VS Build Tools (MSVC compiler +
Windows SDK; no Visual Studio IDE), rustup's default
`x86_64-pc-windows-msvc` toolchain, and vcpkg as the single C/C++ package
manager: `gtk` (port at 4.22.x as of writing), `libadwaita` (1.8.x),
`coin-or-cbc` (static triplet). One package manager, one manifest to pin,
official Python. Known costs: vcpkg builds from source (first GTK build takes
an hour or two in the VM, cached afterwards), and gtk4-rs against vcpkg-built
GTK is less traveled than the gtk-rs book's recommended path — which is why
there is a plan B.

**Plan B — each project's blessed way.** GTK stack through **gvsbuild** (the
gtk-rs book's recommended Windows route, so the exact combination the gtk-rs
developers test), CBC through **coinbrew** (COIN-OR's own build tool; on
Windows it runs from an MSYS2 or Cygwin shell with `cl` on PATH and
`--enable-msvc`, so MSYS2 appears here purely as a build shell while the
output stays MSVC), Python from python.org, Rust msvc. Alternative for the
CBC piece if coinbrew fights us: vcpkg's `coin-or-cbc` port, as in plan A.
More moving parts than plan A (several build systems), but maximum upstream
blessing per component. Pin the gvsbuild version (and the coinbrew/CBC
versions) in the build script so a rebuild next year uses the same recipe;
bumping a pin is a deliberate act, like bumping the flatpak runtime.

**Plan C — MSYS2.** Everything prebuilt as mingw binaries via pacman (gtk4,
libadwaita, coin-or-cbc, Python), Rust windows-gnu, one `pacman -S` line and
the whole stack exists. Fastest path to a running app by far, and the fallback
of last resort. Caveat: the bundled Python would be mingw Python, so teachers
lose PyPI binary wheels — or we attempt the hybrid (mingw-built app linking
the official `python3XX.dll` through the C ABI), which is the least-documented
configuration of the three.

Installer, independent of the ladder: **Inno Setup 6**. It produces exactly
the wanted UX (one setup.exe, wizard, Start-menu entry, uninstaller,
registry-based file association) from a declarative `.iss` script compiled on
the command line by `ISCC.exe`. MSIX was considered and does not fit the
stated UX: it hard-requires a trusted signature, so there is no
next-next-next path with an unsigned package.

## Roadmap

### Step 1 — Clean the source, on Linux, before anything else

All of this is ordinary Linux-side work, committed and tested before the VM
enters the picture. Every Cargo.toml/Cargo.lock change means a
`collomatique.nix` cargoHash refresh (run by the user) before committing.

1. Remove the unused dependencies from `gtk4/Cargo.toml`: `rustix` (whose
   `pty` feature is unix-only and would likely be the first Windows compile
   error), `portable-pty`, and `libc`. Grep shows zero references in
   `gtk4/src`; the real users of `portable-pty`/`libc` live in `subprocesses/`
   with proper `cfg` guards. Re-verify before deleting.
2. `collo-cbc/build.rs`: emit `cargo:rustc-link-lib=stdc++` only for
   non-MSVC targets. (`cc` already handles the C++ runtime per toolchain;
   `.std("c++17")` maps to `/std:c++17` under cl.exe.)
3. Audit the remaining `#[cfg(unix)]` sites reachable from the gtk4 exe for a
   missing Windows counterpart. Known-fine example: the SIGHUP reset in
   `rpc-engine/src/lib.rs` is correctly unix-only. The point of the audit is
   to find any that are not.
4. Migrate `flatpak/` to `pkgs/flatpak/`, making room for `pkgs/windows/`
   later. Not a pure rename: the manifest builds the app from
   `type: dir, path: ..` (the repo root, which becomes `../..`) and
   `flatpak/build.sh` keeps its build/state directories outside the tree with
   relative paths — both need adjusting, and the flatpak must be rebuilt once
   to confirm.
5. Run the normal test suite on Linux to prove none of this regresses the
   flatpak platform.

### Step 2 — VM bootstrap (manual, once)

Install in the Windows 11 VM: VS Build Tools 2022+ (workload "Desktop
development with C++", which includes the Windows SDK), rustup (default msvc
toolchain), git, Python 3.x from python.org, Inno Setup 6, and a vcpkg clone
(`bootstrap-vcpkg.bat`).

### Step 3 — Plan A spike: vcpkg GTK stack + gtk4-rs wiring

Prove the risky part on a hello-world before touching the workspace.

1. `vcpkg install gtk libadwaita` (dynamic triplet `x64-windows`) and
   `vcpkg install coin-or-cbc:x64-windows-static-md` (static libs, dynamic
   CRT — matches Rust's default `/MD`).
2. Build a minimal gtk4-rs + libadwaita hello-world against it. The `-sys`
   crates go through `system-deps`/pkg-config: point `PKG_CONFIG` at vcpkg's
   pkgconf and `PKG_CONFIG_PATH` at `installed/x64-windows/lib/pkgconfig`.
   Record every env var that turns out to be needed; they become the setup
   section of the build script.
3. Check `glib-compile-resources` runs (vcpkg puts glib tools under
   `installed/<triplet>/tools/glib/`).
4. Gate: hello-world adw window renders → stay on plan A. Not workable in
   reasonable effort → redo this step under plan B (gvsbuild for the GTK
   stack, vcpkg kept for CBC), then plan C.

### Step 4 — First compile of the workspace

`cargo build --release -p collomatique-gtk4` in the VM, with the step 3
environment plus `PYO3_PYTHON` pointing at the python.org interpreter.
Verify the static CBC link line comes through pkg-config's `--static`
metadata. Fix whatever the compiler surfaces (expected to be little, after
step 1).

### Step 5 — First run: native-stack verification

Run the exe from the build environment and verify, in order of risk:

1. **CBC + `cbcPreProcessPointer`**: solve a colloscope from an `examples/`
   file; confirm mid-solve incumbents arrive (that path exercises the data
   symbol).
2. **Worker spawn over ConPTY**: the `--rpc-engine` re-exec path — first
   real-world run of the existing `cfg(windows)` code in `subprocesses/`.
3. **Python embedding**: interpreter initializes, stdlib found, a script
   using xlsxwriter runs.
4. **rfd**: open/save show native Win32 dialogs.
5. General GTK/adwaita behaviour: rendering, HiDPI, keyboard input, whether
   dark mode follows the Windows setting (observe and note; no promise).

### Step 6 — Windows polish (small code changes)

- `#![cfg_attr(windows, windows_subsystem = "windows")]` on
  `gtk4/src/main.rs` so no console window flashes. The `--rpc-engine` child
  must keep working — its stdio goes through ConPTY, which does not require a
  console-subsystem exe — verify it after the change.
- Embed the app icon and version info into the exe (taskbar/Explorer icon),
  e.g. with the `winresource` crate in `gtk4/build.rs` under `cfg(windows)`,
  from an `.ico` generated once out of
  `resources/icons/collomatique-{128,256,512}.png` and committed (same
  reasoning as the pre-scaled flatpak PNGs).
- Python path setup only if step 5.3 required it (see step 7 for the layout
  that should make it unnecessary).

### Step 7 — Bundle layout

Stage an install tree that runs on a machine with no dev tooling at all:

```
Collomatique/
  collomatique-gtk4.exe
  *.dll                          # GTK/adw/glib/cairo/pango... harvested from the
                                 # build env (CBC is static: nothing to ship)
  python3XX.dll + python stdlib  # see below
  lib/gdk-pixbuf-2.0/...         # loaders + regenerated loaders.cache
  share/glib-2.0/schemas/gschemas.compiled
  share/icons/Adwaita/  share/icons/hicolor/   # incl. Collomatique icons
  share/locale/                  # GTK/adw French translations
```

Python bundling: start from the **python.org embeddable package**
(`python-3.X.Y-embed-amd64.zip`, made exactly for app embedding). Ship it
with a `python3XX._pth` under our control that adds a bundled `site-packages`
(xlsxwriter, version-pinned like the flatpak) and enables the user site
directory (`%APPDATA%\Python\...`) so `pip install --user` works for
teachers. If the `._pth` mechanism fights user-site enablement, fall back to
shipping the full python.org layout instead — bigger, but standard. The
bundled minor version is whatever `PYO3_PYTHON` linked (pyo3 is not abi3):
the packaging script must detect it, never hardcode `3.12`.

Acceptance for this step: the staged tree runs on a Windows machine without
vcpkg, Build Tools or Python installed.

### Step 8 — Installer + one-command build

Home: `pkgs/windows/`, next to `pkgs/flatpak/` (the migration happened in
step 1), so all OS packaging lives under `pkgs/`.

- `build.ps1` — plain PowerShell, the whole build in one command:
  check the step 2 tools are present; `vcpkg install` from a committed
  manifest; set the env vars recorded in step 3; `cargo build --release -p
  collomatique-gtk4`; stage the step 7 tree; read the version from the
  workspace `Cargo.toml` (same trick as `flatpak/build.sh`); run `ISCC.exe`
  on the `.iss` script to produce `Collomatique-Setup-<version>.exe`.
- `collomatique.iss` — Inno Setup script: `PrivilegesRequired=lowest` by
  default (per-user install, no UAC, association under HKCU) with an
  all-users option; `[Registry]` entries `Software\Classes\.collomatique` →
  `Collomatique.Document` with `shell\open\command "...\collomatique-gtk4.exe"
  "%1"` and `DefaultIcon`; `ChangesAssociations=yes`; Start-menu entry;
  uninstaller; French and English installer languages.

The exe already takes a file argument (the flatpak desktop file uses
`Exec=collomatique-gtk4 %f`), so double-clicked files ride the same path —
verify the clap parsing accepts a bare Windows path.

### Step 9 — End-to-end acceptance (clean VM snapshot)

On a fresh Windows 11 snapshot with nothing installed: run setup.exe →
SmartScreen "Run anyway" → next-next-next → double-click an
`examples/*.collomatique` file → the app opens it → a solve runs and streams
incumbents → an xlsxwriter export works → `pip install --user` of a binary
wheel (e.g. pandas) works from a user script → uninstall leaves no trace.
That is the definition of done.

## Version pinning summary

- Rust API floors: already pinned by the crates (`gtk4` 0.9/`v4_10`,
  `libadwaita` 0.7/`v1_7`).
- C libraries: vcpkg manifest with a `builtin-baseline` (plan A/C-for-CBC);
  gvsbuild version pin in the build script if plan B is reached.
- Python: minor version follows whatever the build linked; the packaging
  script detects and bundles that exact version.
- Bundled xlsxwriter: version-pinned, same as the flatpak.

## Out of scope for now

64-bit x86_64 only (no ARM64/32-bit); no code signing; no CI for the Windows
build (manual VM builds); distribution channels.
