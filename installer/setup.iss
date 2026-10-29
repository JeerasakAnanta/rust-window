; Build: iscc /DAppVersion=0.1.0 /DExeDir=<dir with rust-window.exe> /DOutDir=<dir> setup.iss
#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#ifndef ExeDir
  #define ExeDir "..\target\release"
#endif
#ifndef OutDir
  #define OutDir "..\dist"
#endif

[Setup]
; Never change AppId: it is how newer installers recognise and upgrade this install.
AppId={{8B1F6B1E-5C0A-4B7E-9E5D-2D6A4C9F7A31}
AppName=Rust Window
AppVersion={#AppVersion}
DefaultDirName={autopf}\RustWindow
PrivilegesRequired=lowest
DefaultGroupName=Rust Window
OutputDir={#OutDir}
OutputBaseFilename=rust-window-setup
Compression=lzma2
SolidCompression=yes
CloseApplications=yes
RestartApplications=yes
UninstallDisplayIcon={app}\rust-window.exe

[Files]
Source: "{#ExeDir}\rust-window.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Rust Window"; Filename: "{app}\rust-window.exe"
