# wipemark.nsi — the Windows installer, Wipemark-<version>-x86_64-setup.exe.
#
# What it is for: D842–D843 (the owner, 2026-11-16: "I think the Windows
# version needs an installer"; format, scope, PATH and uninstall chosen by
# the owner). A per-user NSIS installer made from the same folder the
# portable zip is made of (scripts/release/windows/package.sh), which stays
# beside it on every release. No administrator is asked for, at install or
# at an update.
#
# What it does:
#   - installs into %LOCALAPPDATA%\Programs\Wipemark (or /D=, or where an
#     earlier installation is), for the person running it alone
#     (RequestExecutionLevel user; HKCU only);
#   - a Start menu shortcut, Wipemark, and an entry in Apps & features:
#     HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Wipemark with
#     the name, version, publisher, icon, folder, size and both uninstall
#     strings, NoModify and NoRepair;
#   - "Add wipemark-cli to PATH", ticked by default (an update keeps the
#     earlier choice): the folder appended to the user's PATH
#     (HKCU\Environment, through setup.ps1, never cut at NSIS's 1024
#     characters) and WM_SETTINGCHANGE broadcast so new terminals see it.
#     The entry it added is recorded (PathEntry); the uninstaller takes out
#     exactly that entry and nothing else;
#   - over an earlier version it updates in place; one in another folder is
#     uninstalled first by its own uninstaller, silently, its data kept;
#   - nothing is written while a program or library it would replace is in
#     use (a running Wipemark, a wipemark-cli in a terminal): interactively
#     it asks for Wipemark to be closed (Retry / Cancel); silently it waits
#     up to 30 seconds — time for an application that started its own
#     update to quit — and then ends with exit code 2, nothing changed;
#   - a "Run Wipemark" box on the finish page; English, German and Russian,
#     chosen by the system's language (/LANG=<LCID> overrides);
#   - the uninstaller keeps the person's data and models unless its box
#     "Also delete my data and downloaded models (<size>)" is ticked — the
#     data folder is the application's: %WIPEMARK_DATA_DIR% when set to an
#     absolute path, else %APPDATA%\GigLabo\wipemark\data — and says that a
#     models folder chosen outside it is never deleted (it is not under the
#     data folder, and links inside it are removed, never followed).
#
# Command line (both programs take /S, silent):
#   setup.exe  [/S] [/PATH=0|1] [/LANG=1033|1031|1049] [/D=<folder>]
#              /D= last and unquoted (NSIS's rule). /PATH=0 leaves PATH
#              alone (and takes out an entry an earlier install made).
#              Exit 0 installed, 1 cancelled, 2 not installed (Wipemark
#              running past the wait, or a refused system).
#   uninstall.exe [/S] [/DELETEDATA] [/LANG=…]
#              silent keeps the data unless /DELETEDATA is given.
#              Exit 0 removed, 1 cancelled, 2 Wipemark running past the wait.
#              It copies itself to %TEMP% and returns at once (NSIS's way of
#              deleting itself); wait for uninstall.exe to disappear, or run
#              a copy outside the folder with _?=<folder>.
#
# How to build: scripts/release/windows/installer.sh, which passes VERSION,
# VERSION_QUAD, PKG (the package folder), FILES (the generated list of its
# files), LICENSE_TXT and OUTFILE. Not by hand.
# Needs: NSIS 3 (the release builds with the version installer.sh pins),
# stock plugins only (nsDialogs, nsExec, System), and on the machine it runs
# on, Windows 10 or later, 64-bit, with Windows PowerShell (always there).
# Output: OUTFILE.

!ifndef VERSION | VERSION_QUAD | PKG | FILES | LICENSE_TXT | OUTFILE
  !error "build with scripts/release/windows/installer.sh"
!endif
!ifndef COMPRESSOR
  !define COMPRESSOR "/SOLID lzma"
!endif

Unicode true
ManifestDPIAware true
RequestExecutionLevel user
SetCompressor ${COMPRESSOR}
CRCCheck on
AllowSkipFiles off

!define APP "Wipemark"
!define PUBLISHER "GigLabo"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Wipemark"
!define WAIT_HALF_SECONDS 60

Name "${APP}"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\Wipemark"
InstallDirRegKey HKCU "${UNINST_KEY}" "InstallLocation"
BrandingText "${APP} ${VERSION}"

!include MUI2.nsh
!include nsDialogs.nsh
!include LogicLib.nsh
!include FileFunc.nsh
!include WinMessages.nsh
!include WinVer.nsh
!include x64.nsh

# State. Declared before the language files, whose strings name some.
Var AddPath        ; 1: the box is ticked
Var OldDir         ; an earlier installation's folder, or ""
Var OldVersion
Var OldPathEntry   ; the PATH entry an earlier installation added, or ""
Var WelcomeText
Var PathBox
Var Helper         ; setup.ps1's exit code
Var HelperOut      ; and its first line
Var InUse          ; the file found in use, or ""
Var DataDir
Var DataSize       ; the data folder's size, said for a person
Var DataKiB
Var DeleteData     ; 1: the uninstaller's box is ticked, or /DELETEDATA
Var DataBox
Var UnFinishText   ; the uninstaller's last page: where the data is, or that it went

!define MUI_ICON "${__FILEDIR__}\..\..\icons\windows\wipemark.ico"
!define MUI_UNICON "${__FILEDIR__}\..\..\icons\windows\wipemark.ico"
!define MUI_WELCOMEFINISHPAGE_BITMAP "${__FILEDIR__}\welcome.bmp"
!define MUI_UNWELCOMEFINISHPAGE_BITMAP "${__FILEDIR__}\welcome.bmp"
!define MUI_ABORTWARNING

!define MUI_WELCOMEPAGE_TEXT "$WelcomeText"
!define MUI_PAGE_CUSTOMFUNCTION_PRE WelcomePre
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${LICENSE_TXT}"
Page custom OptionsPage OptionsLeave
!define MUI_PAGE_CUSTOMFUNCTION_LEAVE DirectoryLeave
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN
!define MUI_FINISHPAGE_RUN_TEXT "$(RunWipemark)"
!define MUI_FINISHPAGE_RUN_FUNCTION RunWipemark
!insertmacro MUI_PAGE_FINISH

UninstPage custom un.DataPage un.DataLeave
!insertmacro MUI_UNPAGE_INSTFILES
!define MUI_FINISHPAGE_TEXT "$UnFinishText"
!insertmacro MUI_UNPAGE_FINISH

# The first is the fallback for a system language that is none of them.
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "German"
!insertmacro MUI_LANGUAGE "Russian"
!include /CHARSET=UTF8 "${__FILEDIR__}\lang\English.nsh"
!include /CHARSET=UTF8 "${__FILEDIR__}\lang\German.nsh"
!include /CHARSET=UTF8 "${__FILEDIR__}\lang\Russian.nsh"

VIProductVersion "${VERSION_QUAD}"
VIFileVersion "${VERSION_QUAD}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "ProductName" "${APP}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "CompanyName" "${PUBLISHER}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "FileDescription" "${APP} Setup"
VIAddVersionKey /LANG=${LANG_ENGLISH} "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "LegalCopyright" "Copyright 2026 GigLabo and Denis Esakov"
VIAddVersionKey /LANG=${LANG_ENGLISH} "OriginalFilename" "Wipemark-${VERSION}-x86_64-setup.exe"

# The generated list: WIPEMARK_FILES (File lines), WIPEMARK_DELETE_FILES
# (Delete lines) and WIPEMARK_IN_USE_ALL (one WIPEMARK_IN_USE per program
# and library).
!macro WIPEMARK_IN_USE DIR NAME
  ${If} $InUse == ""
  ${AndIf} ${FileExists} "${DIR}\${NAME}"
    ClearErrors
    FileOpen $0 "${DIR}\${NAME}" a
    ${If} ${Errors}
      StrCpy $InUse "${NAME}"
    ${Else}
      FileClose $0
    ${EndIf}
  ${EndIf}
!macroend
!include "${FILES}"

# Code both programs use, defined once for each (un. functions are the
# uninstaller's).
!macro SHARED PREFIX
  # /LANG=<LCID>, when it is one of ours.
  Function ${PREFIX}PickLanguage
    ${GetParameters} $R0
    ClearErrors
    ${GetOptions} $R0 "/LANG=" $R1
    ${IfNot} ${Errors}
      ${If} $R1 == ${LANG_ENGLISH}
      ${OrIf} $R1 == ${LANG_GERMAN}
      ${OrIf} $R1 == ${LANG_RUSSIAN}
        StrCpy $LANGUAGE $R1
      ${EndIf}
    ${EndIf}
  FunctionEnd

  # setup.ps1 with the action on the stack; the paths it reads are already
  # in WIPEMARK_SETUP_DIR / WIPEMARK_SETUP_DATA. Sets $Helper (exit code,
  # or "error" when PowerShell did not start) and $HelperOut (its output).
  Function ${PREFIX}Helper
    Exch $R0
    InitPluginsDir
    File "/oname=$PLUGINSDIR\setup.ps1" "${__FILEDIR__}\setup.ps1"
    StrCpy $R9 "$PLUGINSDIR\setup.ps1"
    System::Call 'kernel32::SetEnvironmentVariable(t "WIPEMARK_SETUP_PS1", t R9) i'
    nsExec::ExecToStack '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -Command "& ([ScriptBlock]::Create([IO.File]::ReadAllText($$env:WIPEMARK_SETUP_PS1))) $R0"'
    Pop $Helper
    Pop $HelperOut
    # The first line only, without its line break.
    StrCpy $R1 0
    ${Do}
      StrCpy $R2 $HelperOut 1 $R1
      ${If} $R2 == ""
      ${OrIf} $R2 == "$\r"
      ${OrIf} $R2 == "$\n"
        ${ExitDo}
      ${EndIf}
      IntOp $R1 $R1 + 1
    ${Loop}
    StrCpy $HelperOut $HelperOut $R1
    DetailPrint "setup.ps1 $R0: $HelperOut"
    System::Call 'kernel32::SetEnvironmentVariable(t "WIPEMARK_SETUP_PS1", n) i'
    Pop $R0
  FunctionEnd

  # Every program and library of ours in $R5 (and $R6, when not empty) that
  # is in use: $InUse names the first, or is "". Interactively asks until
  # they are free or the person cancels; silently waits up to 30 seconds.
  Function ${PREFIX}WaitForFree
    StrCpy $R4 0
    ${Do}
      StrCpy $InUse ""
      !insertmacro WIPEMARK_IN_USE_ALL "$R5"
      ${If} $R6 != ""
        !insertmacro WIPEMARK_IN_USE_ALL "$R6"
      ${EndIf}
      ${If} $InUse == ""
        ${ExitDo}
      ${EndIf}
      ${If} ${Silent}
        ${If} $R4 >= ${WAIT_HALF_SECONDS}
          DetailPrint "$InUse is in use: nothing was changed"
          ${ExitDo}
        ${EndIf}
        Sleep 500
        IntOp $R4 $R4 + 1
      ${Else}
        ${IfNot} ${Cmd} `MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "$(Running)" IDRETRY`
          ${ExitDo}
        ${EndIf}
      ${EndIf}
    ${Loop}
  FunctionEnd

  # The user's PATH changed: tell every top-level window, so a terminal
  # opened from Explorer (or the Start menu) after this sees it.
  Function ${PREFIX}Broadcast
    SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
  FunctionEnd
!macroend
!insertmacro SHARED ""
!insertmacro SHARED "un."

Function .onInit
  Call PickLanguage
  ${IfNot} ${AtLeastWin10}
  ${OrIfNot} ${RunningX64}
    ${IfNot} ${Silent}
      MessageBox MB_OK|MB_ICONSTOP "$(NeedsWindows10)"
    ${EndIf}
    SetErrorLevel 2
    Abort
  ${EndIf}
  ReadRegStr $OldDir HKCU "${UNINST_KEY}" "InstallLocation"
  ReadRegStr $OldVersion HKCU "${UNINST_KEY}" "DisplayVersion"
  ReadRegStr $OldPathEntry HKCU "${UNINST_KEY}" "PathEntry"
  ClearErrors
  ReadRegDWORD $AddPath HKCU "${UNINST_KEY}" "AddToPath"
  ${If} ${Errors}
    StrCpy $AddPath 1
  ${EndIf}
  ${GetParameters} $R0
  ClearErrors
  ${GetOptions} $R0 "/PATH=" $R1
  ${IfNot} ${Errors}
    ${If} $R1 == 0
      StrCpy $AddPath 0
    ${Else}
      StrCpy $AddPath 1
    ${EndIf}
  ${EndIf}
FunctionEnd

# Here and not in .onInit: a language /LANG= picks there takes effect only
# after .onInit, so a string read in it would be in the system's language.
Function WelcomePre
  ${If} $OldVersion == "${VERSION}"
    StrCpy $WelcomeText "$(WelcomeAgain)"
  ${ElseIf} $OldVersion != ""
    StrCpy $WelcomeText "$(WelcomeUpdate)"
  ${Else}
    StrCpy $WelcomeText "$(WelcomeNew)"
  ${EndIf}
FunctionEnd

Function OptionsPage
  !insertmacro MUI_HEADER_TEXT "$(OptionsTitle)" "$(OptionsSubtitle)"
  nsDialogs::Create 1018
  Pop $0
  ${NSD_CreateCheckbox} 0 0 100% 12u "$(PathBox)"
  Pop $PathBox
  ${If} $AddPath == 1
    ${NSD_Check} $PathBox
  ${EndIf}
  ${NSD_CreateLabel} 12u 16u -12u 48u "$(PathText)"
  Pop $0
  nsDialogs::Show
FunctionEnd

Function OptionsLeave
  ${NSD_GetState} $PathBox $0
  ${If} $0 == ${BST_CHECKED}
    StrCpy $AddPath 1
  ${Else}
    StrCpy $AddPath 0
  ${EndIf}
FunctionEnd

# The last page before anything is written: ask for a running Wipemark to
# be closed here, where Cancel leaves the person on the page.
Function DirectoryLeave
  StrCpy $R5 $INSTDIR
  StrCpy $R6 ""
  ${If} $OldDir != ""
  ${AndIf} $OldDir != $INSTDIR
    StrCpy $R6 $OldDir
  ${EndIf}
  Call WaitForFree
  ${If} $InUse != ""
    Abort
  ${EndIf}
FunctionEnd

Function RunWipemark
  SetOutPath "$INSTDIR"
  Exec '"$INSTDIR\wipemark.exe"'
FunctionEnd

Section "-Wipemark"
  SetShellVarContext current

  # Again here: a silent run has no pages, and an interactive one may have
  # started Wipemark again since the folder page.
  StrCpy $R5 $INSTDIR
  StrCpy $R6 ""
  ${If} $OldDir != ""
  ${AndIf} $OldDir != $INSTDIR
    StrCpy $R6 $OldDir
  ${EndIf}
  Call WaitForFree
  ${If} $InUse != ""
    SetErrorLevel 2
    Abort "$(NotChanged)"
  ${EndIf}

  # An earlier installation in another folder goes first, by its own
  # uninstaller run silently in place (its data kept), so no second copy
  # and no second PATH entry stay behind.
  ${If} $R6 != ""
  ${AndIf} ${FileExists} "$OldDir\uninstall.exe"
    DetailPrint "$(RemovingOld) $OldDir"
    ExecWait '"$OldDir\uninstall.exe" /S _?=$OldDir' $0
    Delete "$OldDir\uninstall.exe"
    RMDir "$OldDir"
    StrCpy $OldPathEntry ""
  ${EndIf}

  SetOutPath "$INSTDIR"
  SetOverwrite on
  # Over an earlier installation here: the backends its llama.cpp had and
  # this one may not. Never in a folder that is not ours.
  ${If} $OldDir == $INSTDIR
  ${AndIf} ${FileExists} "$INSTDIR\uninstall.exe"
    Delete "$INSTDIR\ggml-*.dll"
  ${EndIf}
  !insertmacro WIPEMARK_FILES
  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateShortcut "$SMPROGRAMS\${APP}.lnk" "$INSTDIR\wipemark.exe" "" "$INSTDIR\wipemark.exe" 0 SW_SHOWNORMAL "" "$(ShortcutComment)"

  # PATH: the box decides, whatever an earlier installation did.
  StrCpy $R9 "$INSTDIR"
  System::Call 'kernel32::SetEnvironmentVariable(t "WIPEMARK_SETUP_DIR", t R9) i'
  ${If} $AddPath == 1
    Push path-add
    Call Helper
    ${If} $Helper == 0
      ${If} $HelperOut == "added"
        WriteRegStr HKCU "${UNINST_KEY}" "PathEntry" "$INSTDIR"
        Call Broadcast
      ${ElseIf} $OldPathEntry == $INSTDIR
        # The entry is the one an earlier installation of ours made.
        WriteRegStr HKCU "${UNINST_KEY}" "PathEntry" "$INSTDIR"
      ${Else}
        DeleteRegValue HKCU "${UNINST_KEY}" "PathEntry"
      ${EndIf}
    ${Else}
      ${IfNot} ${Silent}
        MessageBox MB_OK|MB_ICONEXCLAMATION "$(PathFailed)"
      ${EndIf}
    ${EndIf}
  ${ElseIf} $OldPathEntry != ""
    StrCpy $R9 "$OldPathEntry"
    System::Call 'kernel32::SetEnvironmentVariable(t "WIPEMARK_SETUP_DIR", t R9) i'
    Push path-remove
    Call Helper
    ${If} $Helper == 0
      DeleteRegValue HKCU "${UNINST_KEY}" "PathEntry"
      Call Broadcast
    ${EndIf}
  ${EndIf}
  System::Call 'kernel32::SetEnvironmentVariable(t "WIPEMARK_SETUP_DIR", n) i'

  # Apps & features.
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "${APP}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\wipemark.exe,0"
  WriteRegStr HKCU "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegStr HKCU "${UNINST_KEY}" "URLInfoAbout" "https://github.com/GigLaboCom/wipemark-app"
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "AddToPath" $AddPath
  ${GetSize} "$INSTDIR" "/S=0K /G=0" $0 $1 $2
  WriteRegDWORD HKCU "${UNINST_KEY}" "EstimatedSize" $0
SectionEnd

# ---- the uninstaller ----

Function un.onInit
  Call un.PickLanguage
  SetShellVarContext current
  # The application's data folder: WIPEMARK_DATA_DIR when it is an
  # absolute path (as Layout::discover reads it), else the default.
  ReadEnvStr $DataDir WIPEMARK_DATA_DIR
  StrCpy $0 $DataDir 2 1
  StrCpy $1 $DataDir 2
  ${If} $0 != ":\"
  ${AndIf} $1 != "\\"
    StrCpy $DataDir "$APPDATA\GigLabo\wipemark\data"
  ${EndIf}
  StrCpy $DeleteData 0
  ${GetParameters} $R0
  ClearErrors
  ${GetOptions} $R0 "/DELETEDATA" $R1
  ${IfNot} ${Errors}
    StrCpy $DeleteData 1
  ${EndIf}
FunctionEnd

# A size in KiB as a person reads it: KB, MB, or GB with one decimal.
Function un.SayKiB
  ${If} $DataKiB < 1024
    StrCpy $DataSize "$DataKiB $(UnitKB)"
  ${ElseIf} $DataKiB < 1048576
    IntOp $0 $DataKiB / 1024
    StrCpy $DataSize "$0 $(UnitMB)"
  ${Else}
    IntOp $0 $DataKiB / 1048576
    IntOp $1 $DataKiB % 1048576
    IntOp $1 $1 * 10
    IntOp $1 $1 / 1048576
    StrCpy $DataSize "$0$(DecimalSeparator)$1 $(UnitGB)"
  ${EndIf}
FunctionEnd

Function un.DataPage
  !insertmacro MUI_HEADER_TEXT "$(UnTitle)" "$(UnSubtitle)"
  nsDialogs::Create 1018
  Pop $0
  ${NSD_CreateLabel} 0 0 100% 10u "$(UnFolder)"
  Pop $0
  ${NSD_CreateText} 0 11u 100% 12u "$INSTDIR"
  Pop $0
  SendMessage $0 ${EM_SETREADONLY} 1 0

  StrCpy $R9 "$DataDir"
  System::Call 'kernel32::SetEnvironmentVariable(t "WIPEMARK_SETUP_DATA", t R9) i'
  Push measure
  Call un.Helper
  System::Call 'kernel32::SetEnvironmentVariable(t "WIPEMARK_SETUP_DATA", n) i'
  StrCpy $DataKiB 0
  ${If} $Helper == 0
    StrCpy $DataKiB $HelperOut
  ${EndIf}

  ${If} ${FileExists} "$DataDir\*.*"
    ${NSD_CreateLabel} 0 32u 100% 20u "$(UnDataText)"
    Pop $0
    ${NSD_CreateText} 0 53u 100% 12u "$DataDir"
    Pop $0
    SendMessage $0 ${EM_SETREADONLY} 1 0
    ${NSD_CreateLabel} 0 69u 100% 20u "$(UnDataKept)"
    Pop $0
    Call un.SayKiB
    ${NSD_CreateCheckbox} 0 91u 100% 12u "$(UnDataBox)"
    Pop $DataBox
    ${If} $DeleteData == 1
      ${NSD_Check} $DataBox
    ${EndIf}
    ${NSD_CreateLabel} 12u 105u -12u 20u "$(UnModelsElsewhere)"
    Pop $0
  ${Else}
    StrCpy $DataBox ""
    ${NSD_CreateLabel} 0 32u 100% 10u "$(UnNoData)"
    Pop $0
    ${NSD_CreateText} 0 43u 100% 12u "$DataDir"
    Pop $0
    SendMessage $0 ${EM_SETREADONLY} 1 0
  ${EndIf}
  nsDialogs::Show
FunctionEnd

Function un.DataLeave
  StrCpy $DeleteData 0
  ${If} $DataBox != ""
    ${NSD_GetState} $DataBox $0
    ${If} $0 == ${BST_CHECKED}
      StrCpy $DeleteData 1
    ${EndIf}
  ${EndIf}
  StrCpy $R5 $INSTDIR
  StrCpy $R6 ""
  Call un.WaitForFree
  ${If} $InUse != ""
    Abort
  ${EndIf}
FunctionEnd

Section "Uninstall"
  SetShellVarContext current

  StrCpy $R5 $INSTDIR
  StrCpy $R6 ""
  Call un.WaitForFree
  ${If} $InUse != ""
    SetErrorLevel 2
    Abort "$(NotChanged)"
  ${EndIf}

  # Exactly the entry the installer added, if it added one.
  ReadRegStr $0 HKCU "${UNINST_KEY}" "PathEntry"
  ${If} $0 != ""
    StrCpy $R9 "$0"
    System::Call 'kernel32::SetEnvironmentVariable(t "WIPEMARK_SETUP_DIR", t R9) i'
    Push path-remove
    Call un.Helper
    System::Call 'kernel32::SetEnvironmentVariable(t "WIPEMARK_SETUP_DIR", n) i'
    Call un.Broadcast
  ${EndIf}

  Delete "$SMPROGRAMS\${APP}.lnk"
  !insertmacro WIPEMARK_DELETE_FILES
  Delete "$INSTDIR\uninstall.exe"
  # Not /r: only what this installed goes, and the folder only if empty.
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "${UNINST_KEY}"

  StrCpy $UnFinishText "$(MUI_UNTEXT_FINISH_INFO_TEXT)"
  ${If} $DeleteData == 1
    StrCpy $R9 "$DataDir"
    System::Call 'kernel32::SetEnvironmentVariable(t "WIPEMARK_SETUP_DATA", t R9) i'
    Push remove-data
    Call un.Helper
    System::Call 'kernel32::SetEnvironmentVariable(t "WIPEMARK_SETUP_DATA", n) i'
    ${If} $Helper != 0
      ${IfNot} ${Silent}
        MessageBox MB_OK|MB_ICONEXCLAMATION "$(UnDataFailed)"
      ${EndIf}
    ${Else}
      StrCpy $UnFinishText "$(UnFinishDeleted)"
    ${EndIf}
  ${ElseIf} ${FileExists} "$DataDir\*.*"
    DetailPrint "$(UnDataKeptLog) $DataDir"
    StrCpy $UnFinishText "$(UnFinishKept)"
  ${EndIf}
SectionEnd
