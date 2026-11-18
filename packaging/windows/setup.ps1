# setup.ps1 — what the Windows installer and uninstaller leave to PowerShell:
# the user's PATH, and the size and the removal of the person's data.
#
# What it is for: packaging/windows/wipemark.nsi, the per-user installer
# (D842–D843; the owner, 2026-11-16: "I think the Windows version needs an
# installer", wipemark-cli on the user's PATH by a ticked box, the person's
# data kept at uninstall unless a box says otherwise). Two things NSIS
# cannot do safely itself:
#   - PATH. NSIS holds a string in 1024 characters, and a user's PATH is
#     often longer: an installer that read it, appended and wrote it back
#     would cut it (the failure of the old EnvVarUpdate scripts). Here it is
#     read and written whole through the registry, its type (REG_EXPAND_SZ,
#     so %USERPROFILE%-style entries stay unexpanded) kept.
#   - The data folder. NSIS's RMDir /r walks into a junction or a directory
#     link and deletes what it points at (exehead/util.c, myDelete, tests
#     FILE_ATTRIBUTE_DIRECTORY only), so a models folder linked into the
#     data folder would go with it. Here a link is removed as a link and
#     never followed, and the size shown counts what would be removed.
#
# What it does, by action (the first argument):
#   path-add     appends $env:WIPEMARK_SETUP_DIR to HKCU\Environment\Path
#                unless an entry already names that folder (case, quotes, a
#                trailing backslash and environment variables aside); after
#                a value ending in ';' it appends "<dir>;", so path-remove
#                gives the value back byte for byte. Prints "added" or
#                "present" (an entry the installer did not make — it is
#                then not recorded, and the uninstaller leaves it).
#   path-remove  takes every entry equal to $env:WIPEMARK_SETUP_DIR (case,
#                quotes and a trailing backslash aside) out of the user's
#                Path and keeps every other byte, empty entries and order
#                included; a value left empty is deleted, as it was before
#                the installer made it. Prints "removed" or "absent".
#   measure      the size of $env:WIPEMARK_SETUP_DATA in KiB, rounded up,
#                links not followed. Prints the number, 0 when it is not
#                there.
#   remove-data  deletes $env:WIPEMARK_SETUP_DATA: a junction or a symbolic
#                link inside it (or the folder itself, if it is one) is
#                removed as a link, its target untouched; then the two
#                folders above it, if they are GigLabo\wipemark and empty.
#                Prints "removed" or "absent".
# The paths come in environment variables, never on the command line: a
# user name may hold an apostrophe, and no quoting then has to be right.
#
# How it is run (by the installer, from its plugins folder):
#   powershell -NoLogo -NoProfile -NonInteractive -Command
#     "& ([ScriptBlock]::Create([IO.File]::ReadAllText($env:WIPEMARK_SETUP_PS1))) <action>"
# A script block and not -File, so a machine's execution policy, which
# governs script files, does not decide whether PATH can be set.
# By hand, to try it:  $env:WIPEMARK_SETUP_DIR = 'C:\x'; .\setup.ps1 path-add
# Needs: Windows PowerShell 5.1 (every Windows 10 and 11) or PowerShell 7.
# Output: one word or number on stdout and exit 0; on a failure, "error: "
# and the reason on stdout and exit 1. The installer reads both.
param([Parameter(Mandatory = $true)][string]$Action)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2

function Fail([string]$why) {
  Write-Output "error: $why"
  exit 1
}

# An entry as a folder: no surrounding blanks or quotes, no trailing
# backslash, one case.
function Norm([string]$entry) {
  $entry.Trim().Trim('"').TrimEnd('\').ToLowerInvariant()
}

function Env-Path {
  $key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment')
  $value = $key.GetValue('Path', $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
  $kind = [Microsoft.Win32.RegistryValueKind]::ExpandString
  if ($null -ne $value) {
    $kind = $key.GetValueKind('Path')
    if ($kind -ne [Microsoft.Win32.RegistryValueKind]::ExpandString -and
        $kind -ne [Microsoft.Win32.RegistryValueKind]::String) {
      Fail "HKCU\Environment\Path is a $kind value, not a string; it is left as it is"
    }
  }
  return @{ Key = $key; Value = $value; Kind = $kind }
}

function Is-Link([IO.FileSystemInfo]$info) {
  ($info.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0
}

# A junction or a directory link goes as a directory without its contents
# (RemoveDirectory on a reparse point removes the point); a file link as a
# file. Neither touches its target.
function Remove-Link([IO.FileSystemInfo]$info) {
  if ($info -is [IO.DirectoryInfo]) {
    [IO.Directory]::Delete($info.FullName, $false)
  } else {
    $info.Attributes = [IO.FileAttributes]::Normal
    $info.Delete()
  }
}

function Measure-Tree([IO.DirectoryInfo]$dir) {
  [long]$sum = 0
  foreach ($entry in $dir.GetFileSystemInfos()) {
    if (Is-Link $entry) { continue }
    if ($entry -is [IO.DirectoryInfo]) { $sum += Measure-Tree $entry }
    else { $sum += $entry.Length }
  }
  return $sum
}

function Remove-Tree([IO.DirectoryInfo]$dir) {
  foreach ($entry in $dir.GetFileSystemInfos()) {
    if (Is-Link $entry) { Remove-Link $entry; continue }
    if ($entry -is [IO.DirectoryInfo]) { Remove-Tree $entry }
    else {
      $entry.Attributes = [IO.FileAttributes]::Normal
      $entry.Delete()
    }
  }
  $dir.Attributes = [IO.FileAttributes]::Directory
  $dir.Delete()
}

function Remove-IfEmpty([string]$path) {
  if ($path -and [IO.Directory]::Exists($path) -and
      -not [IO.Directory]::EnumerateFileSystemEntries($path).GetEnumerator().MoveNext()) {
    [IO.Directory]::Delete($path, $false)
  }
}

try {
  switch ($Action) {
    'path-add' {
      $dir = $env:WIPEMARK_SETUP_DIR
      if (-not $dir) { Fail 'WIPEMARK_SETUP_DIR is not set' }
      $path = Env-Path
      $old = $path.Value
      if ($old) {
        foreach ($entry in $old.Split(';')) {
          if ($entry.Trim() -and
              (Norm ([Environment]::ExpandEnvironmentVariables($entry))) -eq (Norm $dir)) {
            Write-Output 'present'
            exit 0
          }
        }
      }
      if (-not $old) { $new = $dir }
      elseif ($old.EndsWith(';')) { $new = $old + $dir + ';' }
      else { $new = $old + ';' + $dir }
      $path.Key.SetValue('Path', $new, $path.Kind)
      Write-Output 'added'
    }
    'path-remove' {
      $dir = $env:WIPEMARK_SETUP_DIR
      if (-not $dir) { Fail 'WIPEMARK_SETUP_DIR is not set' }
      $path = Env-Path
      $old = $path.Value
      if ($null -eq $old) { Write-Output 'absent'; exit 0 }
      $keep = New-Object System.Collections.Generic.List[string]
      $removed = 0
      foreach ($entry in $old.Split(';')) {
        if ($entry.Trim() -and (Norm $entry) -eq (Norm $dir)) { $removed++ }
        else { $keep.Add($entry) }
      }
      if ($removed -eq 0) { Write-Output 'absent'; exit 0 }
      $new = [string]::Join(';', $keep)
      if ($new -eq '') { $path.Key.DeleteValue('Path') }
      else { $path.Key.SetValue('Path', $new, $path.Kind) }
      Write-Output 'removed'
    }
    'measure' {
      $data = $env:WIPEMARK_SETUP_DATA
      if (-not $data) { Fail 'WIPEMARK_SETUP_DATA is not set' }
      if (-not [IO.Directory]::Exists($data)) { Write-Output 0; exit 0 }
      $info = New-Object IO.DirectoryInfo $data
      if (Is-Link $info) { Write-Output 0; exit 0 }
      $bytes = Measure-Tree $info
      Write-Output ([long][Math]::Ceiling($bytes / 1024.0))
    }
    'remove-data' {
      $data = $env:WIPEMARK_SETUP_DATA
      if (-not $data) { Fail 'WIPEMARK_SETUP_DATA is not set' }
      if (-not [IO.Directory]::Exists($data)) { Write-Output 'absent'; exit 0 }
      $info = New-Object IO.DirectoryInfo $data
      if (Is-Link $info) { Remove-Link $info } else { Remove-Tree $info }
      # The layout's own parents, GigLabo\wipemark, and nothing else: a
      # WIPEMARK_DATA_DIR elsewhere leaves its parents alone.
      $parent = [IO.Path]::GetDirectoryName($data.TrimEnd('\'))
      if ($parent -and [IO.Path]::GetFileName($parent) -eq 'wipemark') {
        Remove-IfEmpty $parent
        $grand = [IO.Path]::GetDirectoryName($parent)
        if ($grand -and [IO.Path]::GetFileName($grand) -eq 'GigLabo') { Remove-IfEmpty $grand }
      }
      Write-Output 'removed'
    }
    default { Fail "unknown action '$Action' (path-add, path-remove, measure, remove-data)" }
  }
} catch {
  Fail $_.Exception.Message
}
exit 0
