#ifndef AppVersion
  #define AppVersion "0.2.0"
#endif
#ifndef BinaryPath
  #define BinaryPath "..\target\release\stereo-nudge.exe"
#endif
[Setup]
AppId={{9A4919CB-90E6-4DA3-BAB0-490E1971952D}
AppName=Stereo Nudge
AppVersion={#AppVersion}
AppPublisher=Stereo Nudge contributors
AppPublisherURL=https://github.com/CampbellReid/stereo-nudge
DefaultDirName={localappdata}\Programs\StereoNudge
DefaultGroupName=Stereo Nudge
DisableDirPage=yes
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.19041
OutputDir=..\dist
OutputBaseFilename=stereo-nudge-{#AppVersion}-windows-x64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayName=Stereo Nudge
UninstallDisplayIcon={app}\stereo-nudge.exe
CloseApplications=yes
CloseApplicationsFilter=stereo-nudge.exe
RestartApplications=no
SetupLogging=yes
LicenseFile=..\LICENSE
[Tasks]
Name: "startup"; Description: "Run Stereo Nudge when I sign in to Windows"; GroupDescription: "Startup:"; Flags: unchecked
[Files]
Source: "{#BinaryPath}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\docs\*.md"; DestDir: "{app}\docs"; Flags: ignoreversion
[InstallDelete]
Type: files; Name: "{userstartup}\Stereo Nudge.lnk"
[Icons]
Name: "{userprograms}\Stereo Nudge\Stereo Nudge"; Filename: "{app}\stereo-nudge.exe"; Parameters: "--seconds 0 --startup-wait-seconds 60 --log-dir ""{localappdata}\StereoNudge\logs"""; WorkingDir: "{app}"
Name: "{userstartup}\Stereo Nudge"; Filename: "{app}\stereo-nudge.exe"; Parameters: "--seconds 0 --startup-wait-seconds 60 --log-dir ""{localappdata}\StereoNudge\logs"""; WorkingDir: "{app}"; Tasks: startup
Name: "{userprograms}\Stereo Nudge\Uninstall Stereo Nudge"; Filename: "{uninstallexe}"
[Run]
Filename: "{app}\stereo-nudge.exe"; Parameters: "--seconds 0 --startup-wait-seconds 60 --log-dir ""{localappdata}\StereoNudge\logs"""; Description: "Launch Stereo Nudge"; Flags: nowait postinstall skipifsilent
