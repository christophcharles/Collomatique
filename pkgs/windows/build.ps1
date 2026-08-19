# Build Collomatique for Windows.
#
#     powershell -ExecutionPolicy Bypass -File pkgs\windows\build.ps1
#
# Run it from a terminal opened after pkgs\windows\bootstrap.ps1 has finished,
# so that VCPKG_ROOT is in the environment.
#
# What it does today is the dependencies, then Collomatique. The dependencies
# come from three places, and each one is the place its own upstream blesses:
# CBC from COIN-OR's own release binaries, GTK and libadwaita built by gvsbuild,
# and the Python the application embeds built by vcpkg from the manifest next to
# this file. It reports what landed, builds the application against all three,
# then stages a directory that runs on a machine with no build tools on it. The
# installer is what remains.
#
# It is three rather than one because vcpkg cannot serve either of the first two.
# For GTK: libadwaita needs appstream, appstream needs libxmlb, and vcpkg's
# libxmlb port declares "supports": "!windows | mingw", so vcpkg refuses it for
# MSVC before compiling anything -- and gtk.org names MSYS2 and gvsbuild for
# Windows, never vcpkg. For CBC: vcpkg's coin-or-* ports pin raw master commits
# instead of releases, which is explained where CBC is fetched below. Python and
# pkgconf are what vcpkg is left doing, and it does them well.
#
# Expect most of a day on a first run, and a lot of downloading: two of the three
# build everything from source. Interrupting is safe and re-running resumes --
# vcpkg keeps every package it has already built, gvsbuild is asked to skip
# projects it has already built, and CBC is skipped once it is unpacked.
#
# Nothing is written inside the repository. Everything goes under -OutRoot, which
# is also where the staged application and the installer will be put later.

#Requires -Version 5.1

param(
    # Outside the working tree, like pkgs/flatpak/build.sh does with its own
    # output, and everything this script produces goes under here.
    #
    # Short, and at the root of C:, because both halves of the build nest deeply
    # against a 260-character path limit -- vcpkg's trees and gvsbuild's alike.
    # "collo-build" rather than "collomatique-build" for the seven characters:
    # every source tree underneath inherits them.
    [string]$OutRoot = 'C:\collo-build',

    # See triplets\x64-collomatique.cmake for what this one changes.
    [string]$Triplet = 'x64-collomatique'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Native commands below are checked through $LASTEXITCODE, which does not exist
# until one has run. Strict mode makes reading it before then an error.
$global:LASTEXITCODE = 0

function Write-Step { param([string]$Message) Write-Host "==> $Message" }
function Write-Note { param([string]$Message) Write-Host "    $Message" }
function Write-Fail { param([string]$Message) Write-Host "error: $Message" }

# ---------------------------------------------------------------------------
# Where everything is
# ---------------------------------------------------------------------------

$ManifestDir   = $PSScriptRoot
$TripletDir    = Join-Path $ManifestDir 'triplets'
$InstalledRoot = Join-Path $OutRoot 'vcpkg_installed'
$Prefix        = Join-Path $InstalledRoot $Triplet

# This script lives in pkgs\windows\, so the workspace is two levels up.
$RepoRoot = (Resolve-Path (Join-Path $ManifestDir '..\..')).Path

# cargo would put this in <repo>\target. It goes under -OutRoot instead, for the
# same reason everything else does: the working tree stays clean, and everything
# the build produces sits in one place for the bundle step to harvest from.
$CargoTarget = Join-Path $OutRoot 'target'

# What gvsbuild gets as its --build-dir, and where it installs to. gvsbuild's own
# default is C:\gtk-build; putting it here instead keeps everything this script
# produces under one root.
#
# The prefix below is not a choice: gvsbuild builds the path as
# <build-dir>\gtk\<platform>\<configuration>, and those last two segments are the
# ones passed on its command line further down.
$GtkBuildRoot = Join-Path $OutRoot 'gtk-build'
$GtkPrefix    = Join-Path $GtkBuildRoot 'gtk\x64\release'

# CBC is downloaded rather than built, so its version is a pin like any other and
# lives here with the paths. See the section that fetches it for why it does not
# come from vcpkg.
$CbcVersion = '2.10.13'
$CbcUrl     = "https://github.com/coin-or/Cbc/releases/download/releases/$CbcVersion/Cbc-releases.$CbcVersion-w64-msvc17-md.zip"
$CbcSha256  = 'b9702ad7501b4249a9721984ce3c6dc8fb9b6cfb995f42d493a5bd54b8f42a74'
$CbcPrefix  = Join-Path $OutRoot "cbc-$CbcVersion"

Write-Step "Collomatique Windows build"
Write-Note "manifest: $ManifestDir"
Write-Note "triplet:  $Triplet"
Write-Note "output:   $OutRoot"
Write-Note "gtk:      $GtkBuildRoot"
Write-Note "cbc:      $CbcPrefix"
Write-Host

# bootstrap.ps1 sets VCPKG_ROOT for the user account, so a terminal opened
# before it ran will not have it. Fall back to the path bootstrap.ps1 clones
# into rather than fail on a variable that is only missing from this session.
$VcpkgRootEnv = [Environment]::GetEnvironmentVariable('VCPKG_ROOT')
$VcpkgRoot    = if ($VcpkgRootEnv) { $VcpkgRootEnv } else { 'C:\vcpkg' }
$VcpkgExe     = Join-Path $VcpkgRoot 'vcpkg.exe'

if (-not (Test-Path $VcpkgExe)) {
    Write-Fail "vcpkg is not at $VcpkgExe."
    if (-not $VcpkgRootEnv) {
        Write-Note "VCPKG_ROOT is not set in this session. If bootstrap.ps1 has already"
        Write-Note "run, open a new terminal and try again -- environment variables are"
        Write-Note "only picked up by processes started afterwards."
    }
    Write-Note "otherwise run pkgs\windows\bootstrap.ps1 first."
    exit 1
}

if (-not (Test-Path $OutRoot)) {
    try {
        $null = New-Item -ItemType Directory -Path $OutRoot
    } catch {
        Write-Fail "could not create $OutRoot."
        Write-Note "Windows reserves the root of C: for administrators. Run this elevated,"
        Write-Note "or pass a path you own: -OutRoot C:\Users\you\collo-build"
        exit 1
    }
}

# ---------------------------------------------------------------------------
# CBC, from COIN-OR's own release
# ---------------------------------------------------------------------------

# This used to come from vcpkg and no longer does. vcpkg's COIN-OR ports pin raw
# master commits rather than releases: coin-or-cbc is REF ca088df3 with
# version-date 2024-06-04, coin-or-clp is dated 2023-02-01, coin-or-osi
# 2024-04-16 -- no tags, and not even contemporaries of each other. Cbc's master
# says AC_INIT([Cbc],[devel],...), so the cbc.pc vcpkg installs reports
# "Version: devel", and collo-cbc/build.rs asks pkg-config for CBC >= 2.10, which
# no version comparison can ever satisfy against the word "devel".
#
# COIN-OR publishes MSVC binaries with each release. This is not the same bargain
# as the gvsbuild zip we turned down: those were raw CI output the project says
# it cannot promise to update even for security issues, while these are the
# project's own release artifacts, tagged and versioned.
#
# The "md" in the name is the /MD compiler flag -- the dynamic C runtime, which
# is what rustc links with and what our vcpkg triplet is set to. The alternative
# asset is "dbg", a debug build. Everything inside is a static .lib and there is
# no DLL at all, which is what we need anyway: collo_cbc.cpp imports the data
# symbol cbcPreProcessPointer, and MSVC will not auto-import data from a DLL.
#
# One oddity, checked in the VM before adopting this. The .pc files carry the
# absolute path of the GitHub Actions runner that built them,
# prefix=/d/a/Cbc/Cbc/dist. pkgconf recomputes that prefix from wherever it finds
# the file, so what comes out is a real path under the directory below.

$CbcPc = Join-Path $CbcPrefix 'lib\pkgconfig\cbc.pc'
if (Test-Path $CbcPc) {
    Write-Step "CBC $CbcVersion is already unpacked"
    Write-Note "at: $CbcPrefix"
} else {
    $CbcZip = Join-Path $OutRoot "Cbc-$CbcVersion-w64-msvc17-md.zip"

    if (-not (Test-Path $CbcZip)) {
        Write-Step "downloading CBC $CbcVersion (about 18 MB)"
        Write-Note $CbcUrl

        # curl.exe, spelled with the extension because plain "curl" is a
        # PowerShell alias for Invoke-WebRequest. --fail matters: without it an
        # HTTP error page is written to the file and curl still exits 0, and we
        # would go on to unpack HTML as if it were a zip.
        & curl.exe --fail --location --output $CbcZip $CbcUrl

        if ($LASTEXITCODE -ne 0) {
            # Delete what landed. Otherwise the next run finds a file at that
            # path, skips the download, and fails on the checksum instead --
            # which is a confusing way to report a network problem.
            Remove-Item $CbcZip -Force -ErrorAction SilentlyContinue
            Write-Fail "could not download CBC (curl exit code $LASTEXITCODE)."
            Write-Note "the URL above is the pinned release asset; check it is reachable."
            exit 1
        }
    }

    # Get-FileHash returns uppercase hex and the pin above is lowercase, which is
    # fine: -ne on strings is case-insensitive in PowerShell. The pin is kept in
    # the case that sha256sum and GitHub print, so it can be pasted either way.
    $CbcActualSha = (Get-FileHash -Path $CbcZip -Algorithm SHA256).Hash
    if ($CbcActualSha -ne $CbcSha256) {
        Write-Fail "the CBC download does not match its expected checksum."
        Write-Note "expected: $CbcSha256"
        Write-Note "got:      $CbcActualSha"
        Write-Note "delete $CbcZip and run again."
        exit 1
    }

    Write-Step "unpacking CBC $CbcVersion"
    Expand-Archive -Path $CbcZip -DestinationPath $CbcPrefix -Force
    Write-Note "into: $CbcPrefix"
}

# The archives are named the autotools way -- libCbc.lib, libCbcSolver.lib -- but
# cbc.pc asks for -lCbc, and rustc on MSVC turns that straight into the filename
# Cbc.lib. Nothing in between translates: the first attempt died on
# "LNK1181: cannot open input file 'CbcSolver.lib'" with the right /LIBPATH
# already on the command line. So each archive is copied to its MSVC spelling.
#
# This is what vcpkg does to the same sources, which is why its tree carries
# Cbc.lib and even z.lib instead of zlib.lib. coinbrew would not have avoided it
# either -- it builds through the same libtool and produces the same names.
#
# Copies rather than renames, so the tree still matches what was unpacked. It
# sits outside the unpack branch above on purpose: a tree unpacked by an earlier
# run is fixed by the next run rather than needing to be deleted by hand.
$CbcLibDir = Join-Path $CbcPrefix 'lib'
if (-not (Test-Path (Join-Path $CbcLibDir 'Cbc.lib'))) {
    Write-Step "copying the CBC libraries to their MSVC names"
    $CbcCopied = 0
    foreach ($lib in Get-ChildItem -Path $CbcLibDir -Filter 'lib*.lib') {
        $MsvcName = $lib.Name.Substring(3)
        Copy-Item -Path $lib.FullName -Destination (Join-Path $CbcLibDir $MsvcName) -Force
        $CbcCopied++
    }
    Write-Note "$CbcCopied libraries, libFoo.lib -> Foo.lib"
}

Write-Host

# ---------------------------------------------------------------------------
# Python and pkgconf, from vcpkg
# ---------------------------------------------------------------------------

# Run from the manifest directory rather than pass --x-manifest-root: vcpkg
# picks up vcpkg.json from the working directory on its own, and that behaviour
# is not going to be renamed the way an experimental flag might be.
#
# vcpkg is called bare, with no pipeline and no redirection. That is the lesson
# bootstrap.ps1 learned the hard way: a pipeline ends when every handle on its
# input closes, not when the writing process exits, so a build that leaves a
# process behind hangs the script forever. It also gives back vcpkg's real
# progress output, which matters a lot across a build this long.

Write-Step "building Python and pkgconf with vcpkg (from source, on a first run)"
Write-Host

Push-Location $ManifestDir
try {
    & $VcpkgExe install `
        "--triplet=$Triplet" `
        "--overlay-triplets=$TripletDir" `
        "--x-install-root=$InstalledRoot"
} finally {
    Pop-Location
}

if ($LASTEXITCODE -ne 0) {
    Write-Host
    Write-Fail "vcpkg install failed (exit code $LASTEXITCODE)."
    Write-Note "the failing port prints the path of its build log; read that first."
    Write-Note "re-running is cheap: vcpkg keeps every package it already built."
    Write-Note "if a port is refused for the triplet rather than failing to compile,"
    Write-Note "--allow-unsupported turns that refusal into a warning and tries anyway."
    exit 1
}

Write-Host

# ---------------------------------------------------------------------------
# GTK and libadwaita, from gvsbuild
# ---------------------------------------------------------------------------

# gvsbuild is a Python program that builds the GTK stack with MSVC.
# bootstrap.ps1 installs it, pinned, with uv; this runs it with uv, which is what
# gvsbuild's documentation does for both of the install routes it offers.
#
# What gvsbuild is asked for. Their dependencies come automatically, and that is
# most of the stack: glib, cairo, pango, gdk-pixbuf, graphene, harfbuzz, and so
# on down.
#
# adwaita-icon-theme is here even though it is not a library. A libadwaita
# application with no icon theme shows no icons, and it does not arrive as a
# dependency -- gvsbuild's own CI lists it separately for the same reason.
#
# librsvg is not listed and does not need to be. gvsbuild makes it a dependency
# of gtk4 itself, and adwaita-icon-theme depends on it as well, so it is built
# either way -- which settles the "will symbolic icons render" question before it
# was asked.
#
# It is also why a run downloads rustup. librsvg is written in Rust, and gvsbuild
# installs its own pinned cargo as one of its tools rather than using the rustup
# that bootstrap.ps1 installed.
#
# gtksourceview5, gtkmm, protobuf-c and the PyGObject wheels are in gvsbuild's CI
# build and deliberately not here. We use none of them.
$GtkProjects = @(
    'gtk4'
    'libadwaita'
    'adwaita-icon-theme'
)

$Uv = Get-Command uv -ErrorAction SilentlyContinue
if (-not $Uv) {
    Write-Fail "uv is not in PATH, so gvsbuild cannot be run."
    Write-Note "bootstrap.ps1 installs uv and gvsbuild. If it has already run, open"
    Write-Note "a new terminal -- PATH is only picked up by processes started after."
    exit 1
}

# --configuration release rather than gvsbuild's default of debug-optimized. The
# install prefix is <build-dir>\gtk\<platform>\<configuration>, so passing both
# explicitly is what makes $GtkPrefix above correct rather than a guess.
#
# --fast-build skips projects already built, which is what makes an interrupted
# run cheap to resume.
#
# --msys-dir is not passed. gvsbuild searches the usual locations, and
# bootstrap.ps1 installs MSYS2 into the first of them, C:\msys64. Pass it here if
# that ever stops being true.
#
# --enable-gi and --py-wheel are on gvsbuild's CI line and left off ours. They
# build GObject introspection data and Python wheels; gtk4-rs links the libraries
# directly and needs neither, and gobject-introspection is a heavy dependency to
# carry for nothing.
#
# Called bare, no pipeline and no redirection, for the reason spelled out above
# the vcpkg call.
$UvArgs = @(
    'run', 'gvsbuild', 'build'
    '--build-dir', $GtkBuildRoot
    '--platform', 'x64'
    '--configuration', 'release'
    '--vs-ver', 'vs2022'
    '--fast-build'
) + $GtkProjects

Write-Step "building GTK and libadwaita with gvsbuild (hours, from source)"
Write-Note "projects: $($GtkProjects -join ' ')"
Write-Note "into:     $GtkPrefix"
Write-Host

& uv @UvArgs

if ($LASTEXITCODE -ne 0) {
    Write-Host
    Write-Fail "gvsbuild failed (exit code $LASTEXITCODE)."
    Write-Note "it names the project it was on and the log it was writing; read that."
    Write-Note "re-running resumes -- --fast-build skips what is already built."
    Write-Note "if a project complains about a missing shell command, MSYS2 needs the"
    Write-Note "package that provides it: pacman -S <package> in C:\msys64."
    exit 1
}

Write-Host

# ---------------------------------------------------------------------------
# What actually landed
# ---------------------------------------------------------------------------
#
# The build is long enough that "it exited 0" is not a satisfying answer. This
# names the files the next steps depend on, so a failure is seen here rather
# than an hour into the cargo build.

Write-Step "the three prefixes"
Write-Note "vcpkg:    $Prefix"
Write-Note "gvsbuild: $GtkPrefix"
Write-Note "cbc:      $CbcPrefix"
Write-Host

# Reporting only. A tool that writes a warning to stderr while printing its
# version must not abort the script the way 'Stop' would make it.
$ErrorActionPreference = 'Continue'

# cbc.pc is what collo-cbc/build.rs probes for; gtk4.pc and libadwaita-1.pc are
# what the gtk4 and libadwaita crates probe for. Each lives in its own prefix,
# which is why all three directories go on PKG_CONFIG_PATH further down.
#
# python3 is not looked for: pyo3 finds Python through an interpreter rather than
# pkg-config, so whether the port ships a .pc file says nothing either way.
foreach ($group in @(
    @{ Dir = (Join-Path $CbcPrefix 'lib\pkgconfig'); Files = @('cbc.pc') }
    @{ Dir = (Join-Path $GtkPrefix 'lib\pkgconfig'); Files = @('gtk4.pc', 'libadwaita-1.pc') }
)) {
    foreach ($pc in $group.Files) {
        $path = Join-Path $group.Dir $pc
        if (Test-Path $path) {
            Write-Note "$($pc): $path"
        } else {
            Write-Note "$($pc): NOT FOUND in $($group.Dir)"
        }
    }
}

# Runs $Path and prints its first line of output, or says where it looked.
function Show-Tool {
    param([string]$Label, [string]$Path, [string[]]$Arguments)

    if (-not (Test-Path $Path)) {
        Write-Note "$($Label): NOT FOUND at $Path"
        return
    }

    $output = (& $Path @Arguments 2>$null | Select-Object -First 1)
    if ($output) {
        Write-Note "$($Label): $output"
    } else {
        Write-Note "$($Label): $Path (printed nothing)"
    }
}

# gtk4/build.rs runs this one at compile time. It comes with glib, so it is in
# the gvsbuild prefix and not the vcpkg one.
Show-Tool -Label 'glib-compile-resources' `
    -Path (Join-Path $GtkPrefix 'bin\glib-compile-resources.exe') `
    -Arguments @('--version')

# The pkg-config implementation the cargo build will be pointed at.
Show-Tool -Label 'pkgconf' `
    -Path (Join-Path $Prefix 'tools\pkgconf\pkgconf.exe') `
    -Arguments @('--version')

# The interpreter the application links against, and the one it will ship -- not
# the gvsbuild one above. A Python that exists to be linked against is not
# necessarily one a teacher can install packages into, which is why pip is
# reported rather than assumed. The port ships pip as a bundled wheel rather than
# installed, so the pip line reads "printed nothing" until
# `python -m ensurepip --upgrade` has been run once. Whether the shipped bundle
# does that, and when, is a step-7 decision.
$VcpkgPython = Join-Path $Prefix 'tools\python3\python.exe'
Show-Tool -Label 'python' -Path $VcpkgPython -Arguments @('--version')
Show-Tool -Label 'python pip' -Path $VcpkgPython -Arguments @('-m', 'pip', '--version')

$pythonLib = Join-Path $Prefix 'tools\python3\Lib'
if (Test-Path $pythonLib) {
    $moduleCount = @(Get-ChildItem -Path $pythonLib -Filter '*.py' -ErrorAction SilentlyContinue).Count
    Write-Note "python stdlib: $pythonLib ($moduleCount top-level .py files)"
} else {
    Write-Note "python stdlib: NOT FOUND at $pythonLib"
}

Write-Host

# ---------------------------------------------------------------------------
# Collomatique itself
# ---------------------------------------------------------------------------

# $ErrorActionPreference stays 'Continue' from the report above rather than going
# back to 'Stop'. cargo writes its whole progress display to stderr, and the
# combination of 'Stop' and a chatty native command is a well-known way to have a
# script die on a line that was only ever a status message. Success is decided by
# $LASTEXITCODE below, which is what cargo actually promises.

# One binary: collomatique-gtk4 is a multiplexed exe. The GUI re-executes itself
# with --rpc-engine for the solver and Python workers, so this single build
# already contains everything -- there is no second binary to produce.
$CargoPackage = 'collomatique-gtk4'

# Four environment variables are what connect the workspace to the three prefixes
# above. They are set here rather than expected from the machine, so that a build
# depends on this script and not on how a terminal happens to be configured, and
# they are set on this process only -- nothing is written back to the machine.

# The -sys crates find their C libraries through pkg-config. Rust's pkg_config
# crate runs whatever $PKG_CONFIG names, so this points it at the pkgconf vcpkg
# built rather than requiring one on PATH.
$env:PKG_CONFIG = Join-Path $Prefix 'tools\pkgconf\pkgconf.exe'

# Three directories, one per prefix. The separator is a semicolon -- this is a
# native Windows pkgconf, and it splits the variable the way Windows splits PATH,
# not the way Unix does.
#
# vcpkg's is still listed even though nothing in the workspace asks pkg-config for
# Python: pkgconf resolves Requires: chains through this same path, so a future
# port that others depend on is found without editing this line.
#
# CBC's comes first on purpose. An install root built before coin-or-cbc was
# dropped from vcpkg.json may still hold the old "Version: devel" cbc.pc, and
# pkgconf takes the first match in the path.
$env:PKG_CONFIG_PATH = @(
    (Join-Path $CbcPrefix 'lib\pkgconfig')
    (Join-Path $GtkPrefix 'lib\pkgconfig')
    (Join-Path $Prefix    'lib\pkgconfig')
) -join ';'

# gtk4/build.rs compiles the .gresource bundle through glib-build-tools, which
# spawns glib-compile-resources by name and therefore needs it on PATH. It comes
# with glib, so it is in the gvsbuild prefix.
$env:PATH = (Join-Path $GtkPrefix 'bin') + ';' + $env:PATH

# pyo3 has no interpreter to guess between here, and must not be allowed to guess
# anyway: the interpreter the application links against has to be the one it will
# ship, or the DLL in the bundle will not match the one it was built for.
$env:PYO3_PYTHON = $VcpkgPython

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Fail "cargo is not in PATH."
    Write-Note "bootstrap.ps1 installs rustup. If it has already run, open a new"
    Write-Note "terminal -- PATH is only picked up by processes started afterwards."
    exit 1
}

Write-Step "building $CargoPackage (release)"
Write-Note "PKG_CONFIG_PATH: $env:PKG_CONFIG_PATH"
Write-Note "PYO3_PYTHON:     $env:PYO3_PYTHON"
Write-Note "target dir:      $CargoTarget"
Write-Host

# Bare, no pipeline and no redirection, for the reason given above the vcpkg call.
Push-Location $RepoRoot
try {
    & cargo build --release --package $CargoPackage --target-dir $CargoTarget
} finally {
    Pop-Location
}

if ($LASTEXITCODE -ne 0) {
    Write-Host
    Write-Fail "cargo build failed (exit code $LASTEXITCODE)."
    Write-Note "if a -sys crate could not find its library, the error names the .pc"
    Write-Note "file it wanted; check it is in one of the two directories above."
    exit 1
}

Write-Host

$Exe = Join-Path $CargoTarget "release\$CargoPackage.exe"
if (Test-Path $Exe) {
    Write-Step "built: $Exe"
} else {
    Write-Fail "cargo reported success but $Exe is not there."
    exit 1
}

# ---------------------------------------------------------------------------
# The staged application
# ---------------------------------------------------------------------------

# Everything a machine with no build tools needs, arranged the way it will be
# installed. The installer step will point Inno Setup at this directory and copy
# it verbatim, so anything wrong here is wrong in the installer too.
#
# No configuration file, no environment variable and no code in the application
# makes this work. Both halves relocate themselves:
#
#   - glib takes the directory of its own loaded DLL as the installation prefix,
#     so lib\ and share\ beside the exe are found wherever the folder ends up.
#   - CPython searches for Lib\os.py starting beside the executable, so the same
#     directory is also a Python prefix.
#
# The consequence, and it surprises everyone who opens the folder: Windows
# filenames are case-insensitive, so Python's Lib\ and glib's lib\ are one
# directory. It holds the standard library and gdk-pixbuf-2.0\ side by side.
# That is deliberate. The tidy alternative was a python312._pth file naming a
# python\ subdirectory, and it was turned down because such a file makes the
# interpreter ignore every environment variable -- which would settle, badly and
# in advance, the question of where a teacher's own packages go.

$StageRoot = Join-Path $OutRoot 'stage'

# Copies the contents of $From into $To, creating $To. $From is a wildcard or a
# single file; a bare directory name would copy the directory itself instead of
# its contents, so call sites always end in \*.
function Copy-Staged {
    param([string]$Label, [string]$From, [string]$To)

    $found = @(Get-Item -Path $From -ErrorAction SilentlyContinue)
    if ($found.Count -eq 0) {
        Write-Fail "nothing to stage for $($Label): $From"
        exit 1
    }
    $null = New-Item -ItemType Directory -Path $To -Force
    Copy-Item -Path $From -Destination $To -Recurse -Force
    Write-Note "$($Label): $($found.Count)"
}

Write-Step "staging the application"
Write-Note "into: $StageRoot"

# Rebuilt from nothing every time. The copying is trivial next to the rest of
# this script, and a stale DLL left behind by a rename upstream is exactly the
# kind of bug that only appears on someone else's machine.
if (Test-Path $StageRoot) {
    Remove-Item -Path $StageRoot -Recurse -Force
}
$null = New-Item -ItemType Directory -Path $StageRoot

Copy-Item -Path $Exe -Destination $StageRoot -Force
Write-Note "the application: $CargoPackage.exe"

# Everything gvsbuild built, rather than a list of what the exe imports. A list
# would be shorter and would go stale silently the first time a GTK version
# changes what it pulls in; this cannot.
Copy-Staged 'GTK, libadwaita and their stack' (Join-Path $GtkPrefix 'bin\*.dll') $StageRoot

# python312.dll, and the libraries the standard library loads at run time --
# sqlite3, the SSL pair, bz2, lzma, ffi. Missing one of these does not fail at
# startup, it fails the first time a teacher's script imports something.
Copy-Staged 'the Python runtime DLLs' (Join-Path $Prefix 'bin\*.dll') $StageRoot

# python.exe comes along on purpose: it is how a teacher installs a package into
# this interpreter later. Nothing is installed into it here -- pip and xlsxwriter
# belong to whatever ships the application, not to this staging directory.
Copy-Staged 'the Python interpreter and standard library' `
    (Join-Path $Prefix 'tools\python3\*') $StageRoot

# The image loaders. GTK4 has its own PNG and JPEG loading, but icon themes are
# SVG and that goes through librsvg's gdk-pixbuf loader -- and the whole UI is
# built out of Adwaita's *-symbolic icons, so without this there are no icons.
Copy-Staged 'the gdk-pixbuf loaders' `
    (Join-Path $GtkPrefix 'lib\gdk-pixbuf-2.0\*') (Join-Path $StageRoot 'lib\gdk-pixbuf-2.0')

# GTK reads its own settings from GSettings and will not start without the
# compiled schemas.
Copy-Staged 'the GSettings schemas' `
    (Join-Path $GtkPrefix 'share\glib-2.0\schemas\gschemas.compiled') `
    (Join-Path $StageRoot 'share\glib-2.0\schemas')

# Adwaita is the theme the UI names icons from. hicolor is the fallback theme
# every GTK application is required to have present.
Copy-Staged 'the Adwaita icon theme' `
    (Join-Path $GtkPrefix 'share\icons\Adwaita\*') (Join-Path $StageRoot 'share\icons\Adwaita')
Copy-Staged 'the hicolor fallback theme' `
    (Join-Path $GtkPrefix 'share\icons\hicolor\*') (Join-Path $StageRoot 'share\icons\hicolor')

# GTK's and libadwaita's own translations -- the text in file dialogs and
# standard buttons. Every language, not just French: picking is a size
# optimisation and belongs with the installer, not here.
Copy-Staged 'the GTK translations' `
    (Join-Path $GtkPrefix 'share\locale\*') (Join-Path $StageRoot 'share\locale')

# The loader cache has to be rebuilt, because the one in gvsbuild's prefix names
# absolute paths inside C:\collo-build. Written with paths relative to the cache
# file itself, which is what lets the folder be installed anywhere.
#
# The version directory is read rather than written down: it is gdk-pixbuf's
# module ABI version, currently 2.10.0, and it is not ours to predict.
$PixbufRoot    = Join-Path $StageRoot 'lib\gdk-pixbuf-2.0'
$PixbufAbiDir  = @(Get-ChildItem -Path $PixbufRoot -Directory)[0].FullName
$QueryLoaders  = Join-Path $GtkPrefix 'bin\gdk-pixbuf-query-loaders.exe'

Write-Step "rebuilding the gdk-pixbuf loader cache"
Push-Location $PixbufAbiDir
try {
    # Relative names in, relative names out. Capturing output from a native
    # command is what the rest of this script avoids, but the rule is about
    # long-running builds holding a pipeline open; this one prints and exits.
    $LoaderNames = @(Get-ChildItem -Path 'loaders' -Filter '*.dll' |
        ForEach-Object { Join-Path 'loaders' $_.Name })
    $LoaderCache = & $QueryLoaders @LoaderNames
} finally {
    Pop-Location
}

if ($LASTEXITCODE -ne 0 -or -not $LoaderCache) {
    Write-Fail "gdk-pixbuf-query-loaders produced no cache (exit code $LASTEXITCODE)."
    Write-Note "without it the application starts but shows no icons at all."
    exit 1
}
Set-Content -Path (Join-Path $PixbufAbiDir 'loaders.cache') -Value $LoaderCache -Encoding ASCII
Write-Note "$($LoaderNames.Count) loaders"

Write-Host

$StageSize = (Get-ChildItem -Path $StageRoot -Recurse -File | Measure-Object -Property Length -Sum).Sum
Write-Step "staged: $StageRoot"
Write-Note ("{0:N0} MB" -f ($StageSize / 1MB))
Write-Note "run it from there. It should not need anything from this build"
Write-Note "environment -- if it does, that is the bug this directory exists to find."
