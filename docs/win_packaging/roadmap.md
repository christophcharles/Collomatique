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
`zlib` and `ctypes` all import. So one interpreter is enough; who runs
`ensurepip`, and when, is a step 7/8 question. See step 7 for where `--user`
puts things.

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

**Done**: `d8c34b19` added the build to the script, and then compiling it for
real took three more commits — `08d38dce`, `df468047` and `a9be6378`.

The expectation written above was the wrong way round. The Rust code needed
nothing: not one source file changed, and the only thing the compiler asked for
was a Cargo feature (`a9be6378` — `windows-sys` emits `CreateJobObjectW` only
when `Win32_Security` is on, because its signature names a type from there).
Everything else was the dependencies.

**CBC no longer comes from vcpkg at all** (`08d38dce`), which is the one real
change to step 3's outcome. vcpkg's `cbc.pc` reports `Version: devel`, so
`collo-cbc/build.rs`, which asks pkg-config for CBC >= 2.10, can never be
satisfied by it — and that is not a packaging slip to work around: vcpkg's
COIN-OR ports pin raw master commits rather than releases, none of them tagged
and none contemporaries. CBC is now COIN-OR's own release binaries, pinned to
2.10.13 by URL and SHA256, `-md` for the dynamic CRT that matches rustc. They
are static `.lib` files, which is what `cbcPreProcessPointer` needs anyway,
since MSVC will not auto-import a data symbol from a DLL.

The `-l` mismatch this step said to watch did happen (`df468047`), just not on
`-lz`: the archives carry autotools names like `libCbcSolver.lib`, `cbc.pc`
asks for `-lCbcSolver`, and rustc turns that into the literal filename
`CbcSolver.lib`. Nothing in between translates. The script now copies each
`lib*.lib` to its MSVC name after unpacking — the same thing vcpkg's own tree
does.

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

**Done.** Items 1, 3 and 4 are confirmed on the VM as written: a solve streams
mid-solve incumbents, so `cbcPreProcessPointer` works through the static
COIN-OR archives; the embedded interpreter starts and finds its stdlib, and a
script using xlsxwriter runs once xlsxwriter is installed into that interpreter;
and rfd opens native Win32 dialogs. Item 2 was rebuilt rather than verified and
item 5 found one real problem, both below.

Item 3's condition is worth spelling out, because step 7 depends on it: it was
tested against an interpreter that had been through `ensurepip` and a pinned
`pip install xlsxwriter`. So the requirement is real and measured. What is open
is only who performs that install, now that the staging step does not.

**Item 2 was not verified, it was rebuilt.** ConPTY is gone on Windows;
`3266731d`, `ddf4da59`, `0eb05c5f` and `b05b06b8` are the story, and the
teardown note at the top of `subprocesses/src/process.rs` is where the shape
now lives. Briefly:

The in-band RPC could not survive a terminal emulator. It travelled as marked,
80-byte-chunked lines on the same stream as the logs, and the chunking and the
markers existed only because a tty wraps and transforms text. On unix
`cfmakeraw` turns all of that off; ConPTY has no such switch — it reflows at
the console width and injects escape sequences. So the RPC moved off the
stream entirely, onto its own private byte-clean local socket: a unix domain
socket on Linux, a named pipe on Windows, one design on both platforms, with
the name handed to the child in `COLLOMATIQUE_RPC_CHANNEL`. The whole
marker-and-chunking layer is deleted.

Then ConPTY turned out not to work at all, for a reason worth recording:
conhost opens by asking the terminal where the cursor is (`ESC[6n`, a Device
Status Report) and waits for the answer. Our consumer is a log reader, not a
terminal emulator, so it never answers and the pseudoconsole stalls — even
`cmd.exe /c echo bonjour` hangs, and the startup preamble only appears when the
pty is torn down. With the RPC on its own channel and teardown already the Job
Object's job, the pty was carrying nothing on Windows, so the worker there now
spawns on plain pipes.

That costs the one thing a terminal was giving for free: a C runtime behind a
pipe block-buffers, so a log meant to be watched arrives in lumps or not until
the end. Two answers, both narrow. CBC calls `setvbuf(_IONBF)` on stdout at
startup (`collo_cbc_unbuffer_output`; MSVC has no line buffering to ask for —
`_IOLBF` there behaves as `_IOFBF`). Python gets `PYTHONUNBUFFERED=1` in the
worker's environment, which `Py_Initialize` reads, so an embedded interpreter
honours it too.

Linux is untouched by all this: it still spawns on a pty, because there the pty
is load-bearing — closing the master hangs the child up, and that SIGHUP
cascade is how a subtree dies with its parent.

**Item 5 found one real problem: fractional display scaling.** The VM runs at
Windows' 150% setting, and there the text grows while the widgets do not. The
window comes out cramped, with buttons too small for the labels inside them.
Nothing else under item 5 came back as a problem.

This is not a packaging fault, and not something to fix in our code. GTK has two
scaling knobs and on Win32 they disagree. `scale-factor` is an **integer**, and
it is what scales widgets, spacing and icons; the Win32 backend can only report
1 or 2, so at 150% it reports 1 and the whole layout is built at 100%. The font
DPI is a separate setting, it does follow the system, and at 150% that is 144
dpi. Text at 1.5 inside a layout at 1.0 is exactly the symptom.

Upstream this is [GNOME/gtk#1036, "Support fractional scaling on
windows"](https://gitlab.gnome.org/GNOME/gtk/-/issues/1036) — open since GTK
3.93 and never closed. Wayland gained fractional scaling in GTK 4.14; the Win32
backend did not. The two environment variables do not add up to a fix either:
`GDK_SCALE` takes integers only and moves the UI alone, `GDK_DPI_SCALE` moves
text alone, and `GDK_SCALE=2` at a 150% display is simply too big.

The way out is to stop GTK from seeing the real DPI, and let Windows scale the
finished window as a bitmap. Confirmed by hand on the VM, with no build:
Properties → Compatibility → *override high DPI scaling behaviour* → **System**.
Everything is proportionate again. It is slightly soft at 150% and exact at
200%, where the stretch is a whole number.

**So the permanent form is an application manifest declaring the process
DPI-unaware**, which is a step 6 item — it belongs in the same resource as the
icon. A manifest works where an API call would not: manifest awareness is fixed
before any code runs, which is what lets it win over the
`SetProcessDpiAwarenessContext` call GTK makes for itself.

### Step 6 — Windows polish (small code changes)

- `#![cfg_attr(windows, windows_subsystem = "windows")]` on
  `gtk4/src/main.rs` so no console window flashes. The `--rpc-engine` child
  must keep working — its stdio is a set of pipes the parent creates, which
  needs no console of its own — verify it after the change.
- Embed the app icon and version info into the exe (taskbar/Explorer icon),
  e.g. with the `winresource` crate in `gtk4/build.rs` under `cfg(windows)`,
  from an `.ico` generated once out of
  `resources/icons/collomatique-{128,256,512}.png` and committed (same
  reasoning as the pre-scaled flatpak PNGs).
- **An application manifest declaring the process DPI-unaware**, in that same
  resource. This is the fractional-scaling answer from step 5: Windows then
  scales the finished window itself, instead of GTK scaling the text and not
  the widgets. Already confirmed by hand through the Compatibility tab, so what
  is left is only to make it permanent.
- Python path setup only if step 5.3 required it (see step 7 for the layout
  that should make it unnecessary).

**Done for the console half**: `add53b8c`, then `e20a8c39`, `9bc6d175`,
`06e2215b` and `254a2a22`. The window no longer opens, and — the reason it was
annoying — no longer takes the focus the GUI should have had.

**There is no command line on Windows, and that is a decision rather than a
gap.** `--help`, `--version` and `--debug` produce no terminal output there. It
is worth writing down why, because the obvious objection ("just attach a
console") was tried and abandoned:

Windows decides console-or-GUI from a single flag in the executable, chosen at
build time. A program cannot be both. `AttachConsole(ATTACH_PARENT_PROCESS)`
plus `SetStdHandle` did bring back everything **Rust** prints, because
`std::io` re-reads the standard handles on every write. It never brought back
CBC's log, because the C runtime builds its own descriptor table once at
startup, from handles that were still null at that point, and nothing done
afterwards revives it. Repairing that too — `_open_osfhandle` and `_dup2` onto
descriptors 1 and 2 — still did not produce a usable command line, and shell
redirection stayed unpredictable: PowerShell does not hand a
`windows`-subsystem program a real handle at all, so `>` captured nothing.

What is kept is only the part that stops printing from being fatal
(`gtk4/src/windows_stdio.rs`). A null standard handle is not a silent sink on
Windows: `std::io` reports `ERROR_INVALID_HANDLE`, `println!` panics on a failed
write, and with no console that panic is invisible too — the application would
simply vanish. So stdout and stderr are pointed at `NUL`. The guard that leaves
an **already set** handle alone matters more than the rest of the file: the
`--rpc-engine` worker is this same executable, and its whole log travels on a
pipe the parent hands it at creation time.

Silence is not the same as doing nothing, so terminal-only arguments are
answered in a message box instead (`254a2a22`, `gtk4/src/windows_cli.rs`),
showing clap's own text for help, version and usage errors, and a sentence of
its own for `--debug`. Unix keeps its command line exactly as it was.

**Done for the icon** (`2239ad51`), with one detail different from the bullet
above. `resources/icons/generate-sizes.sh` grew a second job: it writes
`collomatique.ico` next to the scaled PNGs, from the same 1024×1024 master
rather than from those copies — one downscale instead of two. The sizes are
Pillow's seven defaults, 16 through 256, stored PNG-compressed inside the
`.ico`; Windows has read that form since Vista, and if some corner of the shell
ever draws one of them wrong, `bitmap_format="bmp"` is the one keyword to change.
It is generated on Linux and committed for the same reason the scaled PNGs are,
one step further: the Windows machine has no image tooling on it at all. The
pre-commit hook lists it beside them, so it cannot go stale on its own.

`gtk4/build.rs` compiles it into the executable with `winresource` — dependency
and code both under `cfg(windows)` — and sets `ProductName` and
`FileDescription` while it is there. Without those, the file's properties and
the row Task Manager shows read `collomatique-gtk4` and an empty description.

Everything else inherits that one resource rather than naming the icon again:
the Start-menu shortcut points at the executable, and the file type's
`DefaultIcon` is `...exe,0`. The only other place that needs the `.ico` itself
is Inno Setup's `SetupIconFile`, because setup.exe runs before anything is
installed. Version info comes along with the same resource, so that half of the
bullet is done too.

**Done for the DPI manifest** (`e81998f7`), in that same resource as planned.
`gtk4/collomatique-gtk4.exe.manifest` declares the process DPI-unaware, and
`build.rs` compiles it in beside the icon — one `set_manifest_file` line, one
more `rerun-if-changed`. GTK is then told 96 dpi and lays the window out at
100%, and Windows stretches the finished result: the Compatibility-tab override
from step 5, made permanent.

The declaration also **locks** the setting, and that is the load-bearing half.
GTK calls `SetProcessDpiAwarenessContext` for itself at startup; with the
manifest that call fails with `ERROR_ACCESS_DENIED`, and
`_gdk_win32_enable_hidpi` reads back what was set instead of insisting. Plain
absence of these elements is unaware too, but a *changeable* unaware, and GTK's
call would win. Two elements say the same thing to two generations of Windows:
`dpiAwareness=unaware` for Windows 10 1607 and later, which ignores the second,
and `dpiAware=false` for anything older.

Nothing else is declared, on purpose. UTF-8 as the process code page, long path
support and a supported-OS list are all real manifest settings with real
effects, and none of them is about scaling. `asInvoker` is there because a
manifest should say it; it is what the application already did.

There was no manifest to collide with: rustc puts one in its own binary, not in
the programs it compiles.

**The first one did not start** (`98d83c32` fixes it), and the failure is worth
keeping because nothing about it points at the cause. The installed
Collomatique answered a double-click with a box: *CreateProcess a échoué ; code
14001. L'application n'a pas pu démarrer car sa configuration côte-à-côte est
incorrecte.* No file is named, and the build had been silent.

The manifest's comments contained `--`, which an XML comment may not, so the
file did not parse. That is not a warning: Windows builds the activation
context before the process starts, so a manifest it cannot read means the
program never runs at all. The elements were right the whole time; only the
prose around them was wrong. This is a trap of our own making, since `--` is
the dash the rest of the tree writes freely, and the file now carries a note
saying so.

The pre-commit hook parses the **staged** manifest with `python3` and refuses
the commit if it does not read, which is what would have caught this before it
reached the VM. `xml.etree` is in the standard library, so this asks nothing new
of the shell. Every other file here can be wrong in a way that shows up as a
test failure or a compile error; this one can only be found by running the
program on Windows, which is exactly the slowest place to find anything.

What is left in this step is the Python path setup, if it turns out to be
needed.

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
nothing to match. The vcpkg port carries `ensurepip` but no installed `pip`, so
something has to run `python -m ensurepip --upgrade` once before a teacher can
add a package — and `xlsxwriter` has to arrive the same way, version-pinned like
the flatpak.

Not while staging, though. `build.ps1` used to do both and no longer does: the
staging directory is rebuilt from nothing on every run, so paying for a download
and an install there buys nothing that survives. It belongs to whatever ships
the application — the installer, most likely (step 8) — and which of the two it
is has not been settled.

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

**Done** (`dfefa936`, then `0711bd97` and `2239ad51`), with one deliberate
change from the plan above.

`build.ps1` reads the version out of the workspace `Cargo.toml` the way
`pkgs/flatpak/build.sh` does, finds `ISCC.exe` in the two machine-wide places
`bootstrap.ps1` reports, and passes every path and version into the `.iss` with
`/D`. The `.iss` refuses to compile without them, so it has nowhere to go stale.
`build.ps1` is now the whole build in one command. It also checks the committed
`.ico` exists at startup rather than hours later, because `gtk4/build.rs` fails
without it.

**The privilege default came out the other way round.** The plan above was
`lowest` with an all-users option; the first build shipped exactly that, and the
install landed in `%LOCALAPPDATA%\Programs\Collomatique` — inside a hidden
folder, which will trip a teacher up the first time they go looking for it. The
awkward part is that there is no way out by choosing a better directory: every
visible location needs administrator rights. So the choice is about UAC, not
about the path.

What ships is `PrivilegesRequired=admin` plus
`PrivilegesRequiredOverridesAllowed=dialog`: Setup opens on a page asking for all
users or this user only, *before* it elevates. All users is the default answer,
so the common case is one extra click and `C:\Program Files\Collomatique`; the
other answer keeps the old per-user behaviour and is what still installs where a
teacher has no administrator rights. An upgrade does not ask again —
`UsePreviousPrivileges` defaults to yes and finds the mode the first install
used. Every registry entry is written under `HKA` to match, so the association
follows the same fork.

Two smaller things worth knowing. `ArchitecturesInstallIn64BitMode` is
load-bearing rather than merely correct: without it Setup runs 32-bit and the
all-users path becomes `Program Files (x86)`. And `ArchitecturesAllowed` is
`x64compatible` rather than `x64`, which lets the x86_64 build run under Windows
on ARM's emulation — that is not the ARM64 port listed as out of scope, only a
refusal we had no reason to make.

The association is five entries: the extension naming the ProgID, the same
ProgID again under `OpenWithProgids` so Collomatique is offered in "Open with"
even where a previous choice is recorded, then the ProgID's type name
("Colloscope Collomatique"), its `DefaultIcon` and its `shell\open\command`.
Uninstalling removes the ProgID subtree, and removes the extension key only if
nothing else is left in it.

What has actually been run: the first installer, `dfefa936`, built and installed
on the VM. The mode dialog, the icon and the association came after it and have
not been. The clap check above is the one a double-click exercises, so it is the
thing to try first — from a folder whose name contains a space.

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
- Bundled xlsxwriter: version-pinned, same as the flatpak. No pin in the tree
  yet — it goes wherever the install of it ends up living (step 7).
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
