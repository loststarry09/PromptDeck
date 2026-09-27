; PromptDeck — Windows x64 installer (Inno Setup 6)
; Preview build. Built from the Windows mirror produced by scripts/build-windows.sh.
; Product icon is the existing project icon (ui/assets/app.ico, generated from ui/assets/app-icon.png).

#define AppName "PromptDeck"
#define AppVersion "0.1.0-preview.1"
#define AppPublisher "PromptDeck"
#define AppExeName "promptdeck.exe"

[Setup]
AppId={{7C3F2E9A-4B21-4D6E-9A1C-2F5B8D0E3A44}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
DefaultDirName={autopf}\{#AppName}
DisableDirPage=no
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\..\dist
OutputBaseFilename=PromptDeck-0.1.0-preview.1-windows-x64-setup
SetupIconFile=..\..\crates\promptdeck-app\ui\assets\app.ico
UninstallDisplayIcon={app}\{#AppExeName}
VersionInfoVersion=0.1.0.1
VersionInfoProductName={#AppName}
VersionInfoProductVersion=0.1.0.1
VersionInfoProductTextVersion={#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
SetupLogging=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "创建桌面快捷方式"; GroupDescription: "附加任务:"; Flags: unchecked

[Files]
Source: "..\..\target\x86_64-pc-windows-msvc\release\{#AppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\{#AppExeName}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExeName}"; Description: "启动 {#AppName}"; Flags: nowait postinstall skipifsilent

; NOTE: user Prompt data lives in %LOCALAPPDATA%\PromptDeck (ADR-0006), outside {app}.
; No [UninstallDelete] entry touches it, so uninstall preserves user data by design.
