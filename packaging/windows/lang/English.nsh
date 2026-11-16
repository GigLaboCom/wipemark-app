# English.nsh — the installer's and uninstaller's own words in English
# (packaging/windows/wipemark.nsi; NSIS's Modern UI has the rest). German.nsh
# and Russian.nsh hold the same strings: the application's three languages.
# A string may name the installer's variables ($OldVersion, $InUse,
# $HelperOut, $DataSize, $OldDir), read when it is shown.

!define /redef L ${LANG_ENGLISH}

LangString WelcomeNew ${L} "Setup will install Wipemark ${VERSION} on this computer, for you alone: no administrator rights are needed.$\r$\n$\r$\nClick Next to continue."
LangString WelcomeUpdate ${L} "Wipemark $OldVersion is installed. Setup will update it to ${VERSION} in its folder; your data, settings and downloaded models are kept.$\r$\n$\r$\nClose Wipemark before you continue.$\r$\n$\r$\nClick Next to continue."
LangString OptionsTitle ${L} "Options"
LangString OptionsSubtitle ${L} "Choose how Wipemark is set up."
LangString PathBox ${L} "Add wipemark-cli to PATH"
LangString PathText ${L} "Lets you run wipemark-cli in any new terminal window: Command Prompt, PowerShell, Windows Terminal. The installation folder is added to your user PATH, and uninstalling Wipemark takes it out again."
LangString Running ${L} "Wipemark is running ($InUse is in use).$\r$\n$\r$\nClose Wipemark, and any wipemark-cli still running in a terminal, then click Retry."
LangString NotChanged ${L} "Wipemark is running: nothing was changed."
LangString RemovingOld ${L} "Removing the earlier installation in"
LangString PathFailed ${L} "Wipemark is installed, but wipemark-cli could not be added to PATH:$\r$\n$HelperOut"
LangString RunWipemark ${L} "Run Wipemark"
LangString ShortcutComment ${L} "Strips AI provenance marks from your own text and images"
LangString NeedsWindows10 ${L} "Wipemark needs 64-bit Windows 10 or later."

LangString UnTitle ${L} "Uninstall Wipemark"
LangString UnSubtitle ${L} "Remove Wipemark from this computer."
LangString UnFolder ${L} "Wipemark will be removed from this folder:"
LangString UnDataText ${L} "Your data (settings, history, logs) and the models you downloaded are in this folder:"
LangString UnDataKept ${L} "They are kept, so that a later installation finds them, unless you tick the box below."
LangString UnDataBox ${L} "Also delete my data and downloaded models ($DataSize)"
LangString UnModelsElsewhere ${L} "A models folder you chose yourself in Settings > Models, outside this folder, is never deleted."
LangString UnNoData ${L} "There is no Wipemark data in this folder:"
LangString UnDataFailed ${L} "Wipemark is uninstalled, but your data could not all be deleted:$\r$\n$HelperOut"
LangString UnDataKeptLog ${L} "Kept: your data and models in"
LangString UnitKB ${L} "KB"
LangString UnitMB ${L} "MB"
LangString UnitGB ${L} "GB"
LangString DecimalSeparator ${L} "."
