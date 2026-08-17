# Build Collomatique for Windows.
#
#     powershell -ExecutionPolicy Bypass -File pkgs\windows\build.ps1
#
# Run it from a terminal opened after pkgs\windows\bootstrap.ps1 has finished,
# so that VCPKG_ROOT is in the environment.
#
# What it does today is the dependencies only: CBC and Python are built with
# vcpkg from the manifest next to this file, GTK and libadwaita are downloaded
# ready-built, and then it reports what landed. It does not compile Collomatique
# itself yet. The rest of the build is added one piece at a time as the roadmap
# advances: the cargo build, the bundle layout, then the installer.
#
# The two halves are separate because GTK and libadwaita cannot come from vcpkg.
# libadwaita requires appstream, appstream requires libxmlb, and vcpkg's libxmlb
# port declares "supports": "!windows | mingw", so vcpkg refuses it for MSVC
# before compiling anything. The GTK project does not recommend vcpkg for GTK
# either -- gtk.org names MSYS2 and gvsbuild, and gvsbuild is the one that
# produces MSVC libraries. So the GTK stack comes from gvsbuild, and vcpkg keeps
# the rest. Everything ends up in two prefixes under -OutRoot.
#
# Interrupting the build is safe and re-running resumes: vcpkg keeps every
# package it has already built, and the GTK archive is kept once downloaded.
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

$ManifestDir = $PSScriptRoot
$TripletDir  = Join-Path $ManifestDir 'triplets'

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

# Absolute from here on. The .NET zip API further down resolves a relative path
# against the process working directory, and this script changes that.
$OutRoot = (Resolve-Path -Path $OutRoot).Path

$InstalledRoot = Join-Path $OutRoot 'vcpkg_installed'
$Prefix        = Join-Path $InstalledRoot $Triplet
$GtkPrefix     = Join-Path $OutRoot 'gtk'
$DownloadDir   = Join-Path $OutRoot 'downloads'

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
# GTK and libadwaita, from a gvsbuild release
# ---------------------------------------------------------------------------

# These are downloaded ready-built rather than compiled here. gvsbuild's CI
# builds them on every release and attaches the result:
#
#     - name: Build GTK4
#       run: >
#         uv run gvsbuild build --ninja-opts -j2 --enable-gi --py-wheel gtk4
#         libadwaita gtksourceview5 pygobject protobuf-c adwaita-icon-theme gtkmm
#     - name: Archive GTK runtime
#       run: 7z a -tzip GTK4_Gvsbuild_..._x64.zip C:\gtk-build\gtk\x64\release\*
#
# libadwaita is on that build line, and the archive is the whole install prefix
# with nothing filtered out -- headers and import libraries included, despite
# the step being named "runtime". Note that gvsbuild's README describes the
# release as containing fewer projects than this and does not mention libadwaita
# at all; the workflow is the one to believe.
#
# Taking the release rather than running gvsbuild keeps two large tools out of
# bootstrap.ps1 -- gvsbuild needs MSYS2 as a build shell and a Python of its own
# to run itself -- and turns hours of compiling into a download. It also pins
# harder: a release asset is one immutable file with a checksum, where a
# gvsbuild git ref would still rebuild from whatever upstream tarballs it
# fetched that day.
#
# One thing the release does not carry: librsvg, which is not on the build line
# above. If symbolic icons come out missing when the application first runs,
# that is where to look, and fixing it means running gvsbuild after all.

$GtkRelease = '2026.8.0'
$GtkArchive = "GTK4_Gvsbuild_${GtkRelease}_x64.zip"
$GtkUrl     = "https://github.com/wingtk/gvsbuild/releases/download/$GtkRelease/$GtkArchive"

# Published beside the asset on the release page. Bump it together with
# $GtkRelease: a tag whose checksum does not match stops the build rather than
# quietly installing something nobody chose. Compared case-insensitively, which
# is PowerShell's default for -ne on strings.
$GtkSha256 = '1f95a92d037f5292da05e6ab1037032ff21ddb7b20d4ac8e83e3674c864c07b0'

# Written after the archive is unpacked, so an extraction interrupted halfway
# is not mistaken for a finished one on the next run.
$GtkStamp = Join-Path $GtkPrefix '.gvsbuild-release'

# -replace rather than .Trim(): an empty stamp file reads back as $null, and a
# method call on that would throw where this just compares unequal.
$gtkInstalled = $false
if (Test-Path $GtkStamp) {
    $stamped = (Get-Content -Path $GtkStamp -Raw) -replace '\s', ''
    $gtkInstalled = ($stamped -eq $GtkRelease)
}

if ($gtkInstalled) {
    Write-Step "GTK $GtkRelease is already unpacked in $GtkPrefix, skipping"
} else {
    if (-not (Test-Path $DownloadDir)) {
        $null = New-Item -ItemType Directory -Path $DownloadDir
    }

    $GtkArchivePath = Join-Path $DownloadDir $GtkArchive

    if (Test-Path $GtkArchivePath) {
        Write-Step "reusing $GtkArchive from $DownloadDir"
    } else {
        Write-Step "downloading $GtkArchive (a few hundred megabytes, and silent)"

        # The progress bar is turned off because on Windows PowerShell 5.1 it
        # redraws on every read and can slow a large download by an order of
        # magnitude. That is what "and silent" above is warning about.
        #
        # Downloaded under .part and renamed once complete, so an interrupted
        # download is not picked up as a finished one by the branch above.
        $previousProgress = $ProgressPreference
        $ProgressPreference = 'SilentlyContinue'
        try {
            Invoke-WebRequest -Uri $GtkUrl -OutFile "$GtkArchivePath.part" -UseBasicParsing
        } finally {
            $ProgressPreference = $previousProgress
        }
        Move-Item -Path "$GtkArchivePath.part" -Destination $GtkArchivePath -Force
    }

    Write-Step "checking the archive against the checksum pinned in this script"
    $actualSha256 = (Get-FileHash -Path $GtkArchivePath -Algorithm SHA256).Hash
    if ($actualSha256 -ne $GtkSha256) {
        Write-Fail "$GtkArchive is not the file this script expects."
        Write-Note "expected SHA256: $GtkSha256"
        Write-Note "got:             $actualSha256"
        Write-Note "delete $GtkArchivePath and run this again -- a truncated download"
        Write-Note "looks exactly like this. If it fails a second time, the checksum"
        Write-Note "in this script no longer matches the release it names."
        exit 1
    }

    # A leftover from an interrupted run, or from a different release, is
    # removed rather than unpacked over: the two versions' files would mix.
    if (Test-Path $GtkPrefix) {
        Write-Step "removing the previous contents of $GtkPrefix"
        Remove-Item -Path $GtkPrefix -Recurse -Force
    }

    # ZipFile rather than Expand-Archive: this archive holds thousands of small
    # files, and Expand-Archive on Windows PowerShell 5.1 walks them through the
    # pipeline one at a time. ExtractToDirectory is the same .NET code without
    # that, and creates the destination itself.
    #
    # 7z was given the contents of the prefix (release\*) rather than the
    # directory, so bin\, lib\, include\ and share\ are at the top of the
    # archive and this lands them directly in $GtkPrefix.
    Write-Step "unpacking into $GtkPrefix"
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    try {
        [System.IO.Compression.ZipFile]::ExtractToDirectory($GtkArchivePath, $GtkPrefix)
    } catch {
        Write-Fail "unpacking $GtkArchive failed."
        Write-Note $_.Exception.Message
        Write-Note "if that mentions a path being too long: this archive nests deeply"
        Write-Note "and -OutRoot has to leave room under 260 characters. C:\gtk-build"
        Write-Note "is about as short as it gets."
        exit 1
    }

    Set-Content -Path $GtkStamp -Value $GtkRelease -Encoding ASCII
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
# what the gtk4 and libadwaita crates probe for. They sit in two different
# prefixes, which is why step 4 will put both directories on PKG_CONFIG_PATH.
#
# python3 is not looked for: pyo3 finds Python through an interpreter rather
# than pkg-config, so whether the port ships a .pc file says nothing either way.
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
# the gvsbuild prefix rather than the vcpkg one.
Show-Tool -Label 'glib-compile-resources' `
    -Path (Join-Path $GtkPrefix 'bin\glib-compile-resources.exe') `
    -Arguments @('--version')

# The pkg-config implementation the cargo build will be pointed at.
Show-Tool -Label 'pkgconf' `
    -Path (Join-Path $Prefix 'tools\pkgconf\pkgconf.exe') `
    -Arguments @('--version')

# The interpreter the application links against, and the one it will ship. A
# Python that exists to be linked against is not necessarily one a teacher can
# install packages into, which is why pip is reported and not assumed. The port
# ships pip as a bundled wheel rather than installed, so the pip line reads
# "printed nothing" until `python -m ensurepip --upgrade` has been run once.
# Whether the shipped bundle does that, and when, is a step-7 decision.
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
