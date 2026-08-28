# Install the Windows development tools Collomatique is built with.
#
#     powershell -ExecutionPolicy Bypass -File pkgs\windows\bootstrap.ps1
#
# Run it once in a fresh Windows 11 VM, then take a snapshot: that snapshot is
# the real build environment, and this script is the recipe for recreating one.
# Running it again on a machine that already has the tools is safe; it skips
# what is already installed and reports what it found.
#
# Eight tools, and that is the whole list:
#
#   VS Build Tools   the MSVC compiler and the Windows SDK. No IDE. Using it
#                    requires a valid Visual Studio licence; Visual Studio
#                    Community grants one, free, to an individual developer.
#   git              needed to get vcpkg, which is a git checkout.
#   vcpkg            CBC and the Python the application embeds come from here,
#                    built from source. That Python is not a tool: the app
#                    links against libpython, so it is a build dependency like
#                    the others, and taking it from vcpkg means the interpreter
#                    we link and the interpreter we ship are the same one.
#                    winget has no vcpkg package, so this one is cloned and
#                    bootstrapped instead. See the clone section below.
#   Python 3.12      a second Python, and this one *is* a tool: gvsbuild is
#                    written in Python. Nothing links against this one and
#                    nothing ships it.
#   uv               how gvsbuild's documentation installs gvsbuild, and how it
#                    runs it. Nothing else here uses it. It is on the list
#                    because we took gvsbuild for being the route GTK blesses,
#                    and the install and run instructions are part of that
#                    route: doing it our own way with pip would put us back on
#                    a path nobody upstream tests.
#   MSYS2            a build shell, not a compiler. Parts of the GTK stack are
#                    driven by shell scripts, and gvsbuild runs them through
#                    MSYS2's bash. What comes out is still MSVC's code: a
#                    mingw toolchain would be a mingw-w64-* package, and none
#                    is installed here.
#   rustup           the Rust toolchain, tracking stable. Deliberately not
#                    pinned to a version: Cargo.lock already pins every
#                    dependency, and the flatpak tracks rust-stable the same
#                    way, so all platforms build with roughly the same compiler.
#   Inno Setup 7     compiles the installer at the end of the build. The current
#                    line, rather than the 6 that winget also still offers.
#
# GTK and libadwaita are the reason the list grew to seven. vcpkg cannot build
# them for MSVC -- libadwaita needs appstream, appstream needs libxmlb, and
# vcpkg's libxmlb port declares "supports": "!windows | mingw", so it is refused
# before anything compiles. gtk.org names MSYS2 and gvsbuild for Windows and
# never vcpkg, so gvsbuild is the path taken, and Python and MSYS2 are what it
# needs to run.
#
# gvsbuild also publishes prebuilt zips, which would have kept the list at five.
# Its own README argues against them for anything distributed: they are the raw
# output of a CI run, and the project cannot promise timely updates even for
# security issues. We ship an installer to teachers, and that zip carries zlib,
# libpng, freetype and harfbuzz -- so a CVE in any of them would be ours to fix
# and we would have no way to. Building the stack ourselves is what buys that
# back.
#
# This script installs tools. It does not build anything: no vcpkg package and
# no GTK. gvsbuild is the one thing here that is pinned to a version rather than
# taken as it comes, because it is a tool that decides what the GTK stack is;
# the rest of the pinning lives in the build script, next to what it pins.
#
# Beyond the tools themselves it leaves two lasting marks on the machine: the
# VCPKG_ROOT environment variable, and one root certificate fetched into the
# Windows certificate store -- see the last section for why that is needed.

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
# only, so the ARM64 cross compiler is a couple of gigabytes bought for nothing.
# Add it back here if that ever changes. ARM64, code signing, a CI build and
# distribution channels are all deliberately out of scope.
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
        # The Python that runs gvsbuild, not the one the application embeds --
        # that one is a vcpkg port and this script does not install it.
        #
        # gvsbuild asks for 3.10 or newer and excludes exactly 3.13.4. 3.12 sits
        # inside that with nothing to think about. winget's ids carry the minor
        # version, so moving to 3.13 later is an edit here; any version in range
        # would do, because nothing links against this one.
        #
        # --scope machine picks the manifest's machine installer, which passes
        # InstallAllUsers=1 PrependPath=1, so python lands on the machine PATH
        # instead of in one account.
        Id    = 'Python.Python.3.12'
        Name  = 'Python 3.12 (for gvsbuild)'
        Extra = @('--scope', 'machine')
    }
    @{
        # gvsbuild's documentation installs uv with exactly this, and no scope:
        #     winget install --id=astral-sh.uv -e
        # No --scope machine here, unlike the others. uv keeps the tools it
        # installs under the profile of whoever runs it, so a machine-wide uv
        # would not make a machine-wide gvsbuild anyway.
        Id   = 'astral-sh.uv'
        Name = 'uv'
    }
    @{
        # The build shell gvsbuild drives the GTK stack through. Its manifest
        # installs into C:\msys64 either way -- --scope machine only adds
        # AllUsers=true, which is what a build tool wants.
        #
        # No pacman packages are installed. gvsbuild's documentation names msys2
        # as a prerequisite with no package list, and says it downloads the other
        # tools it needs itself. If a project turns out to want more, the error
        # names the missing command and it is a pacman line here.
        Id    = 'MSYS2.MSYS2'
        Name  = 'MSYS2'
        Extra = @('--scope', 'machine')
    }
    @{
        Id   = 'Rustlang.Rustup'
        Name = 'rustup'
    }
    @{
        # The unsuffixed JRSoftware.InnoSetup id is the older 6.x line. The
        # suffix is the version, so a move to 8 later is an edit here.
        #
        # --scope machine because winget's default put it under LOCALAPPDATA,
        # and a build tool belongs on the machine, not in one account.
        Id    = 'JRSoftware.InnoSetup.7'
        Name  = 'Inno Setup 7'
        Extra = @('--scope', 'machine')
    }
)

# vcpkg is not in winget, so it is cloned from its own repository, which is the
# way its documentation describes anyway.
#
# A short root, on C: rather than under the user profile: vcpkg builds from
# source and those trees nest deeply, against a 260-character path limit.
$VcpkgRoot = 'C:\vcpkg'
$VcpkgRepository = 'https://github.com/microsoft/vcpkg.git'

# Where the MSYS2 manifest puts it, for the report at the end. Nothing is
# installed here; gvsbuild finds MSYS2 by searching the usual locations, of
# which this is the first.
$MsysRoot = 'C:\msys64'

# The host triple is named rather than left to rustup's autodetection, so the
# script says out loud which toolchain the build expects. msvc, not gnu: every
# C and C++ library the app links, vcpkg's and gvsbuild's alike, is built with
# the Build Tools above.
$RustToolchain = 'stable-x86_64-pc-windows-msvc'

# The tool that builds GTK and libadwaita. Pinned, unlike the bare command in
# gvsbuild's documentation: everything else in this build is pinned, and a tool
# that decides what the whole GTK stack is has no business moving on its own.
$GvsbuildVersion = '2026.8.0'

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

# Install-Package reports through this rather than by returning, because it must
# call winget with no pipeline and no redirection at all: a native command's own
# output would otherwise join the function's return value.
#
# The pipeline is what has to go. `winget install | Out-Host` was seen to sit
# there for good after winget had printed "Successfully installed" and exited,
# with no winget process left; a pipeline ends when every handle on its input is
# closed, and the Visual Studio installer leaves processes behind which had
# inherited one. Called bare, winget writes to the console directly and there is
# no pipe to wait on.
$script:InstallOk = $false

function Install-Package {
    param([hashtable]$Package)

    $script:InstallOk = $false

    $id   = $Package.Id
    $name = $Package.Name

    # Announced before the query, not after: the query is silent and can take a
    # while, so without this a slow one is indistinguishable from a hang.
    Write-Step "checking $name"

    # A package can be registered with winget and still not be usable -- an
    # interrupted Visual Studio install leaves exactly that. Where an entry
    # carries a Verify block, it, and not winget's list, decides whether there
    # is anything to do.
    if (Test-PackageInstalled -Id $id) {
        if ((-not $Package.ContainsKey('Verify')) -or (& $Package.Verify)) {
            Write-Step "$name is already installed, skipping"
            if ($Package.ContainsKey('Extra')) {
                Write-Note "to change how it was installed, re-run winget by hand:"
                Write-Note "  winget install --exact --id $id --source winget $($Package.Extra -join ' ')"
            }
            $script:InstallOk = $true
            return
        }
        Write-Step "$name is registered but incomplete, installing it again"
    }

    Write-Step "installing $name ($id)"

    $wingetArgs = @(
        'install', '--exact', '--id', $id, '--source', 'winget',
        '--accept-package-agreements', '--accept-source-agreements'
    )
    if ($Package.ContainsKey('Extra')) { $wingetArgs += $Package.Extra }

    & winget @wingetArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Fail "winget failed to install $name (exit code $LASTEXITCODE)."
        Write-Note "if it says no package matched, the id '$id' is wrong or gone;"
        Write-Note "find the current one with: winget search $name"
        return
    }

    $script:InstallOk = $true
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
    Write-Note "the --scope machine installs below will be refused, and cloning vcpkg"
    Write-Note "into $VcpkgRoot will fail outright, because Windows reserves the root"
    Write-Note "of C: for administrators. Re-run this elevated."
    Write-Host
}

# ---------------------------------------------------------------------------
# Install
# ---------------------------------------------------------------------------

Write-Step "asking winget what is already installed"

$failed = @()
foreach ($package in $Packages) {
    Install-Package -Package $package
    if (-not $script:InstallOk) { $failed += $package.Name }
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
# gvsbuild
# ---------------------------------------------------------------------------

# Installed the way gvsbuild's own documentation installs it, with uv rather than
# pip. gvsbuild was chosen for being the route GTK blesses, and the install
# instructions are part of that route.
#
# uv puts its tools under the profile of whoever runs it; there is no scope
# switch. Elevating a terminal keeps the same profile, so running this script as
# administrator still installs gvsbuild into your own account -- unless you
# elevate into a *different* administrator account, in which case build.ps1 will
# not find it.
#
# `uv tool install` is a no-op when the version asked for is already installed,
# so re-running this script costs nothing here.
if (Get-Command uv -ErrorAction SilentlyContinue) {
    Write-Step "installing gvsbuild $GvsbuildVersion with uv"
    & uv tool install "gvsbuild==$GvsbuildVersion"
    if ($LASTEXITCODE -ne 0) {
        Write-Fail "uv could not install gvsbuild==$GvsbuildVersion (exit code $LASTEXITCODE)."
        Write-Note "gvsbuild needs Python 3.10 or newer and refuses exactly 3.13.4."
        $failed += 'gvsbuild'
    } else {
        # uv puts tool executables in %USERPROFILE%\.local\bin, which Windows does
        # not have on PATH. Until this has run, `uv run gvsbuild` reports gvsbuild
        # not found even though `uv tool list` shows it installed. A no-op once the
        # directory is there.
        Write-Step "putting uv's tool directory on PATH"
        & uv tool update-shell
        if ($LASTEXITCODE -ne 0) {
            Write-Fail "uv tool update-shell failed (exit code $LASTEXITCODE)."
            Write-Note "without it, build.ps1 will not find gvsbuild. Run it by hand."
            $failed += 'gvsbuild'
        }
    }
} else {
    Write-Fail "uv is not in PATH even after installing it."
    Write-Note "open a new terminal and run:"
    Write-Note "  uv tool install gvsbuild==$GvsbuildVersion"
    Write-Note "  uv tool update-shell"
    $failed += 'gvsbuild'
}
Write-Host

# ---------------------------------------------------------------------------
# Certificate store
# ---------------------------------------------------------------------------

# gvsbuild builds librsvg, which is a dependency of gtk4 and is written in Rust,
# so it downloads its own cargo from win.rustup.rs. On a fresh Windows install
# that download fails:
#
#     CERTIFICATE_VERIFY_FAILED: unable to get local issuer certificate
#
# Windows seeds its root certificate store lazily -- a root arrives only once
# something has asked for it. Python does not trigger that; it reads whatever is
# already there. And gvsbuild fails hard rather than falling back, because the
# rustup installer has no published hash and gvsbuild will not fetch an
# unverifiable file over an unverified connection. That refusal is correct.
#
# One request through a Windows HTTP client is enough. curl.exe goes through
# schannel, which does trigger the root update, and the root stays in the store
# afterwards. The response is discarded; only the side effect is wanted.
#
# Written as fetch-and-discard rather than check-then-fetch: asking whether a
# particular root is present is far more work than downloading a few hundred
# kilobytes, and repeating it costs nothing.
#
# curl, not curl.exe, would be PowerShell 5.1's alias for Invoke-WebRequest.

$CertWarmUrl = 'https://win.rustup.rs/x86_64'

Write-Step "warming the certificate store for $CertWarmUrl"
if (Get-Command curl.exe -ErrorAction SilentlyContinue) {
    & curl.exe --silent --show-error --location --output NUL $CertWarmUrl
    if ($LASTEXITCODE -eq 0) {
        Write-Note "done. gvsbuild can verify that host now."
    } else {
        # Deliberately not counted as a failed install: a network blip here would
        # otherwise make this script exit reporting a missing tool. It bites in
        # build.ps1, and that is where it will be understood.
        Write-Fail "curl.exe could not reach $CertWarmUrl (exit code $LASTEXITCODE)."
        Write-Note "gvsbuild's cargo download will fail until this succeeds once."
    }
} else {
    Write-Fail "curl.exe is not in PATH."
    Write-Note "it ships with Windows 10 1803 and later. Without it, fetch"
    Write-Note "$CertWarmUrl once with any Windows HTTP client before building."
    Write-Note "a browser will not do: Edge and Chrome carry their own root"
    Write-Note "stores, so succeeding there says nothing about the Windows one."
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

# MSYS2 puts nothing on PATH, and that is deliberate on its part: its bash
# belongs to gvsbuild, not to this shell. Checked by path, like vcpkg above.
$MsysBash = Join-Path $MsysRoot 'usr\bin\bash.exe'
if (Test-Path $MsysBash) {
    Write-Note "MSYS2: $MsysRoot"
} else {
    Write-Note "MSYS2: NOT FOUND at $MsysBash"
}

foreach ($probe in @(
    @{ Label = 'git';    Command = 'git';    Arguments = @('--version') }
    @{ Label = 'python'; Command = 'python'; Arguments = @('--version') }
    @{ Label = 'uv';     Command = 'uv';     Arguments = @('--version') }
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
# Only the machine-wide locations are searched, because the install above asks
# for --scope machine. A copy under LOCALAPPDATA is a per-user install that this
# script did not ask for, and finding it would hide that the scope was ignored.
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
