# Build Collomatique for Windows.
#
#     powershell -ExecutionPolicy Bypass -File pkgs\windows\build.ps1
#
# Run it from a terminal opened after pkgs\windows\bootstrap.ps1 has finished,
# so that VCPKG_ROOT is in the environment.
#
# What it does today is the first piece only: it builds the C and C++
# dependencies -- GTK, libadwaita, CBC, Python -- with vcpkg, from the manifest
# next to this file, and then reports what landed. It does not compile
# Collomatique itself yet. The rest of the build is added one piece at a time as
# the roadmap advances: the cargo build, the bundle layout, then the installer.
#
# The first run compiles the whole GTK stack from source and takes hours, not
# minutes. Interrupting it is safe and re-running resumes: vcpkg keeps what it
# has already built.
#
# Nothing is written inside the repository. Everything goes under -OutRoot,
# which is also where the staged application and the installer will be put later.

#Requires -Version 5.1

param(
    # Outside the working tree, like pkgs/flatpak/build.sh does with its own
    # output. Short, and on C:, because vcpkg's build trees nest deeply against
    # a 260-character path limit.
    [string]$OutRoot = 'C:\collomatique-build',

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

Write-Step "Collomatique Windows build"
Write-Note "manifest: $ManifestDir"
Write-Note "triplet:  $Triplet"
Write-Note "output:   $OutRoot"
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
# The C and C++ dependencies
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

Write-Step "building the dependencies with vcpkg (hours on a first run)"
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

Write-Step "installed into $Prefix"

# Reporting only. A tool that writes a warning to stderr while printing its
# version must not abort the script the way 'Stop' would make it.
$ErrorActionPreference = 'Continue'

# The .pc files the -sys crates read. python3 is deliberately not among them:
# pyo3 finds Python through an interpreter, not through pkg-config, so whether
# the port ships a .pc file says nothing either way.
$PkgConfigDir = Join-Path $Prefix 'lib\pkgconfig'
foreach ($pc in @('gtk4.pc', 'libadwaita-1.pc', 'cbc.pc')) {
    $path = Join-Path $PkgConfigDir $pc
    if (Test-Path $path) {
        Write-Note "$($pc): $path"
    } else {
        Write-Note "$($pc): NOT FOUND in $PkgConfigDir"
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

# gtk4/build.rs needs this on PATH when the cargo build runs.
Show-Tool -Label 'glib-compile-resources' `
    -Path (Join-Path $Prefix 'tools\glib\glib-compile-resources.exe') `
    -Arguments @('--version')

# The pkg-config implementation the cargo build will be pointed at.
Show-Tool -Label 'pkgconf' `
    -Path (Join-Path $Prefix 'tools\pkgconf\pkgconf.exe') `
    -Arguments @('--version')

# The interpreter the application links against, and the one it will ship. Both
# lines below are open questions the roadmap says to answer by running rather
# than by reasoning: a Python that exists to be linked against is not
# necessarily one a teacher can install packages into. Reported, not enforced --
# what to do about it, if anything, is decided when the bundle is laid out.
$python = Join-Path $Prefix 'tools\python3\python.exe'
Show-Tool -Label 'python' -Path $python -Arguments @('--version')
Show-Tool -Label 'python pip' -Path $python -Arguments @('-m', 'pip', '--version')

$pythonLib = Join-Path $Prefix 'tools\python3\Lib'
if (Test-Path $pythonLib) {
    $moduleCount = @(Get-ChildItem -Path $pythonLib -Filter '*.py' -ErrorAction SilentlyContinue).Count
    Write-Note "python stdlib: $pythonLib ($moduleCount top-level .py files)"
} else {
    Write-Note "python stdlib: NOT FOUND at $pythonLib"
}

Write-Host
Write-Step "dependencies done. Compiling Collomatique itself is the next step."
