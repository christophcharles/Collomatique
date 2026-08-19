# Build Collomatique for Windows.
#
#     powershell -ExecutionPolicy Bypass -File pkgs\windows\build.ps1
#
# Run it from a terminal opened after pkgs\windows\bootstrap.ps1 has finished,
# so that VCPKG_ROOT is in the environment.
#
# What it does today is the dependencies, then Collomatique. The dependencies
# come in two halves: CBC and the Python the application embeds are built by
# vcpkg, from the manifest next to this file, and GTK and libadwaita are built by
# gvsbuild. It reports what landed, then builds the application against both. The
# rest is added one piece at a time as the roadmap advances: the bundle layout,
# then the installer.
#
# The halves are separate because GTK cannot come from vcpkg. libadwaita needs
# appstream, appstream needs libxmlb, and vcpkg's libxmlb port declares
# "supports": "!windows | mingw", so vcpkg refuses it for MSVC before compiling
# anything. gtk.org names MSYS2 and gvsbuild for Windows and never vcpkg, and
# gvsbuild is the one that produces MSVC libraries. So each half uses the tool
# its own upstream blesses, and the two produce two prefixes.
#
# Expect most of a day on a first run, and a lot of downloading: both halves
# build everything from source. Interrupting is safe and re-running resumes --
# vcpkg keeps every package it has already built, and gvsbuild is asked to skip
# projects it has already built.
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

Write-Step "Collomatique Windows build"
Write-Note "manifest: $ManifestDir"
Write-Note "triplet:  $Triplet"
Write-Note "output:   $OutRoot"
Write-Note "gtk:      $GtkBuildRoot"
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
# CBC and Python, from vcpkg
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

Write-Step "building the dependencies with vcpkg (from source, on a first run)"
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

Write-Step "the two prefixes"
Write-Note "vcpkg:    $Prefix"
Write-Note "gvsbuild: $GtkPrefix"
Write-Host

# Reporting only. A tool that writes a warning to stderr while printing its
# version must not abort the script the way 'Stop' would make it.
$ErrorActionPreference = 'Continue'

# cbc.pc is what collo-cbc/build.rs probes for; gtk4.pc and libadwaita-1.pc are
# what the gtk4 and libadwaita crates probe for. They are in two different
# prefixes, which is why step 4 will put both directories on PKG_CONFIG_PATH.
#
# python3 is not looked for: pyo3 finds Python through an interpreter rather than
# pkg-config, so whether the port ships a .pc file says nothing either way.
foreach ($group in @(
    @{ Dir = (Join-Path $Prefix    'lib\pkgconfig'); Files = @('cbc.pc') }
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

# Four environment variables are what connect the workspace to the two prefixes
# above. They are set here rather than expected from the machine, so that a build
# depends on this script and not on how a terminal happens to be configured, and
# they are set on this process only -- nothing is written back to the machine.

# The -sys crates find their C libraries through pkg-config. Rust's pkg_config
# crate runs whatever $PKG_CONFIG names, so this points it at the pkgconf vcpkg
# built rather than requiring one on PATH.
$env:PKG_CONFIG = Join-Path $Prefix 'tools\pkgconf\pkgconf.exe'

# Two directories, because there are two prefixes: CBC is vcpkg's, the whole GTK
# stack is gvsbuild's. The separator is a semicolon -- this is a native Windows
# pkgconf, and it splits the variable the way Windows splits PATH, not the way
# Unix does.
$env:PKG_CONFIG_PATH = @(
    (Join-Path $Prefix    'lib\pkgconfig')
    (Join-Path $GtkPrefix 'lib\pkgconfig')
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

Write-Note "it will not run outside this build environment yet -- the GTK DLLs are"
Write-Note "in the gvsbuild prefix, not beside it. Bundling is the next step."
