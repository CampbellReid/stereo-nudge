#ifndef AppVersion
  #define AppVersion "0.3.0"
#endif
#ifndef BinaryPath
  #define BinaryPath "..\target\release\stereo-nudge.exe"
#endif
[Setup]
AppId={{9A4919CB-90E6-4DA3-BAB0-490E1971952D}
AppName=Stereo Nudge
AppVersion={#AppVersion}
AppPublisher=Campbell Reid
AppPublisherURL=https://github.com/CampbellReid/stereo-nudge
AppSupportURL=https://github.com/CampbellReid/stereo-nudge/issues
AppUpdatesURL=https://github.com/CampbellReid/stereo-nudge/releases/latest
AppCopyright=Copyright (C) 2026 Campbell Reid
VersionInfoCompany=Campbell Reid
VersionInfoDescription=Stereo Nudge installer
DefaultDirName={localappdata}\Programs\StereoNudge
DefaultGroupName=Stereo Nudge
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableWelcomePage=no
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
Source: "..\dist\legal\THIRD-PARTY-NOTICES.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\legal\rust\*"; DestDir: "{app}\licenses\rust"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "Project.url"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\docs\*.md"; DestDir: "{app}\docs"; Flags: ignoreversion
[InstallDelete]
Type: files; Name: "{userstartup}\Stereo Nudge.lnk"
[Icons]
Name: "{userprograms}\Stereo Nudge\Stereo Nudge"; Filename: "{app}\stereo-nudge.exe"; Parameters: "--seconds 0 --startup-wait-seconds 60 --log-dir ""{localappdata}\StereoNudge\logs"""; WorkingDir: "{app}"
Name: "{userstartup}\Stereo Nudge"; Filename: "{app}\stereo-nudge.exe"; Parameters: "--seconds 0 --startup-wait-seconds 60 --log-dir ""{localappdata}\StereoNudge\logs"""; WorkingDir: "{app}"; Tasks: startup
Name: "{userprograms}\Stereo Nudge\Uninstall Stereo Nudge"; Filename: "{uninstallexe}"
Name: "{userprograms}\Stereo Nudge\Project on GitHub"; Filename: "{app}\Project.url"
[Run]
Filename: "{app}\stereo-nudge.exe"; Parameters: "--seconds 0 --startup-wait-seconds 60 --log-dir ""{localappdata}\StereoNudge\logs"""; Description: "Launch Stereo Nudge"; Flags: nowait postinstall skipifsilent
