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

; A per-user installation: into %LOCALAPPDATA%\Programs, with the Start-menu
; entry and the uninstall record under HKCU. No UAC prompt, which for a teacher
; on a school machine may be the difference between installing and not.
;
; The all-users option the roadmap wants is not here yet. It is one line
; (PrivilegesRequiredOverridesAllowed), but it also puts a UAC prompt in front of
; the first screen, and that is worth testing on its own rather than as a rider.
PrivilegesRequired=lowest
DefaultDirName={autopf}\Collomatique
DefaultGroupName=Collomatique

; 64-bit only, because everything staged is: the exe, the GTK stack, Python.
; "x64compatible" rather than "x64" so that Windows on ARM, which runs x64 code
; through emulation, is not refused for no reason.
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
Filename: "{app}\collomatique-gtk4.exe"; \
    Description: "{cm:LaunchProgram,Collomatique}"; \
    Flags: nowait postinstall skipifsilent
