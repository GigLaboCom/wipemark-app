# installer-check.ps1 — install the Windows installer silently, check what it
# did, uninstall it, and check that it is gone and the rest is as it was.
#
# What it is for: D842–D843 (the owner, 2026-11-16): the per-user NSIS
# installer (packaging/windows/wipemark.nsi) must put wipemark-cli on the
# user's PATH and take exactly that entry out again, keep the person's data
# at uninstall unless asked, update in place, and never hang or change
# anything in silent mode while Wipemark runs. gate.yml runs it on every push
# against an installer made from a debug layout; release.yml against the
# release's own installer before it is attached.
#
# What it does (each check prints "ok  …"; the first that fails stops it):
#   0. refuses a machine where Wipemark is installed or wipemark-cli is
#      already on PATH; sets the user's PATH to a known value (an unexpanded
#      %USERPROFILE% entry, one with a space, a trailing ';'), and seeds a
#      data folder — the default one with -DefaultDataDir (CI's runner has
#      none), else a scratch one through WIPEMARK_DATA_DIR;
#   1. setup /S /D=<scratch>\O'Brien Programs\Wipemark (a space and an
#      apostrophe on purpose): exit 0; every file of the package there, byte
#      for byte, and uninstall.exe; the Start menu shortcut to wipemark.exe;
#      the Uninstall key (name, version, publisher, icon, folder, size, both
#      uninstall strings, NoModify, NoRepair, PathEntry); PATH exactly the
#      old value plus the folder; a new shell, its PATH read from the
#      registry as Explorer gives it to a new terminal, finds wipemark-cli
#      there and `--version` names the version;
#   2. setup /S again (an update in place): PATH holds the folder once;
#   3. setup /S /PATH=0: PATH exactly as before, no PathEntry; setup /S
#      (the choice kept): still not on PATH; setup /S /PATH=1: back;
#   4. with wipemark-cli running from the folder: setup /S ends with exit 2
#      within the wait (30 s) and changes nothing; so does uninstall /S;
#   5. the QuietUninstallString as Apps & features runs it, waited for to
#      the end of the copy it hands over to: the folder, the shortcut and
#      the key gone, PATH exactly the value of step 0, the data folder and
#      its files still there;
#   6. setup /S, then uninstall /S /DELETEDATA with a junction in the data
#      folder pointing at a folder outside it: the data folder and its empty
#      GigLabo\wipemark parents gone, the junction's target and its file
#      untouched, PATH as in step 0.
# Whatever happens, the user's PATH is put back as it was before step 0, and
# what the run made is removed.
#
# How to run (Windows):
#   pwsh -File scripts/release/windows/installer-check.ps1 `
#     -Setup dist\Wipemark-<v>-x86_64-setup.exe -Package dist\Wipemark-<v>-x86_64-windows [-DefaultDataDir]
# Needs: Windows 10 or 11, PowerShell 7 or Windows PowerShell 5.1; a user
# with no Wipemark installed (it refuses otherwise). Writes only under the
# user's profile (HKCU, the Start menu, a scratch folder in TEMP).
# Output: one line per check; exit 0 when all pass, 1 at the first failure.
param(
  [Parameter(Mandatory = $true)][string]$Setup,
  [Parameter(Mandatory = $true)][string]$Package,
  [switch]$DefaultDataDir
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2

$Setup = (Resolve-Path -LiteralPath $Setup).Path
$Package = (Resolve-Path -LiteralPath $Package).Path
if ((Split-Path -Leaf $Setup) -notmatch '^Wipemark-(.+)-x86_64-setup\.exe$') { throw "$Setup is not Wipemark-<version>-x86_64-setup.exe" }
$version = $Matches[1]

$uninstKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Wipemark'
$lnk = Join-Path ([Environment]::GetFolderPath('Programs')) 'Wipemark.lnk'
$tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$scratch = Join-Path $tempRoot ("wipemark-installer-check-" + [Guid]::NewGuid().ToString('N').Substring(0, 8))
$inst = Join-Path $scratch "O'Brien Programs\Wipemark"
$outside = Join-Path $scratch 'outside models'
if ($DefaultDataDir) {
  $data = Join-Path $env:APPDATA 'GigLabo\wipemark\data'
} else {
  $data = Join-Path $scratch 'data root\GigLabo\wipemark\data'
  $env:WIPEMARK_DATA_DIR = $data
}
$giglabo = Split-Path -Parent (Split-Path -Parent $data)

function Ok([string]$what) { Write-Output "ok  $what" }
function Check([bool]$cond, [string]$what) {
  if (-not $cond) { throw "FAIL $what" }
  Ok $what
}

function Env-Key { [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment') }
function Read-UserPath {
  $key = Env-Key
  $value = $key.GetValue('Path', $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
  $kind = if ($null -ne $value) { $key.GetValueKind('Path') } else { $null }
  return @{ Value = $value; Kind = $kind }
}
function Write-UserPath($saved) {
  $key = Env-Key
  if ($null -eq $saved.Value) { $key.DeleteValue('Path', $false) }
  else { $key.SetValue('Path', $saved.Value, $saved.Kind) }
}

# A program, waited for; its exit code.
function Run([string]$exe, [string]$arguments, [int]$seconds = 300) {
  $psi = New-Object Diagnostics.ProcessStartInfo $exe, $arguments
  $psi.UseShellExecute = $false
  $p = [Diagnostics.Process]::Start($psi)
  if (-not $p.WaitForExit($seconds * 1000)) {
    $p.Kill()
    throw "FAIL $exe $arguments did not end within $seconds s"
  }
  return $p.ExitCode
}

function Install([string]$extra = '') {
  $arguments = ("/S $extra /D=$inst") -replace '  +', ' '
  $watch = [Diagnostics.Stopwatch]::StartNew()
  $code = Run $Setup $arguments
  return @{ Code = $code; Seconds = [int]$watch.Elapsed.TotalSeconds }
}

# The uninstaller copied out of the folder and run in place with _?=, so it
# is waited for and its exit code read (from inside the folder it copies
# itself to TEMP and returns at once).
function Uninstall([string]$extra = '') {
  $copy = Join-Path $scratch 'uninstall-copy.exe'
  Copy-Item -LiteralPath (Join-Path $inst 'uninstall.exe') $copy -Force
  $code = Run $copy ("/S $extra _?=$inst" -replace '  +', ' ')
  Remove-Item -LiteralPath $copy -Force
  return $code
}

# A new shell's view: PATH as Explorer builds it for a new terminal.
function Fresh-Shell([string]$command) {
  $psi = New-Object Diagnostics.ProcessStartInfo "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe", "-NoLogo -NoProfile -NonInteractive -Command $command"
  $psi.UseShellExecute = $false
  $psi.RedirectStandardOutput = $true
  $psi.EnvironmentVariables['Path'] = [Environment]::GetEnvironmentVariable('Path', 'Machine') + ';' + [Environment]::GetEnvironmentVariable('Path', 'User')
  $p = [Diagnostics.Process]::Start($psi)
  $out = $p.StandardOutput.ReadToEnd()
  $p.WaitForExit()
  return ($out -split "`r?`n" | Where-Object { $_ -ne '' })
}

function Expect-Installed([string]$what) {
  foreach ($file in Get-ChildItem -LiteralPath $Package -File) {
    $there = Join-Path $inst $file.Name
    if (-not (Test-Path -LiteralPath $there)) { throw "FAIL $what`: no $there" }
    if ((Get-FileHash -LiteralPath $there).Hash -ne (Get-FileHash -LiteralPath $file.FullName).Hash) { throw "FAIL $what`: $($file.Name) differs from the package's" }
  }
  Check (Test-Path -LiteralPath (Join-Path $inst 'uninstall.exe')) "$what`: every file of the package, byte for byte, and uninstall.exe"
}

function Path-Entries { @((Read-UserPath).Value -split ';' | Where-Object { $_.TrimEnd('\') -ieq $inst }) }

$original = Read-UserPath
$cli = $null
try {
  # 0. Where it starts.
  if (Test-Path $uninstKey) { throw "Wipemark is installed for this user ($uninstKey): this check would replace it; run it as another user" }
  if (Test-Path -LiteralPath $lnk) { throw "$lnk exists: this check would replace it" }
  if ($DefaultDataDir -and (Test-Path -LiteralPath $data)) { throw "$data exists: -DefaultDataDir needs a user with no Wipemark data" }
  $found = Fresh-Shell '(Get-Command wipemark-cli -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1).Source'
  if ($found) { throw "wipemark-cli is already on PATH ($found): the check could not tell its own" }
  $base = '%USERPROFILE%\wm-check-bin;C:\wm check dir;'
  Write-UserPath @{ Value = $base; Kind = [Microsoft.Win32.RegistryValueKind]::ExpandString }
  New-Item -ItemType Directory -Force (Join-Path $data 'models\m') | Out-Null
  Set-Content -LiteralPath (Join-Path $data 'sentinel.txt') 'the person''s data'
  Set-Content -LiteralPath (Join-Path $data 'models\m\weights.gguf') 'a downloaded model'
  Ok "user PATH set to $base; data seeded in $data"

  # 1. A first installation.
  $r = Install
  Check ($r.Code -eq 0) "setup /S /D=<folder with a space and an apostrophe>: exit $($r.Code) in $($r.Seconds) s"
  Expect-Installed 'installed'
  $shell = New-Object -ComObject WScript.Shell
  Check ((Test-Path -LiteralPath $lnk) -and ($shell.CreateShortcut($lnk).TargetPath -ieq (Join-Path $inst 'wipemark.exe'))) "Start menu shortcut $lnk -> wipemark.exe"
  $k = Get-ItemProperty $uninstKey
  Check ($k.DisplayName -eq 'Wipemark' -and $k.DisplayVersion -eq $version -and $k.Publisher -eq 'GigLabo') "Uninstall key: Wipemark $version, GigLabo"
  Check ($k.InstallLocation -ieq $inst -and $k.DisplayIcon -ieq "$inst\wipemark.exe,0") 'Uninstall key: InstallLocation and DisplayIcon'
  Check ($k.UninstallString -eq "`"$inst\uninstall.exe`"" -and $k.QuietUninstallString -eq "`"$inst\uninstall.exe`" /S") 'Uninstall key: UninstallString and QuietUninstallString'
  Check ($k.NoModify -eq 1 -and $k.NoRepair -eq 1 -and $k.EstimatedSize -gt 0) "Uninstall key: NoModify, NoRepair, EstimatedSize $($k.EstimatedSize) KB"
  Check ($k.PathEntry -ieq $inst) 'Uninstall key: PathEntry names the folder'
  $now = Read-UserPath
  Check ($now.Value -ceq ($base + $inst + ';') -and $now.Kind -eq [Microsoft.Win32.RegistryValueKind]::ExpandString) "user PATH is the old value plus the folder, still REG_EXPAND_SZ: $($now.Value)"
  $said = @(Fresh-Shell '(Get-Command wipemark-cli -CommandType Application).Source; wipemark-cli --version')
  Check ($said.Count -ge 2 -and $said[0] -ieq "$inst\wipemark-cli.exe" -and $said[1] -eq "wipemark-cli $version") "a new shell finds $($said[0]) and it says '$($said[-1])'"

  # 2. An update in place.
  $r = Install
  Check ($r.Code -eq 0) "setup /S over itself: exit $($r.Code) in $($r.Seconds) s"
  Expect-Installed 'updated'
  Check (@(Path-Entries).Count -eq 1 -and (Get-ItemProperty $uninstKey).PathEntry -ieq $inst) 'the folder is on PATH once, still recorded'

  # 3. The box decides, and an update keeps the choice.
  $r = Install '/PATH=0'
  Check ($r.Code -eq 0 -and (Read-UserPath).Value -ceq $base) "setup /S /PATH=0: exit $($r.Code), PATH exactly as before"
  Check ($null -eq (Get-ItemProperty $uninstKey).PSObject.Properties['PathEntry'] -and (Get-ItemProperty $uninstKey).AddToPath -eq 0) 'no PathEntry, AddToPath 0'
  $r = Install
  Check ($r.Code -eq 0 -and (Read-UserPath).Value -ceq $base) 'setup /S after /PATH=0 keeps it off PATH'
  $r = Install '/PATH=1'
  Check ($r.Code -eq 0 -and (Read-UserPath).Value -ceq ($base + $inst + ';')) 'setup /S /PATH=1 puts it back'

  # 4. Running: nothing changes, nothing hangs.
  $psi = New-Object Diagnostics.ProcessStartInfo (Join-Path $inst 'wipemark-cli.exe'), 'inspect -'
  $psi.UseShellExecute = $false
  $psi.RedirectStandardInput = $true
  $psi.RedirectStandardOutput = $true
  $cli = [Diagnostics.Process]::Start($psi)
  Start-Sleep -Seconds 2
  Check (-not $cli.HasExited) 'wipemark-cli inspect - runs from the folder, waiting on its input'
  $stamp = (Get-Item -LiteralPath (Join-Path $inst 'uninstall.exe')).LastWriteTimeUtc
  $r = Install
  Check ($r.Code -eq 2 -and $r.Seconds -lt 90) "setup /S while it runs: exit $($r.Code) after $($r.Seconds) s"
  Check ((Get-Item -LiteralPath (Join-Path $inst 'uninstall.exe')).LastWriteTimeUtc -eq $stamp) 'nothing was rewritten'
  $code = Uninstall
  Check ($code -eq 2 -and (Test-Path $uninstKey) -and (Test-Path -LiteralPath (Join-Path $inst 'wipemark.exe'))) "uninstall /S while it runs: exit $code, nothing removed"
  $cli.StandardInput.Close()
  if (-not $cli.WaitForExit(30000)) { $cli.Kill() }
  $cli = $null

  # 5. Uninstall as Apps & features does: the data stays.
  $quiet = (Get-ItemProperty $uninstKey).QuietUninstallString
  $exe = $quiet.Substring(1, $quiet.IndexOf('"', 1) - 1)
  $code = Run $exe '/S'
  # It has handed over to its copy, %TEMP%\~nsu<n>.tmp\Un.exe (NSIS's
  # str_uninstchild), and returned: the copy is what to wait for — the data
  # is the last thing it would touch.
  $deadline = (Get-Date).AddSeconds(120)
  do {
    Start-Sleep -Milliseconds 300
    $un = @(Get-Process -Name Un -ErrorAction SilentlyContinue | Where-Object { $_.Path -like '*\~nsu*.tmp\Un.exe' })
  } while ($un.Count -gt 0 -and (Get-Date) -lt $deadline)
  Check ($un.Count -eq 0) "the uninstaller's copy in TEMP ended"
  Check ($code -eq 0 -and -not (Test-Path -LiteralPath $inst)) "the QuietUninstallString: exit $code, the folder gone"
  Check (-not (Test-Path -LiteralPath $lnk) -and -not (Test-Path $uninstKey)) 'the shortcut and the Uninstall key gone'
  $now = Read-UserPath
  Check ($now.Value -ceq $base -and $now.Kind -eq [Microsoft.Win32.RegistryValueKind]::ExpandString) "user PATH exactly as before the installation: $($now.Value)"
  Check ((Test-Path -LiteralPath (Join-Path $data 'sentinel.txt')) -and (Test-Path -LiteralPath (Join-Path $data 'models\m\weights.gguf'))) "the data folder kept: $data"

  # 6. Uninstall with /DELETEDATA: the data goes, never through a link.
  $r = Install
  Check ($r.Code -eq 0) 'setup /S again'
  New-Item -ItemType Directory -Force $outside | Out-Null
  Set-Content -LiteralPath (Join-Path $outside 'keep.gguf') 'a model folder chosen elsewhere'
  New-Item -ItemType Junction -Path (Join-Path $data 'linked models') -Target $outside | Out-Null
  $code = Uninstall '/DELETEDATA'
  Check ($code -eq 0 -and -not (Test-Path -LiteralPath $inst)) "uninstall /S /DELETEDATA: exit $code, the folder gone"
  Check (-not (Test-Path -LiteralPath $data) -and -not (Test-Path -LiteralPath $giglabo)) "the data folder and its empty GigLabo\wipemark parents gone"
  Check (Test-Path -LiteralPath (Join-Path $outside 'keep.gguf')) "the junction's target untouched: $outside"
  Check ((Read-UserPath).Value -ceq $base) 'user PATH exactly as before'
  Write-Output "all checks passed: $Setup"
} finally {
  if ($cli -and -not $cli.HasExited) { $cli.Kill() }
  Write-UserPath $original
  if ((Test-Path $uninstKey) -and ((Get-ItemProperty $uninstKey).InstallLocation -ieq $inst)) { Remove-Item $uninstKey -Recurse -Force }
  if ((Test-Path -LiteralPath $lnk) -and ((New-Object -ComObject WScript.Shell).CreateShortcut($lnk).TargetPath -ieq (Join-Path $inst 'wipemark.exe'))) { Remove-Item -LiteralPath $lnk -Force }
  # The junction first, as a link, so no removal below can walk into it;
  # then the data this run seeded, and its parents only when left empty.
  $link = Join-Path $data 'linked models'
  if (Test-Path -LiteralPath $link) { [IO.Directory]::Delete($link, $false) }
  if (Test-Path -LiteralPath $data) { Remove-Item -LiteralPath $data -Recurse -Force -ErrorAction SilentlyContinue }
  foreach ($dir in @((Split-Path -Parent $data), $giglabo)) {
    if ((Test-Path -LiteralPath $dir) -and -not (Get-ChildItem -LiteralPath $dir -Force)) { Remove-Item -LiteralPath $dir -Force }
  }
  Remove-Item -LiteralPath $scratch -Recurse -Force -ErrorAction SilentlyContinue
}
