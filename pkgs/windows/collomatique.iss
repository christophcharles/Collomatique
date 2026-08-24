; Inno Setup script for Collomatique.
;
; This is not run by hand. build.ps1 compiles it with ISCC.exe and passes in
; every path and version it needs, so nothing here is written down twice:
;
;     ISCC.exe /DAppVersion=... /DVersionInfo=... /DStageDir=... /DOutputDir=...
;              /DIconFile=... pkgs\windows\collomatique.iss
;
; What it does: copy the staged directory, put an entry in the Start menu,
; register an uninstaller, and make Explorer open .collomatique files with
; Collomatique.
;
; Nothing here touches the bundled Python, and that is now a finished answer
; rather than a postponed one. pip and XlsxWriter are already unpacked inside
; the staged directory, so they arrive as ordinary files with everything else --
; nothing to download here, nothing to run, nothing that can fail on a teacher's
; machine. A module a teacher installs later goes to their own
; %APPDATA%\collomatique\python, which build.ps1 stages the hook for; see
; pkgs\windows\site\collomatique_site.py. The only trace of any of it below is
; the [UninstallDelete] section at the bottom.

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
#ifndef IconFile
  #error IconFile is not defined -- compile this through pkgs\windows\build.ps1
#endif

; The extension, its ProgID, and the name Explorer shows in its "Type" column.
; Written once here because they are spread over the registry entries at the
; bottom, and getting two of them to disagree is the classic way to register an
; association that quietly does nothing.
#define ExtName  ".collomatique"
#define ProgId   "Collomatique.Document"
#define TypeName "Colloscope Collomatique"

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

; The only two places the icon has to be named. Everywhere else it is inherited:
; the Start-menu shortcut and the file association both point at the executable,
; and the executable carries the same icon compiled in by colloscopes/gtk4/build.rs.
;
;   SetupIconFile         -- setup.exe itself, before anything is installed, so
;                            it is the one place that cannot read it off the
;                            executable and needs the .ico directly.
;   UninstallDisplayIcon  -- beside Collomatique in Windows' "Installed apps".
SetupIconFile={#IconFile}
UninstallDisplayIcon={app}\collomatique-gtk4.exe

; Tell Explorer its association cache is stale. Without it the new file type
; keeps its blank page icon until the next sign-in.
ChangesAssociations=yes

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

[Registry]
; Double-clicking a .collomatique file opens it in Collomatique.
;
; Root HKA is the counterpart of the install-mode dialog: HKLM after an all-users
; install, HKCU after a per-user one. Writing HKLM unconditionally is what makes
; an association fail silently for a teacher without administrator rights.
;
; Windows wants two halves. The extension key names a ProgID, and the ProgID
; carries what to do with the file. Neither is any use without the other, which
; is why both spellings come from the same #define above.

; The extension -> the ProgID. Its own key is removed on uninstall only if
; nothing else is left in it, so an association some other program added later
; is not taken down with ours.
Root: HKA; Subkey: "Software\Classes\{#ExtName}"; \
    ValueType: string; ValueName: ""; ValueData: "{#ProgId}"; \
    Flags: uninsdeletevalue uninsdeletekeyifempty

; The same thing again, in the list Windows reads to build the "Open with" menu.
; The default value above decides what a double-click does; this decides whether
; Collomatique is offered at all when a teacher goes looking.
Root: HKA; Subkey: "Software\Classes\{#ExtName}\OpenWithProgids"; \
    ValueType: string; ValueName: "{#ProgId}"; ValueData: ""; \
    Flags: uninsdeletevalue

; The ProgID: the name Explorer shows in its "Type" column, the icon it draws on
; the file, and the command a double-click runs. This subtree is ours alone, so
; uninstalling removes all of it.
Root: HKA; Subkey: "Software\Classes\{#ProgId}"; \
    ValueType: string; ValueName: ""; ValueData: "{#TypeName}"; \
    Flags: uninsdeletekey

; ",0" is the first icon resource in the executable, which is the one and only
; one colloscopes/gtk4/build.rs puts there. Files get the application's icon because there is
; no separate document artwork -- the same choice the flatpak makes in its
; mime.xml.
Root: HKA; Subkey: "Software\Classes\{#ProgId}\DefaultIcon"; \
    ValueType: string; ValueName: ""; ValueData: "{app}\collomatique-gtk4.exe,0"

; The quoting is not decoration. Without the inner quotes any path containing a
; space -- "C:\Program Files\Collomatique", for one, and every teacher's
; "Mes documents" for another -- arrives at the application split into pieces.
Root: HKA; Subkey: "Software\Classes\{#ProgId}\shell\open\command"; \
    ValueType: string; ValueName: ""; \
    ValueData: """{app}\collomatique-gtk4.exe"" ""%1"""

[Run]
; A "run Collomatique now" checkbox on the last page, ticked to start with.
; "nowait" so setup.exe finishes instead of waiting for the application to be
; closed; "skipifsilent" because a silent install has no last page to offer it on.
;
; The launch goes through explorer.exe rather than running the executable
; directly, so that it starts exactly as if the teacher had clicked the
; Start-menu entry. The spawned explorer hands the request to the logged-in
; user's existing shell, and the shell does the launching. That buys two
; things at once:
;
;   the right account    -- after an all-users install, Setup is elevated,
;                           possibly under a different account entirely. The
;                           shell launches in the teacher's own session, so the
;                           first session's settings land in the teacher's
;                           profile. (postinstall's implied runasoriginaluser
;                           used to be what guaranteed this; the shell route
;                           keeps the guarantee.)
;   the foreground       -- Windows only lets a window come to the front if its
;                           process was started by the foreground process. The
;                           de-elevated helper that runasoriginaluser launches
;                           through is not it, so a direct launch left the main
;                           window behind Explorer, its present() refused. A
;                           launch by the shell holds the same foreground
;                           rights as a Start-menu click.
Filename: "{win}\explorer.exe"; \
    Parameters: """{app}\collomatique-gtk4.exe"""; \
    Description: "{cm:LaunchProgram,Collomatique}"; \
    Flags: nowait postinstall skipifsilent

[UninstallDelete]
; Everything shipped here is recorded by [Files] above and removed with it. This
; is for what pip may write afterwards: a module installed without --prefix
; lands in the application's own directories, and Setup has no record of it.
; That is possible after a "me only" install, where {app} is writable without
; administrator rights.
;
; Named precisely rather than sweeping {app}\Lib. Windows filenames are
; case-insensitive, so Python's Lib\ and glib's lib\ are one directory there,
; and taking it whole would take the GTK stack with it.
Type: filesandordirs; Name: "{app}\Lib\site-packages"
Type: filesandordirs; Name: "{app}\Scripts"

; Deliberately not listed: %APPDATA%\collomatique\python. Those are the
; teacher's own modules, and surviving is the whole point of putting them there.
; An update is an uninstall followed by an install, so removing them here would
; undo the thing that directory exists for.
