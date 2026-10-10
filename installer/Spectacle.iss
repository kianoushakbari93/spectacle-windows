; Inno Setup 7 script for Spectacle.
;
; Build the release executable first, then compile this script:
;   cargo build --release
;   "%LOCALAPPDATA%\Programs\Inno Setup 7\ISCC.exe" installer\Spectacle.iss
; or run scripts\build.ps1, which does both and runs the tests.

#define AppName "Spectacle"
#define AppVersion "3.0"
#define AppFileVersion "3.0.0.0"
#define AppExe "Spectacle.exe"
#define AppPublisher "Kianoush Akbari"
#define AppUrl "https://github.com/kianoushakbari93/spectacle"
; Must match MAIN_WINDOW_CLASS in src/app.rs.
#define MainWindowClass "Spectacle.Main.7b0f2f53-3d2b-4d7e-9a43-6c3a5f8e2d11"

[Setup]
AppId={{8C4F3A62-1E8B-4B0E-9C6D-5F2A7D1E3B94}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppUrl}
AppSupportURL={#AppUrl}
AppCopyright=Copyright (c) 2017 Eric Czarny / 2026 Kianoush Akbari
VersionInfoVersion={#AppFileVersion}
VersionInfoProductVersion={#AppFileVersion}
VersionInfoDescription={#AppName} Setup
DefaultDirName={autopf}\{#AppName}
DisableProgramGroupPage=yes
UninstallDisplayName={#AppName}
UninstallDisplayIcon={app}\{#AppExe}
SetupIconFile=..\resources\spectacle.ico
WizardStyle=modern dynamic
; A 64-bit installer for the 64-bit application, installed to Program Files.
SetupArchitecture=x64
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Windows 10 version 2004 or later; Spectacle is designed for Windows 11.
MinVersion=10.0.19041
; Installing for all users keeps the executable in a folder only
; administrators can change. /CURRENTUSER on the command line installs for
; the current user only, without administrator rights.
PrivilegesRequired=admin
PrivilegesRequiredOverridesAllowed=commandline
; Fallback for running copies the [Code] section could not close.
CloseApplications=yes
RestartApplications=no
Compression=lzma2/ultra64
SolidCompression=yes
OutputDir=..\dist
OutputBaseFilename={#AppName}-{#AppVersion}-Setup

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "finnish"; MessagesFile: "compiler:Languages\Finnish.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
Name: "portuguese"; MessagesFile: "compiler:Languages\Portuguese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\x86_64-pc-windows-msvc\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE.md"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
; Spectacle runs as the person signed in, never with Setup's administrator rights.
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent runasoriginaluser
; A silent update reopens Spectacle if it was running before.
Filename: "{app}\{#AppExe}"; Flags: nowait runasoriginaluser; Check: ShouldRelaunchAfterSilentUpdate

[Code]
const
  WM_CLOSE = $0010;
  RunKey = 'Software\Microsoft\Windows\CurrentVersion\Run';
  StartupApprovedKey = 'Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run';

var
  WasRunning: Boolean;

function IsSpectacleRunning(): Boolean;
begin
  Result := FindWindowByClassName('{#MainWindowClass}') <> 0;
end;

{ Asks a running Spectacle to quit, as its Quit menu item does, and waits for
  it to finish. }
procedure CloseSpectacle();
var
  Attempts: Integer;
begin
  if not IsSpectacleRunning() then
    Exit;
  PostMessage(FindWindowByClassName('{#MainWindowClass}'), WM_CLOSE, 0, 0);
  Attempts := 0;
  while IsSpectacleRunning() and (Attempts < 50) do
  begin
    Sleep(100);
    Attempts := Attempts + 1;
  end;
  { Let the process exit after its window is gone. }
  Sleep(300);
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  WasRunning := IsSpectacleRunning();
  CloseSpectacle();
  Result := '';
end;

function ShouldRelaunchAfterSilentUpdate(): Boolean;
begin
  Result := WizardSilent() and WasRunning;
end;

function InitializeUninstall(): Boolean;
begin
  CloseSpectacle();
  Result := True;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
  begin
    { Stop launching at sign-in. Settings in %APPDATA%\Spectacle are kept,
      as Windows apps conventionally do. }
    RegDeleteValue(HKCU, RunKey, 'Spectacle');
    RegDeleteValue(HKCU, StartupApprovedKey, 'Spectacle');
  end;
end;
