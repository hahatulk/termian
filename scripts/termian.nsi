Unicode true
!include "FileFunc.nsh"

Name "Termian"
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\termian"
InstallDirRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\termian" "InstallLocation"
RequestExecutionLevel user
Icon "..\assets\app\icon.ico"
UninstallIcon "..\assets\app\icon.ico"
SetCompressor /SOLID lzma

Page directory
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles

Function .onInit
    SetShellVarContext current
    ${GetParameters} $R0
    ClearErrors
    ${GetOptions} $R0 "/P" $R1
    IfErrors +2
    SetSilent silent
FunctionEnd

Section "Termian"
    SetOutPath "$INSTDIR"
    File /oname=termian.exe "${BINARY}"
    WriteUninstaller "$INSTDIR\uninstall.exe"
    CreateShortcut "$SMPROGRAMS\Termian.lnk" "$INSTDIR\termian.exe"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\termian" "DisplayName" "Termian"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\termian" "DisplayVersion" "${VERSION}"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\termian" "InstallLocation" "$INSTDIR"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\termian" "UninstallString" '"$INSTDIR\uninstall.exe"'
SectionEnd

Function .onInstSuccess
    ${GetParameters} $R0
    ClearErrors
    ${GetOptions} $R0 "/R" $R1
    IfErrors +2
    ExecShell "open" "$INSTDIR\termian.exe"
FunctionEnd

Section "Uninstall"
    SetShellVarContext current
    Delete "$SMPROGRAMS\Termian.lnk"
    Delete "$INSTDIR\termian.exe"
    Delete "$INSTDIR\uninstall.exe"
    RMDir "$INSTDIR"
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\termian"
SectionEnd
