# Install the Windows development tools Collomatique is built with.
#
#     powershell -ExecutionPolicy Bypass -File pkgs\windows\bootstrap.ps1
#
# Run it once in a fresh Windows 11 VM, then take a snapshot: that snapshot is
# the real build environment, and this script is the recipe for recreating one.
# Running it again on a machine that already has the tools is safe; it skips
# what is already installed and reports what it found.
#
# Five tools, and that is the whole list:
#
#   VS Build Tools   the MSVC compiler and the Windows SDK. No IDE. Using it
#                    requires a valid Visual Studio licence; Visual Studio
#                    Community grants one, free, to an individual developer.
#   git              needed to get vcpkg, which is a git checkout.
#   vcpkg            every C/C++ dependency of the app comes from here: GTK,
#                    libadwaita, CBC, and Python too. Python is not a separate
#                    tool: the app links against libpython, so it is a build
#                    dependency like the others, and taking it from vcpkg means
#                    the interpreter we link and the interpreter we ship are
#                    the same one.
#                    winget has no vcpkg package, so this one is cloned and
#                    bootstrapped instead. See the clone section below.
#   rustup           the Rust toolchain, tracking stable. Deliberately not
#                    pinned to a version: Cargo.lock already pins every
#                    dependency, and the flatpak tracks rust-stable the same
#                    way, so all platforms build with roughly the same compiler.
#   Inno Setup 7     compiles the installer at the end of the build. The current
#                    line, rather than the 6 that winget also still offers.
#
# This script installs tools. It does not build anything and it installs no
# vcpkg package -- the build script does that from a manifest in the repository,
# which is where the pinning of GTK, CBC and Python versions belongs. The one
# lasting mark it leaves on the machine, beyond the tools themselves, is the
# VCPKG_ROOT environment variable.

#Requires -Version 5.1

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Native commands below are checked through $LASTEXITCODE, which does not exist
# until one has run. Strict mode makes reading it before then an error.
$global:LASTEXITCODE = 0

# ---------------------------------------------------------------------------
# What gets installed
# ---------------------------------------------------------------------------

# Passed through to the Visual Studio installer. Changing this list has no
# effect on a machine where Build Tools is already installed; the script says
# so, and prints the command to re-run.
#
# The Rust documentation's version of this command also adds
# Microsoft.VisualStudio.Component.VC.Tools.ARM64. Collomatique targets x86_64
# only (roadmap, "Out of scope"), so the ARM64 cross compiler is a couple of
# gigabytes bought for nothing. Add it back here if that ever changes.
#
# The SDK component carries its version, so it is a real pin: a rebuild next
# year uses the SDK this line names, not whatever is current then.
#
# --override hands this whole string to the Visual Studio bootstrapper and
# replaces the switches winget would otherwise supply itself, so the first
# three have to be here:
#
#   --passive    a progress bar, and nothing to answer. --quiet would show
#                nothing at all, and this downloads gigabytes: on a slow line
#                that is a long time with no way to tell it apart from a hang.
#   --wait       without it the bootstrapper returns as soon as it has started
#                the real installer, winget reports success, and the rest of
#                this script runs while Visual Studio is still installing.
#   --norestart  a reboot, if one is wanted, happens when you say so.
$BuildToolsOverride = @(
    '--passive'
    '--wait'
    '--norestart'
    '--add Microsoft.VisualStudio.Component.VC.Tools.x86.x64'
    '--add Microsoft.VisualStudio.Component.Windows11SDK.22621'
    '--addProductLang En-us'
) -join ' '

$Packages = @(
    @{
        Id     = 'Microsoft.VisualStudio.2022.BuildTools'
        Name   = 'Visual Studio Build Tools 2022'
        Extra  = @('--force', '--override', $BuildToolsOverride)
        Verify = { [bool](Get-MsvcPath) }
    }
    @{
        Id   = 'Git.Git'
        Name = 'git'
    }
    @{
        Id   = 'Rustlang.Rustup'
        Name = 'rustup'
    }
    @{
        # The unsuffixed JRSoftware.InnoSetup id is the older 6.x line. The
        # suffix is the version, so a move to 8 later is an edit here.
        Id   = 'JRSoftware.InnoSetup.7'
        Name = 'Inno Setup 7'
    }
)

# vcpkg is not in winget, so it is cloned from its own repository, which is the
# way its documentation describes anyway.
#
# A short root, on C: rather than under the user profile: vcpkg builds from
# source and those trees nest deeply, against a 260-character path limit.
$VcpkgRoot = 'C:\vcpkg'
$VcpkgRepository = 'https://github.com/microsoft/vcpkg.git'

# The host triple is named rather than left to rustup's autodetection, so the
# script says out loud which toolchain the build expects. msvc, not gnu: the
# whole C/C++ side comes from vcpkg built with the Build Tools above.
$RustToolchain = 'stable-x86_64-pc-windows-msvc'

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

function Write-Step  { param([string]$Message) Write-Host "==> $Message" }
function Write-Note  { param([string]$Message) Write-Host "    $Message" }
function Write-Fail  { param([string]$Message) Write-Host "error: $Message" }

# winget writes PATH into the registry, so a tool installed a moment ago is not
# visible to this still-running process. Re-read both PATHs and rebuild the
# session's own, or every command below would have to be called by full path.
function Update-SessionPath {
    $machine = [Environment]::GetEnvironmentVariable('Path', 'Machine')
    $user    = [Environment]::GetEnvironmentVariable('Path', 'User')
    $env:Path = (@($machine, $user) | Where-Object { $_ }) -join ';'
}

# `winget list` exits 0 when it finds the package and non-zero when it does
# not. Which non-zero code means "nothing matched" has moved between winget
# versions, so only the zero is trusted here.
#
# --accept-source-agreements matters even for a query: on a machine where they
# have not been accepted, winget asks, and this call throws its output away, so
# the question would be invisible and the script would wait for an answer to a
# prompt nobody can see.
#
# stderr goes to $null rather than being merged with 2>&1: merging turns a
# native command's stderr into error records, and $ErrorActionPreference =
# 'Stop' then throws on them, which would fail the check for a package that is
# perfectly well installed.
function Test-PackageInstalled {
    param([string]$Id)

    $null = & winget list --exact --id $Id --accept-source-agreements 2>$null
    return ($LASTEXITCODE -eq 0)
}

# Where the MSVC toolset is, or $null. vswhere.exe ships with any Visual Studio
# and is the supported way to find one. Asking for the component rather than the
# product means an install without the C++ tools answers $null rather than
# looking fine.
function Get-MsvcPath {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path $vswhere)) { return $null }

    $found = & $vswhere -products '*' `
        -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
        -property installationPath -latest 2>$null

    if ($found) { return ($found | Select-Object -First 1) }
    return $null
}

function Install-Package {
    param([hashtable]$Package)

    $id   = $Package.Id
    $name = $Package.Name

    # A package can be registered with winget and still not be usable -- an
    # interrupted Visual Studio install leaves exactly that. Where an entry
    # carries a Verify block, it, and not winget's list, decides whether there
    # is anything to do.
    if (Test-PackageInstalled -Id $id) {
        if ((-not $Package.ContainsKey('Verify')) -or (& $Package.Verify)) {
            Write-Step "$name is already installed, skipping"
            if ($Package.ContainsKey('Extra')) {
                Write-Note "to change its components, re-run winget by hand:"
                Write-Note "  winget install --exact --id $id --source winget $($Package.Extra -join ' ')"
            }
            return $true
        }
        Write-Step "$name is registered but incomplete, installing it again"
    }

    Write-Step "installing $name ($id)"

    $wingetArgs = @(
        'install', '--exact', '--id', $id, '--source', 'winget',
        '--accept-package-agreements', '--accept-source-agreements'
    )
    if ($Package.ContainsKey('Extra')) { $wingetArgs += $Package.Extra }

    # Out-Host, not a bare call: inside a function a native command's stdout
    # joins the return value, and the caller would then test the truth of an
    # array of winget's chatter instead of the $false below.
    & winget @wingetArgs | Out-Host
    if ($LASTEXITCODE -ne 0) {
        Write-Fail "winget failed to install $name (exit code $LASTEXITCODE)."
        Write-Note "if it says no package matched, the id '$id' is wrong or gone;"
        Write-Note "find the current one with: winget search $name"
        return $false
    }

    return $true
}

# ---------------------------------------------------------------------------
# Checks before touching anything
# ---------------------------------------------------------------------------

# Printed before anything else runs, so that silence afterwards means a command
# is stuck rather than that the script never started.
Write-Step "Collomatique Windows dev tools"

if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
    Write-Fail "winget is not available."
    Write-Note "it ships with Windows 11 as part of App Installer; install or"
    Write-Note "update that from the Microsoft Store, then run this again."
    exit 1
}

$identity  = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Note "not running as administrator. Build Tools will raise a UAC prompt,"
    Write-Note "and cloning vcpkg into $VcpkgRoot will fail outright, because Windows"
    Write-Note "reserves the root of C: for administrators. Re-run this elevated."
    Write-Host
}

# ---------------------------------------------------------------------------
# Install
# ---------------------------------------------------------------------------

Write-Step "asking winget what is already installed"

$failed = @()
foreach ($package in $Packages) {
    if (-not (Install-Package -Package $package)) { $failed += $package.Name }
    Write-Host
}

Update-SessionPath

# Everything below assumes the compiler exists, and winget's success for Build
# Tools has already been seen to mean "the installer started". --wait in the
# override should make it mean "the installer finished"; this asks the machine
# rather than trusting that, and stops here instead of failing in rustup.
if (-not (Get-MsvcPath)) {
    Write-Fail "the MSVC toolset is not there, so there is no point going on."
    Write-Note "if Visual Studio Installer is still running, let it finish and"
    Write-Note "run this script again -- it skips what is already installed."
    exit 1
}

# ---------------------------------------------------------------------------
# vcpkg
# ---------------------------------------------------------------------------

# The clone is not shallow: a manifest's builtin-baseline names a commit that
# has to be findable in this checkout's history.
#
# An existing checkout is bootstrapped but never pulled -- moving the registry
# forward is a deliberate act, not something a re-run should do on its own.

$vcpkgExe = Join-Path $VcpkgRoot 'vcpkg.exe'
$vcpkgOk  = $false

if (Test-Path $vcpkgExe) {
    Write-Step "vcpkg is already bootstrapped in $VcpkgRoot, skipping"
    $vcpkgOk = $true
} elseif (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    Write-Fail "git is not in PATH, so vcpkg cannot be cloned."
    Write-Note "open a new terminal and run this script again."
} else {
    $cloned = $true

    if (Test-Path $VcpkgRoot) {
        Write-Step "reusing the checkout already in $VcpkgRoot"
    } else {
        Write-Step "cloning vcpkg into $VcpkgRoot (full history, so it is not quick)"
        & git clone $VcpkgRepository $VcpkgRoot
        if ($LASTEXITCODE -ne 0) {
            Write-Fail "git clone failed (exit code $LASTEXITCODE)."
            Write-Note "if it could not create the directory: Windows reserves the root"
            Write-Note "of C: for administrators. Run this elevated, or change"
            Write-Note "`$VcpkgRoot at the top of this script to a path you own."
            $cloned = $false
        }
    }

    if ($cloned) {
        Write-Step "bootstrapping vcpkg"
        & (Join-Path $VcpkgRoot 'bootstrap-vcpkg.bat') -disableMetrics
        if ($LASTEXITCODE -eq 0) {
            $vcpkgOk = $true
        } else {
            Write-Fail "bootstrap-vcpkg.bat failed (exit code $LASTEXITCODE)."
        }
    }
}

if ($vcpkgOk) {
    # Everything that consumes vcpkg looks for this variable, so setting it
    # spares the build script from naming the path a second time. User scope,
    # which needs no administrator rights.
    $env:VCPKG_ROOT = $VcpkgRoot
    if ([Environment]::GetEnvironmentVariable('VCPKG_ROOT', 'User') -ne $VcpkgRoot) {
        Write-Step "setting VCPKG_ROOT to $VcpkgRoot for your user account"
        [Environment]::SetEnvironmentVariable('VCPKG_ROOT', $VcpkgRoot, 'User')
    }
} else {
    $failed += 'vcpkg'
}
Write-Host

# ---------------------------------------------------------------------------
# Rust toolchain
# ---------------------------------------------------------------------------

# The winget package installs rustup itself; the toolchain is a second step.
# `rustup default` installs the toolchain if it is missing and is a no-op once
# it is there, so this is the whole of it.
if (Get-Command rustup -ErrorAction SilentlyContinue) {
    Write-Step "selecting Rust toolchain $RustToolchain"
    & rustup default $RustToolchain
    if ($LASTEXITCODE -ne 0) {
        Write-Fail "rustup could not install $RustToolchain (exit code $LASTEXITCODE)."
        $failed += 'Rust toolchain'
    }
} else {
    Write-Fail "rustup is not in PATH even after installing it."
    Write-Note "open a new terminal and run: rustup default $RustToolchain"
    $failed += 'Rust toolchain'
}
Write-Host

# ---------------------------------------------------------------------------
# What actually landed
# ---------------------------------------------------------------------------
#
# Version numbers rather than a bare "ok", so that a screenshot of this block
# records the environment a given build came out of.

Write-Step "installed versions"

# This whole block only reports; a tool that writes a warning to stderr while
# printing its version must not abort the script the way 'Stop' would make it.
$ErrorActionPreference = 'Continue'

# cl.exe is not on PATH until vcvars64.bat has run, so Build Tools is reported
# through vswhere rather than by calling the compiler.
$msvc = Get-MsvcPath
if ($msvc) {
    Write-Note "MSVC toolset: $msvc"
} else {
    Write-Note "MSVC toolset: NOT FOUND"
}

# vcpkg is checked by path, not through PATH: bootstrap-vcpkg.bat leaves
# vcpkg.exe in the checkout and puts nothing on PATH. VCPKG_ROOT is how the
# build script will find it.
if (Test-Path $vcpkgExe) {
    Write-Note "vcpkg: $(& $vcpkgExe version 2>$null | Select-Object -First 1)"
    Write-Note "       VCPKG_ROOT=$VcpkgRoot"
} else {
    Write-Note "vcpkg: NOT FOUND at $vcpkgExe"
}

foreach ($probe in @(
    @{ Label = 'git';    Command = 'git';    Arguments = @('--version') }
    @{ Label = 'rustup'; Command = 'rustup'; Arguments = @('--version') }
    @{ Label = 'rustc';  Command = 'rustc';  Arguments = @('--version') }
)) {
    if (Get-Command $probe.Command -ErrorAction SilentlyContinue) {
        $probeArgs = $probe.Arguments
        $output = (& $probe.Command @probeArgs 2>$null | Select-Object -First 1)
        Write-Note "$($probe.Label): $output"
    } else {
        Write-Note "$($probe.Label): NOT FOUND in PATH"
    }
}

# ISCC.exe is the Inno Setup command line compiler. Its installer does not put
# it on PATH, so the build script will have to name this path; find it here
# rather than discover it is elsewhere at the end of a build.
#
# Which of the two Program Files directories Inno Setup 7 installs into is not
# known here, so both are searched and the one found is printed.
$isccCandidates = @(
    (Join-Path $env:ProgramFiles        'Inno Setup 7\ISCC.exe')
    (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 7\ISCC.exe')
)
$iscc = $isccCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if ($iscc) {
    Write-Note "Inno Setup: $iscc"
} else {
    Write-Note "Inno Setup: NOT FOUND, looked in:"
    foreach ($candidate in $isccCandidates) { Write-Note "  $candidate" }
}

Write-Host

if ($failed.Count -gt 0) {
    Write-Fail "these did not install: $($failed -join ', ')"
    exit 1
}

Write-Step "done. Open a new terminal so PATH is picked up, then snapshot the VM."
