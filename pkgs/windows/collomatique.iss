; Inno Setup script for Collomatique.
;
; This is not run by hand. build.ps1 compiles it with ISCC.exe and passes in
; every path and version it needs, so nothing here is written down twice:
;
;     ISCC.exe /DAppVersion=... /DVersionInfo=... /DStageDir=... /DOutputDir=...
;              pkgs\windows\collomatique.iss
;
; What it does is deliberately the smallest thing that installs: copy the staged
; directory, put an entry in the Start menu, and register an uninstaller. It does
; not associate .collomatique files, does not write anything under
; Software\Classes, and does not touch the bundled Python. Those come later, one
; at a time, so that when one of them misbehaves it is obvious which one.

#ifndef AppVersion
  #error AppVersion is not defined -- compile this through pkgs\windows\build.ps1
#endif
#ifndef VersionInfo
  #error VersionInfo is not defined -- compile this through pkgs\windows\build.ps1
#endif
#ifndef StageDir
  #error StageDir is not defined -- compile this through pkgs\windows\build.ps1
#endif
#ifndef OutputDir
  #error OutputDir is not defined -- compile this through pkgs\windows\build.ps1
#endif

[Setup]
; A fixed identity. Inno recognises an existing installation by this GUID and
; nothing else, so it must never change: a new one would turn every future
; release into a second, parallel installation with its own uninstaller.
AppId={{71D89624-07A3-4B6C-A689-87B9A54EB2A6}
AppName=Collomatique
AppVersion={#AppVersion}
AppPublisher=Christoph Charles

; The version stamped into setup.exe's own file properties. It has to be plain
; numbers, which "0.1.0-alpha.1.99" is not, so build.ps1 works out the numeric
; part and passes both. AppVersion above is what the user is shown.
VersionInfoVersion={#VersionInfo}

; The user chooses, on a page shown before anything else happens: for all users,
; or for me only. Both answers work, and which one is picked decides the install
; directory as much as it decides the privileges.
;
;   all users  -- C:\Program Files\Collomatique, Start-menu entry and uninstall
;                 record machine-wide. Windows asks for administrator rights
;                 first: a yes/no for an administrator, a password for anyone
;                 else.
;   me only    -- %LOCALAPPDATA%\Programs\Collomatique, everything under HKCU,
;                 no prompt at all. That directory is inside a hidden folder,
;                 which is why this is not the default -- but it is the answer
;                 that still installs on a school machine where a teacher has no
;                 administrator rights.
;
; "admin" is the default the dialog offers, so the common case is one extra click
; and the familiar path. Inno shows the dialog before elevating, so choosing the
; second answer costs nothing.
;
; Only on a first install: UsePreviousPrivileges defaults to yes, so an upgrade
; finds the mode the previous install used and reuses it without asking again.
; (That default is only unsafe when AppId contains constants. Ours is a literal
; GUID.)
PrivilegesRequired=admin
PrivilegesRequiredOverridesAllowed=dialog

; Resolves per mode on its own: the machine-wide Program Files under "all users",
; the per-user one under "me only". The same is true of {group} below.
DefaultDirName={autopf}\Collomatique
DefaultGroupName=Collomatique

; 64-bit only, because everything staged is: the exe, the GTK stack, Python.
; "x64compatible" rather than "x64" so that Windows on ARM, which runs x64 code
; through emulation, is not refused for no reason.
;
; The second line is what makes an all-users install land in "C:\Program Files"
; rather than in "C:\Program Files (x86)": without it Setup runs in 32-bit mode
; and {autopf} resolves to the wrong one of the two.
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

; The staged tree is a few hundred megabytes of DLLs and Python source, which
; compresses well and compresses better in one solid block. It costs minutes at
; build time and is paid once.
Compression=lzma2/max
SolidCompression=yes

WizardStyle=modern
; The Start-menu folder is not worth a page of the wizard for a single entry.
DisableProgramGroupPage=yes
; The icon shown beside Collomatique in "Installed apps". The exe carries no icon
; resource of its own yet, so for now this is Windows' default application icon.
UninstallDisplayIcon={app}\collomatique-gtk4.exe

OutputDir={#OutputDir}
OutputBaseFilename=Collomatique-Setup-{#AppVersion}

[Languages]
; French first, so it is the default when Windows' own language is neither.
Name: "french";  MessagesFile: "compiler:Languages\French.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
; The whole staged directory, verbatim and recursively. Listing files instead
; would go stale the first time GTK changes what it pulls in -- and build.ps1
; already stages by the same rule, for the same reason.
;
; "ignoreversion" replaces a file whenever it differs, rather than comparing
; version resources. Without it an upgrade can keep an older DLL that happens to
; carry a higher version number than the one shipped beside it.
Source: "{#StageDir}\*"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs ignoreversion

[Icons]
Name: "{group}\Collomatique"; Filename: "{app}\collomatique-gtk4.exe"

[Run]
; A "run Collomatique now" checkbox on the last page, ticked to start with.
; "nowait" so setup.exe finishes instead of waiting for the application to be
; closed; "skipifsilent" because a silent install has no last page to offer it on.
;
; No "runasoriginaluser" flag, and it is wanted: "postinstall" already implies it.
; After an all-users install, Setup is elevated -- possibly under a different
; account entirely, if a password was typed -- and launching Collomatique from
; there would put the first session's settings in that account's profile rather
; than in the teacher's.
Filename: "{app}\collomatique-gtk4.exe"; \
    Description: "{cm:LaunchProgram,Collomatique}"; \
    Flags: nowait postinstall skipifsilent
