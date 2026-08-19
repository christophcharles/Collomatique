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
  their own packages with pip, and PyPI binary wheels (`win_amd64`) target a
  CPython built with MSVC. MSYS2's mingw Python is a patched build with its own
  platform tag and cannot install those wheels — that is the known caveat of
  plan C below.
- **Python is a build dependency, not a separate tool.** The app links
  `libpython`, so it comes from vcpkg alongside GTK and CBC. That makes the
  interpreter linked and the interpreter shipped the same one, which removes
  the whole question of matching a bundled runtime to whatever `PYO3_PYTHON`
  found. What it does not remove is the check in step 3: a vcpkg `python3`
  exists to be linked against, and it has to be shippable to a teacher too.
- **Already Windows-aware**: `subprocesses/src/process.rs` has a real
  `#[cfg(windows)]` path (ConPTY through `portable-pty`, kill-on-close Job
  Objects through `windows-sys`); the worker re-exec resolves its own path with
  `std::env::current_exe()`, which already carries the `.exe` suffix, so nothing
  builds an executable name by hand (`EXE_SUFFIX` appears nowhere in the tree);
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
that is not worth fighting, drop to the next plan. Python comes from vcpkg in
plans A and B; only plan C compromises on the MSVC build wheels need.

**Settled, in step 3: the split is A for everything except GTK, B for GTK.**
vcpkg builds CBC, Python and pkgconf, and does it well — that half is proven by
a real build, see step 3. The GTK stack comes from **gvsbuild**. Plan C was
never reached. The reasoning is under plan A below; the rest of this section is
kept because the alternatives are what we would fall back to, and because the
next person to touch this will want to know what was already ruled out.

**Plan A — vcpkg + MSVC for everything.** VS Build Tools (MSVC compiler +
Windows SDK; no Visual Studio IDE), rustup's default
`x86_64-pc-windows-msvc` toolchain, and vcpkg as the single C/C++ package
manager: `gtk` (port at 4.22.x as of writing), `libadwaita` (1.8.x),
`coin-or-cbc` (static triplet). One package manager, one manifest to pin,
official Python. Known costs: vcpkg builds from source (first GTK build takes
an hour or two in the VM, cached afterwards), and gtk4-rs against vcpkg-built
GTK is less traveled than the gtk-rs book's recommended path — which is why
there is a plan B.

**What happened to the GTK half of plan A.** It does not work, and the reason is
not ours to fix. vcpkg cannot build `libadwaita` for an MSVC triplet at all:
libadwaita requires `appstream`, appstream requires `libxmlb`, and vcpkg's
`libxmlb` port declares `"supports": "!windows | mingw"`, so vcpkg refuses the
whole graph while still planning. That is not a vcpkg invention either —
upstream libadwaita's `meson.build` calls `dependency('appstream')` with no
`required: false`, so there is no switch to turn it off.

Looking into it turned up the better reason to stop. gtk.org's own Windows page
names only MSYS2 and gvsbuild and never mentions vcpkg, and a gtk-rs maintainer
states it plainly in gtk-rs/gtk4-rs#1963: *"vcpkg's GTK is not very useful and
misses various files. It's not recommended to be used by the GTK project."*
A vcpkg `gtk` port exists and is kept version-current, which makes this
impossible to see from the catalogue — worth remembering before trusting a
vcpkg port in a stack that is Linux-first.

`--allow-unsupported` would have forced the `libxmlb` refusal into an attempt,
and was not tried. It answers the wrong question: even a successful build lands
on a GTK its own project disowns.

**Plan B — each project's blessed way.** GTK stack through **gvsbuild** (the
gtk-rs book's recommended Windows route, so the exact combination the gtk-rs
developers test), CBC through **coinbrew** (COIN-OR's own build tool; on
Windows it runs from an MSYS2 or Cygwin shell with `cl` on PATH and
`--enable-msvc`, so MSYS2 appears here purely as a build shell while the
output stays MSVC), Python from vcpkg, Rust msvc. Alternative for the
CBC piece if coinbrew fights us: vcpkg's `coin-or-cbc` port, as in plan A.
More moving parts than plan A (several build systems), but maximum upstream
blessing per component. Pin the gvsbuild version (and the coinbrew/CBC
versions) in the build script so a rebuild next year uses the same recipe;
bumping a pin is a deliberate act, like bumping the flatpak runtime.

Only the gvsbuild half of this is taken. coinbrew is not: vcpkg's CBC built
cleanly on the first working run, so the second build system would buy nothing.
gvsbuild builds with MSVC — that is its whole purpose — and currently ships
libadwaita 1.9.2, past the 1.7 the crates ask for. It has no `appstream` or
`libxmlb` project of its own, so meson resolves those as source subprojects,
which is presumably why the chain that stops vcpkg does not stop it.

The cost to count honestly: gvsbuild needs MSYS2 as a build shell and a Python
to run itself, so the five-tool list in step 2 grows. Both are genuinely tools
rather than dependencies, so the rule that everything C/C++ comes from vcpkg is
not what bends here — the tool count is.

**Plan C — MSYS2.** Everything prebuilt as mingw binaries via pacman (gtk4,
libadwaita, coin-or-cbc, Python), Rust windows-gnu, one `pacman -S` line and
the whole stack exists. Fastest path to a running app by far, and the fallback
of last resort. Caveat: the bundled Python would be mingw Python, so teachers
lose PyPI binary wheels — or we attempt the hybrid (mingw-built app linking
the official `python3XX.dll` through the C ABI), which is the least-documented
configuration of the three.

Installer, independent of the ladder: **Inno Setup 7**. It produces exactly
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

**Done**: `f9e66ce9` (item 1), `74ce9e67` (item 2), `baeb64af` (item 4), plus
`a05d46b8` and `0f27f08a`, which the work ran into on the way. The workspace
suite passes and the flatpak still builds from a cold cache.

Two items did not go as written. Item 2 **deletes** the `stdc++` line instead of
gating it: `cc` already emits the C++ standard library for a `.cpp(true)` build —
the recorded build metadata carried the flag twice — and it picks the right one
per target, `stdc++` for gnu, `c++` for darwin, none at all for MSVC. Naming the
library ourselves, or gating that name on "not MSVC", is guessing in `cc`'s place
for toolchains that do not exist yet. Item 3 found nothing to fix and produced no
commit: the only two `#[cfg(unix)]` sites are the SIGHUP reset and the termios
call on the pty master, both correctly unix-only and both already answered on the
Windows side by the Job Object; there is no `std::os::unix` import, no hardcoded
unix path and no `HOME` read anywhere in the workspace, and the `sleep`-spawning
tests in `subprocesses` are already `#[cfg(all(test, unix))]`.

The two extra commits: `a05d46b8` because the pre-commit hook refused item 1. It
demanded `cargo-sources.json` be staged with any `Cargo.lock` change, but that
file follows the crates.io package set only, and dropping three unused
dependencies changes no package — so there was nothing to stage. It now compares
the two files' checksum sets instead. (`collomatique.nix` keeps the old rule: its
cargoHash covers the whole lock, and it did change.) `0f27f08a` fixes a storage
test that had been red since the version bump in `a66e6b9d`, unrelated to any of
this but in the way of item 5.

### Step 2 — VM bootstrap

`pkgs/windows/bootstrap.ps1`, run elevated in the Windows 11 VM. It started at
five tools — VS Build Tools 2022, git, vcpkg, rustup, Inno Setup 7 — and step 3
grew it to eight: Python 3.12, uv and MSYS2, all three there only so that
gvsbuild can run. Written as a script rather than a list of manual installs
because the VM is disposable — the
point is to roll back to a clean snapshot, run one command, and be back at a
build environment. Take a snapshot afterwards: that snapshot, not the script, is
the reproducible artifact.

**Done**: `d935b4c0`, then `2b7ff721`, `f1f81dd7`, `bb2bd6a9` and `624420a3`,
each fixing something the previous run surfaced, then `645aa5df`, `ec547ee6`,
`b4aecd0d` and `e5545cba` for the gvsbuild tools. Everything installs and is
found. The last full run was against a machine that already had Build Tools; a
run from a blank snapshot is still owed.

gvsbuild itself is installed here rather than in `build.ps1`, because it is a
tool and this is where tools go. It is pinned, and installed the way its own
documentation does it: `uv tool install gvsbuild==<version>`, followed by
`uv tool update-shell`. That second line is not optional — uv puts tool
executables in `%USERPROFILE%\.local\bin`, which Windows does not have on PATH,
and until it has run `uv run gvsbuild` reports gvsbuild as not found while
`uv tool list` cheerfully shows it installed.

The script also fetches one URL with `curl.exe` and throws the result away. The
Windows certificate store is filled in lazily: a root certificate only arrives
once something has asked for it. gvsbuild downloads its own cargo from
`win.rustup.rs`, and on a fresh machine that download fails with an SSL error,
because CPython reads the store but does not trigger the update the way a
schannel client does. Fetching the URL once with `curl.exe` fills the gap. A
browser is not a valid test of this — Edge and Chrome carry their own root
stores, so succeeding there says nothing about the Windows one.

Three things the VM taught, all recorded in the script:

- winget's `--override` **replaces** the switches it would otherwise pass to an
  installer. The Build Tools line in the Rust documentation contains only
  `--add` components, so the Visual Studio bootstrapper ran interactively and,
  with no `--wait`, returned as soon as it had handed off — winget reported
  success while the install was still going. `--passive --wait --norestart` at
  the front of the override fixes both.
- Piping winget into PowerShell (`| Out-Host`) hangs the script after a Build
  Tools install. A pipeline ends when every handle on its input closes, not when
  the writing process exits, and the Visual Studio installer leaves processes
  behind holding that handle. winget is called bare now, which also gives back
  its real progress display.
- winget installs Inno Setup per-user by default, under `%LOCALAPPDATA%`.
  `--scope machine` puts it in `Program Files`, where a build tool belongs.

Python is deliberately not on the list. See the next step.

### Step 3 — `build.ps1`, first part: the C and C++ dependencies

The build script is grown in pieces, one per step, rather than written at the
end: dependencies here, the cargo build in step 4, bundling in step 7, the
installer in step 8. Each piece is exercised on its own before the next is
added.

There is no hello-world spike. The earlier plan proved the GTK wiring on a
throwaway project first; that is a second build to set up and keep working, and
the workspace build reaches the same answer.

**Done for the vcpkg half**: `98d18c01`, then `8f87c147` and `910f0e8a`, each
fixing what the previous run surfaced. `pkgs/windows/vcpkg.json` names
`coin-or-cbc`, `python3` and `pkgconf` with a `builtin-baseline`;
`pkgs/windows/triplets/x64-collomatique.cmake` is the triplet;
`pkgs/windows/build.ps1` runs `vcpkg install` and reports what landed.

The baseline is the version pin for the whole C/C++ side, including Python.
vcpkg resolves it by checking port files out of git history by tree hash, so the
`ports/` directory on disk is never consulted and a stale clone fails loudly
rather than building something else. Per-port `overrides` exist if the baseline
alone ever proves too coarse; it has not yet.

**One triplet, mixed linkage — it works.** The open question was how a static CBC
coexists with everything else dynamic. The answer is per-port customisation
inside one triplet file, which is a documented vcpkg feature: `VCPKG_BUILD_TYPE
release` and dynamic linkage throughout, with `VCPKG_LIBRARY_LINKAGE static` for
the COIN-OR ports and the linear algebra under them. The result is
`Cbc/Cgl/Clp/Osi/CoinUtils/lapack/openblas` as `.lib` with no matching DLLs,
beside a dynamic `python312.dll` and its import library, all in one installed
tree. The two-triplet fallback was not needed.

`lapack` has to be in that static list, for a reason that has nothing to do with
symbols. vcpkg's `lapack` is a metapackage that picks `clapack` on
`(static & windows & !mingw)` and `lapack-reference` otherwise, where `static`
reads `VCPKG_LIBRARY_LINKAGE`. Left dynamic it chose `lapack-reference`, which
pulls in `vcpkg-gfortran` and fails to build (microsoft/vcpkg#49688, open and
stale). Static, it chooses `clapack`, whose chain is `blas` then `openblas`,
neither needing Fortran. That is vcpkg's own escape hatch, not a workaround.

**There is no `CbcSolver.lib`, and that is fine.** vcpkg folds libCbcSolver into
`Cbc.lib`, and `cbc.pc` correctly does not name a separate one. `Cbc.lib`
contains both `cbcPreProcessPointer` and `CbcMain1`, which is what
`collo_cbc.cpp` needs — it includes `<CbcSolver.hpp>` and calls
`CbcMain0`/`CbcMain1`/`CbcSolverUsefulData`. Checking this needs no developer
prompt: COFF archives store symbol names as ASCII, so `findstr /m /c:` finds
them.

**`cbc.pc` carries the whole static chain in `Libs:`**, not in `Libs.private:`
— `-lCbc -lCgl -lOsiClp -lClp -lOsi -lCoinUtils -lbz2 -lz -llapack -llibf2c
-lopenblas`. So `collo-cbc/build.rs` gets the complete link line without asking
pkg-config for static metadata. Those are Unix-style `-l` names against a
library set that MSVC names differently (`-lz` versus `zlib.lib`); whether they
all resolve is a step 4 problem, noted here so it is not a surprise there.

**vcpkg's Python is shippable.** This was the other open question and the answer
is yes, with one step. `pip` is not installed, but `ensurepip` is, carrying a
bundled pip 25.0.1; after `python -m ensurepip --upgrade`, `pip install --user
xlsxwriter` downloaded from PyPI and installed cleanly, and `ssl`, `sqlite3`,
`zlib` and `ctypes` all import. So the bundle step runs `ensurepip` once rather
than shipping a second interpreter. See step 7 for where `--user` puts things.

**Done for the GTK half too**: `4b764ce5`, then `bcd2d020` and `a6f9a98a`.
`build.ps1` runs `uv run gvsbuild build` for `gtk4`, `libadwaita` and
`adwaita-icon-theme`, and everything else in the stack arrives as a dependency of
those three. The install prefix is not a choice: gvsbuild builds it as
`<build-dir>\gtk\<platform>\<configuration>`, so passing `--configuration
release` explicitly is what makes the path the script reports afterwards a fact
rather than a guess.

`adwaita-icon-theme` is asked for by name because it is not a library and does
not arrive as a dependency, and a libadwaita application without it simply shows
no icons. `librsvg` is *not* asked for and does not need to be — gvsbuild makes
it a dependency of `gtk4` itself, which settles the symbolic-icon question before
it was raised. It is also why a run downloads rustup: librsvg is written in Rust,
and gvsbuild installs its own pinned cargo rather than using the rustup that
`bootstrap.ps1` put there.

**Both halves are accepted.** A full run of `build.ps1` finished on 19 Aug 2026
and its closing report found `cbc.pc` in the vcpkg prefix, `gtk4.pc` and
`libadwaita-1.pc` in the gvsbuild one, `glib-compile-resources` 2.88.3, `pkgconf`
3.0.3, and Python 3.12.13 with a 163-module stdlib. `libadwaita-1.pc` existing at
all is the whole point of the detour — it is the file vcpkg could not produce.

Two things a fresh machine will hit again, only one of them fixed:

- **The certificate store needs warming.** Reproducible, and now done by
  `bootstrap.ps1`; see step 2.
- **gvsbuild pins gperf 3.1 to a single hard-coded mirror**,
  `mirrors.ibiblio.org`, in both the pinned version and its `main` branch. That
  server was serving nothing at all during our build, so the download 404s.
  Nothing to report upstream — it is an outage, not a dead URL. The way out is to
  fetch the tarball by hand from `https://ftp.gnu.org/pub/gnu/gperf/` into
  `<build-dir>\src\` and re-run; gvsbuild checks the hash, so a wrong file is
  caught rather than built. Deliberately not automated: a recovery path in the
  build script for a mirror that is usually up is more code to maintain than it
  is worth.

### Step 4 — `build.ps1`, second part: first compile of the workspace

`cargo build --release -p collomatique-gtk4`, driven by the script.

The `-sys` crates find GTK through `system-deps`/pkg-config, so the script
points `PKG_CONFIG` at vcpkg's pkgconf and `PKG_CONFIG_PATH` at **two**
directories — vcpkg's `<install-root>\<triplet>\lib\pkgconfig` for CBC, and
gvsbuild's own prefix for the GTK stack. `PYO3_PYTHON` goes at vcpkg's
interpreter, the same one that will be shipped, which is the whole reason Python
comes from vcpkg. Fix whatever the compiler surfaces (expected to be little,
after step 1).

The CBC link line needs no `--static` handling: step 3 established that
`cbc.pc` already carries the full chain in plain `Libs:`. What to watch instead
is whether its Unix-style `-l` names resolve to the filenames MSVC expects —
`-lz` against `zlib.lib` is the obvious one.

The toolchain gate that used to live here is gone: step 3 answered it, earlier
and more clearly than a compile would have. gvsbuild for GTK, vcpkg for the
rest.

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

Python bundling: take the interpreter and stdlib out of the vcpkg tree, the
same build the exe was linked against, so there is no version to detect and
nothing to match. Run `python -m ensurepip --upgrade` once while staging, since
the vcpkg port carries `ensurepip` but no installed `pip`. Ship `xlsxwriter`
with it, version-pinned like the flatpak.

Where a teacher's own packages land is a real decision, and step 3 measured the
default: `pip install --user` puts them in
`%APPDATA%\Python\Python312\site-packages`. That is outside the application, so
it already survives an update — but it is the machine-wide user site, shared
with any other Python 3.12 on the machine, and it is keyed to the minor version,
so bumping the baseline to a Python 3.13 would silently orphan everything a
teacher installed.

**The inclination is to set `PYTHONUSERBASE` into Collomatique's own private
data directory instead**, which is what the flatpak does. Not yet decided, and
tangled with a separate idea the user is weighing — adding a Python command line
to the application itself — which is deferred. Decide both together.

How the interpreter's own paths are arranged (a `python3XX._pth`, `PYTHONHOME`,
or nothing at all) is still open and belongs to this step.

Acceptance for this step: the staged tree runs on a Windows machine without
vcpkg, Build Tools or Python installed.

### Step 8 — Installer, and `build.ps1` complete

Home: `pkgs/windows/`, next to `pkgs/flatpak/` (the migration happened in
step 1), so all OS packaging lives under `pkgs/`.

- `build.ps1` — by this point it already does steps 3, 4 and 7. What is added
  here: read the version from the workspace `Cargo.toml` (same trick as
  `pkgs/flatpak/build.sh`) and run `ISCC.exe` on the `.iss` script to produce
  `Collomatique-Setup-<version>.exe`. The whole build is then one command.
  `ISCC.exe` is not on PATH; `bootstrap.ps1` reports where it is.
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
- CBC **and Python**: the vcpkg manifest's `builtin-baseline`. This is the pin
  that matters most, since these are the parts that actually break across
  versions. Python's *minor* version deserves treating as a pin in its own
  right, beyond the baseline: teachers' installed packages live in a directory
  named after it (step 7), so a 3.12 to 3.13 move is not a silent bump.
- The GTK stack: the gvsbuild version, pinned in `bootstrap.ps1` (which is where
  gvsbuild is installed), on the same terms — bumping it is a deliberate act,
  like bumping the flatpak runtime. Note this pins the recipe, not the
  ingredients: which GTK version a given gvsbuild builds is decided inside
  gvsbuild, so reading the pin does not tell you the GTK version.
- Bundled xlsxwriter: version-pinned, same as the flatpak.
- The Windows SDK: pinned by the component name in `bootstrap.ps1`
  (`Windows11SDK.22621`).
- Rust itself: **deliberately not pinned**, tracking stable, the same policy as
  the flatpak's `rust-stable`. `Cargo.lock` already pins every dependency, and
  pinning only on Windows would create a skew between the platforms instead of
  removing one. Pin reactively if a release ever breaks the build. Note that a
  `rust-toolchain.toml` would not do it: rustup honours that file, the flatpak's
  SDK extension has no rustup and would ignore it in silence.
- MSVC and the Visual Studio installer: not pinned. Its installer always fetches
  current and the ABI is stable.

## Out of scope for now

64-bit x86_64 only (no ARM64/32-bit); no code signing; no CI for the Windows
build (manual VM builds); distribution channels.
