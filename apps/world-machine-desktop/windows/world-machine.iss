; The World Machine installer for Windows (Inno Setup 6).
;
; Built by package.sh, which stages the app in target\windows-package\stage
; and passes the version, the stage and the output folder:
;
;   ISCC.exe /DAppVersion=0.29.0 /DStageDir=... /DOutputDir=... world-machine.iss
;
; Per-user by default (no administrator prompt; the app keeps its own files
; under %APPDATA% and %LOCALAPPDATA% either way), with the option of all
; users. Unsigned until a code-signing certificate exists: see
; docs/RELEASE_SIGNING.md, "Windows". When one does, package.sh signs the
; app before this runs and the installer after it.

#ifndef AppVersion
  #error AppVersion is required (/DAppVersion=...)
#endif
#ifndef StageDir
  #error StageDir is required (/DStageDir=...)
#endif
#ifndef OutputDir
  #define OutputDir "."
#endif

[Setup]
; Never change AppId: it is how Windows knows a new version replaces the old.
AppId={{6B0B9C1E-3D4A-4C55-9E2F-57A1D0C4E8B2}
AppName=World Machine
AppVersion={#AppVersion}
AppVerName=World Machine {#AppVersion}
AppPublisher=World Machine
AppPublisherURL=https://github.com/hxddh/world-machine
AppSupportURL=https://github.com/hxddh/world-machine/issues
DefaultDirName={autopf}\World Machine
DefaultGroupName=World Machine
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
OutputDir={#OutputDir}
OutputBaseFilename=World-Machine-{#AppVersion}-Windows-x64-Setup
SetupIconFile={#StageDir}\World Machine.ico
UninstallDisplayIcon={app}\World Machine.ico
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
; The player's Worlds live under %APPDATA%\World Machine and are never
; removed by uninstalling.

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#StageDir}\World Machine.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#StageDir}\World Machine.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#StageDir}\World Packs\*"; DestDir: "{app}\World Packs"; Flags: ignoreversion recursesubdirs
Source: "{#StageDir}\READ ME FIRST.txt"; DestDir: "{app}"; Flags: ignoreversion isreadme
Source: "{#StageDir}\LICENSE.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\World Machine"; Filename: "{app}\World Machine.exe"; IconFilename: "{app}\World Machine.ico"
Name: "{autodesktop}\World Machine"; Filename: "{app}\World Machine.exe"; IconFilename: "{app}\World Machine.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\World Machine.exe"; Description: "{cm:LaunchProgram,World Machine}"; Flags: nowait postinstall skipifsilent
