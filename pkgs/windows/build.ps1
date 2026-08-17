# Build Collomatique for Windows.
#
#     powershell -ExecutionPolicy Bypass -File pkgs\windows\build.ps1
#
# Run it from a terminal opened after pkgs\windows\bootstrap.ps1 has finished,
# so that VCPKG_ROOT is in the environment.
#
# What it does today is the dependencies, in two halves. CBC and the Python the
# application embeds are built by vcpkg, from the manifest next to this file.
# GTK and libadwaita are built by gvsbuild. Then it reports what landed. It does
# not compile Collomatique itself yet; the rest is added one piece at a time as
# the roadmap advances: the cargo build, the bundle layout, then the installer.
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
# Nothing is written inside the repository. Output goes under -OutRoot and
# -GtkBuildRoot; -OutRoot is also where the staged application and the installer
# will be put later.

#Requires -Version 5.1

param(
    # Outside the working tree, like pkgs/flatpak/build.sh does with its own
    # output. Short, and on C:, because vcpkg's build trees nest deeply against
    # a 260-character path limit.
    [string]$OutRoot = 'C:\collomatique-build',

    # gvsbuild's own default, and not nested under -OutRoot on purpose. It nests
    # deeply too -- <root>\build\x64\release\<project>\... and then the project's
    # own tree below that -- and gvsbuild chose a two-segment path at the root of
    # C: to stay clear of the same 260-character limit. Twenty more characters of
    # prefix would be spent for tidiness and paid for by a build that fails hours
    # in. It also means the paths in gvsbuild's own logs and documentation match.
    [string]$GtkBuildRoot = 'C:\gtk-build',

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

# Where gvsbuild puts what it installs. Not a choice: it builds the path as
# <build-dir>\gtk\<platform>\<configuration>, and the platform and configuration
# below are the ones passed on its command line.
$GtkPrefix = Join-Path $GtkBuildRoot 'gtk\x64\release'

# The virtual environment gvsbuild itself is installed into. Under -OutRoot
# rather than -GtkBuildRoot: it is a tool we install, not something gvsbuild
# produces, and gvsbuild deletes inside its own build root on --from-scratch.
$VenvDir = Join-Path $OutRoot 'gvsbuild-venv'

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
        Write-Note "or pass a path you own: -OutRoot C:\Users\you\collomatique-build"
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

# gvsbuild is a Python program that builds the GTK stack with MSVC. It is
# installed here rather than in bootstrap.ps1 because it is pinned, and pins
# belong with the build -- the same reason vcpkg's baseline is in vcpkg.json and
# not in bootstrap.ps1.
#
# It goes into a virtual environment of its own. That gives an exact path to
# gvsbuild.exe, where pip's console scripts otherwise land in a Scripts
# directory whose place depends on how Python was installed, and it leaves the
# machine Python untouched.
#
# gvsbuild's own documentation installs it with uv. pip into a venv needs no
# eighth tool in bootstrap.ps1 and pins the version in one line, which is worth
# more to us here.

$GvsbuildVersion = '2026.8.0'

# What gvsbuild is asked for. Their dependencies come automatically, and that is
# most of the stack: glib, cairo, pango, gdk-pixbuf, graphene, harfbuzz, and so
# on down.
#
# adwaita-icon-theme is here even though it is not a library. A libadwaita
# application with no icon theme shows no icons, and it does not arrive as a
# dependency -- gvsbuild's own CI lists it separately for the same reason.
#
# Not here, and the first thing to add if icons come out missing: librsvg, the
# SVG loader. adwaita-icon-theme ships SVG only, so something has to render it.
# Left out of the first build to keep the failure surface smaller -- it is a Rust
# project, so it brings cargo into the GTK build too.
#
# gtksourceview5, gtkmm, protobuf-c and the PyGObject wheels are in gvsbuild's CI
# build and deliberately not here. We use none of them.
$GtkProjects = @(
    'gtk4'
    'libadwaita'
    'adwaita-icon-theme'
)

# The tool Python from bootstrap.ps1, found on PATH. Not the vcpkg one built
# above: that is the interpreter the application embeds, and it has no pip.
$ToolPython = Get-Command python -ErrorAction SilentlyContinue
if (-not $ToolPython) {
    Write-Fail "python is not in PATH, so gvsbuild cannot be installed."
    Write-Note "bootstrap.ps1 installs it. If that has already run, open a new"
    Write-Note "terminal -- PATH is only picked up by processes started afterwards."
    exit 1
}

$VenvPython  = Join-Path $VenvDir 'Scripts\python.exe'
$GvsbuildExe = Join-Path $VenvDir 'Scripts\gvsbuild.exe'

if (-not (Test-Path $VenvPython)) {
    Write-Step "creating a virtual environment for gvsbuild in $VenvDir"
    & $ToolPython.Source -m venv $VenvDir
    if (($LASTEXITCODE -ne 0) -or (-not (Test-Path $VenvPython))) {
        Write-Fail "could not create a virtual environment with $($ToolPython.Source)."
        Write-Note "if that path is under WindowsApps, PATH is finding Windows' Store"
        Write-Note "stub instead of a real interpreter. bootstrap.ps1 installs Python"
        Write-Note "3.12 machine-wide; call that python.exe by its full path to check."
        exit 1
    }
}

# Run every time rather than guarded by a stamp: pip does nothing, and reaches no
# network, when the exact version asked for is already installed.
# --disable-pip-version-check stops it contacting PyPI just to say pip itself is
# out of date.
Write-Step "installing gvsbuild $GvsbuildVersion"
& $VenvPython -m pip install --disable-pip-version-check "gvsbuild==$GvsbuildVersion"
if ($LASTEXITCODE -ne 0) {
    Write-Host
    Write-Fail "pip could not install gvsbuild==$GvsbuildVersion (exit code $LASTEXITCODE)."
    Write-Note "gvsbuild wants Python 3.10 or newer and refuses exactly 3.13.4. If the"
    Write-Note "version above no longer exists on PyPI, this line is what to change."
    exit 1
}

if (-not (Test-Path $GvsbuildExe)) {
    Write-Fail "gvsbuild installed, but $GvsbuildExe is not there."
    Write-Note "delete $VenvDir and run this again."
    exit 1
}

Write-Host

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
$GvsbuildArgs = @(
    'build'
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

& $GvsbuildExe @GvsbuildArgs

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
Write-Step "dependencies done. Compiling Collomatique itself is the next step."
