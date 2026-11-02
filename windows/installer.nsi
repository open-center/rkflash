; RKFlash Windows 安装程序（NSIS，可在 Linux/macOS 上交叉生成）。
; 由 windows/Makefile 调用：makensis -DVERSION=x.y.z -DSTAGE=<目录> -DOUTFILE=<文件> installer.nsi

Unicode true
SetCompressor /SOLID lzma

!include "MUI2.nsh"
!include "x64.nsh"
!include "LogicLib.nsh"

!ifndef VERSION
  !error "需要 -DVERSION=x.y.z"
!endif
!ifndef STAGE
  !error "需要 -DSTAGE=<打包目录>"
!endif
!ifndef OUTFILE
  !error "需要 -DOUTFILE=<安装程序路径>"
!endif

!define PRODUCT "RKFlash"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT}"

Name "${PRODUCT} ${VERSION}"
Caption "${PRODUCT} ${VERSION} 安装"
OutFile "${OUTFILE}"
InstallDir "$PROGRAMFILES64\${PRODUCT}"
InstallDirRegKey HKLM "Software\${PRODUCT}" "InstallDir"
RequestExecutionLevel admin
BrandingText "RKFlash · RK 固件烧录"

VIProductVersion "${VERSION}.0"
VIAddVersionKey /LANG=0 "ProductName" "${PRODUCT}"
VIAddVersionKey /LANG=0 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=0 "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=0 "FileDescription" "RKFlash Setup"
VIAddVersionKey /LANG=0 "CompanyName" "OpenCenter"
VIAddVersionKey /LANG=0 "LegalCopyright" "MIT / GPL-2.0-or-later / LGPL"

!define MUI_ICON "${STAGE}\rkflash.ico"
!define MUI_UNICON "${STAGE}\rkflash.ico"
!define MUI_ABORTWARNING
!define MUI_COMPONENTSPAGE_SMALLDESC
!define MUI_FINISHPAGE_RUN "$INSTDIR\rkflash.exe"
!define MUI_FINISHPAGE_RUN_TEXT "启动 RKFlash"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${STAGE}\licenses\NOTICE.txt"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP "RKFlash 仅支持 64 位 Windows 10/11。"
    Abort
  ${EndIf}
  SetRegView 64
FunctionEnd

Function un.onInit
  SetRegView 64
FunctionEnd

Section "RKFlash 主程序" SecApp
  SectionIn RO
  SetOutPath "$INSTDIR"
  File /r "${STAGE}\*.*"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateDirectory "$SMPROGRAMS\${PRODUCT}"
  CreateShortcut "$SMPROGRAMS\${PRODUCT}\RKFlash.lnk" "$INSTDIR\rkflash.exe" "" "$INSTDIR\rkflash.ico"
  CreateShortcut "$SMPROGRAMS\${PRODUCT}\安装或修复 USB 驱动.lnk" "$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" \
    '-NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\driver\install-driver.ps1"' "$INSTDIR\rkflash.ico"
  CreateShortcut "$SMPROGRAMS\${PRODUCT}\卸载 RKFlash.lnk" "$INSTDIR\uninstall.exe"

  WriteRegStr HKLM "Software\${PRODUCT}" "InstallDir" "$INSTDIR"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayName" "RKFlash · RK 固件烧录"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "Publisher" "OpenCenter"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "URLInfoAbout" "https://github.com/open-center/rkflash"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\rkflash.ico"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKLM "${UNINSTALL_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoRepair" 1
SectionEnd

Section "Rockchip USB 驱动（WinUSB）" SecDriver
  DetailPrint "正在安装 WinUSB 驱动，可能需要一分钟……"
  nsExec::ExecToLog '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\driver\install-driver.ps1" -Quiet'
  Pop $0
  ${If} $0 != 0
    DetailPrint "部分 WinUSB 驱动安装失败（返回 $0），可稍后从开始菜单“安装或修复 USB 驱动”重试。"
    IfSilent +2
      MessageBox MB_ICONEXCLAMATION "部分 WinUSB 驱动安装失败。$\n可稍后从开始菜单“RKFlash → 安装或修复 USB 驱动”重试。"
  ${EndIf}
SectionEnd

Section /o "桌面快捷方式" SecDesktop
  CreateShortcut "$DESKTOP\RKFlash.lnk" "$INSTDIR\rkflash.exe" "" "$INSTDIR\rkflash.ico"
SectionEnd

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SecApp} "RKFlash 图形界面与 rkdeveloptool 烧录核心。"
  !insertmacro MUI_DESCRIPTION_TEXT ${SecDriver} "为 Rockchip 设备（VID 2207）安装 Windows 自带的 WinUSB 驱动。会替换 Rockchip 官方驱动，需再用 RKDevTool 时请重装官方驱动。"
  !insertmacro MUI_DESCRIPTION_TEXT ${SecDesktop} "在桌面创建 RKFlash 快捷方式。"
!insertmacro MUI_FUNCTION_DESCRIPTION_END

Section "Uninstall"
  Delete "$DESKTOP\RKFlash.lnk"
  RMDir /r "$SMPROGRAMS\${PRODUCT}"
  RMDir /r "$INSTDIR"
  DeleteRegKey HKLM "${UNINSTALL_KEY}"
  DeleteRegKey HKLM "Software\${PRODUCT}"
  ; WinUSB 驱动由系统管理，卸载程序不移除，避免影响其他使用 WinUSB 的设备。
SectionEnd
