#define MyAppName "CodexBar"
#ifndef AppVersion
  #define AppVersion "0.0.0-dev"
#endif
#ifndef TargetBinDir
  #define TargetBinDir "..\\target\\x86_64-pc-windows-gnu\\release"
#endif
#ifndef OutputDir
  #define OutputDir "..\\target\\installer"
#endif
#ifndef OutputBaseFilename
  #define OutputBaseFilename "CodexBar-" + AppVersion + "-Setup"
#endif

[Setup]
AppId=WinCodexBar
AppName={#MyAppName}
AppVersion={#AppVersion}
AppVerName={#MyAppName} {#AppVersion}
AppPublisher=CodexBar Contributors
AppPublisherURL=https://github.com/Finesssee/Win-CodexBar
AppSupportURL=https://github.com/Finesssee/Win-CodexBar/issues
AppUpdatesURL=https://github.com/Finesssee/Win-CodexBar/releases
DefaultDirName={localappdata}\Programs\CodexBar
DefaultGroupName=CodexBar
DisableProgramGroupPage=yes
DisableDirPage=auto
PrivilegesRequired=lowest
UsePreviousAppDir=yes
CloseApplications=yes
WizardStyle=modern
Compression=lzma
SolidCompression=yes
OutputDir={#OutputDir}
OutputBaseFilename={#OutputBaseFilename}
SetupIconFile=..\icons\icon.ico
UninstallDisplayIcon={app}\codexbar.exe
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Files]
Source: "{#TargetBinDir}\codexbar.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#TargetBinDir}\gen_icons.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\icons\icon.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\CodexBar"; Filename: "{app}\codexbar.exe"; Parameters: "menubar"; WorkingDir: "{app}"; IconFilename: "{app}\icon.ico"
Name: "{autodesktop}\CodexBar"; Filename: "{app}\codexbar.exe"; Parameters: "menubar"; WorkingDir: "{app}"; Tasks: desktopicon; IconFilename: "{app}\icon.ico"

[Run]
Filename: "{app}\codexbar.exe"; Parameters: "menubar"; Description: "Launch CodexBar"; Flags: nowait postinstall skipifsilent
