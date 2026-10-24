param(
  [Parameter(Mandatory = $true)]
  [string]$Root
)

$ErrorActionPreference = 'Stop'
$archive = Join-Path $Root 'dsh.tar'
$tar = Join-Path $env:SystemRoot 'System32\tar.exe'
$lists = @(Get-ChildItem $Root -Filter 'dsh-*.files' | Sort-Object Name)
$processes = @()
$errorFiles = @()

try {
  if ($lists.Count -eq 0) {
    & $tar -xf $archive -C $Root --no-recursion
    if ($LASTEXITCODE -ne 0) {
      throw "DSH archive extraction failed: $LASTEXITCODE"
    }
    exit 0
  }

  foreach ($list in $lists) {
    $errorFile = $list.FullName + '.err'
    $errorFiles += $errorFile
    $arguments = @(
      '-xf', ('"' + $archive + '"'),
      '-C', ('"' + $Root + '"'),
      '-T', ('"' + $list.FullName + '"'),
      '--no-recursion'
    )
    $process = Start-Process -FilePath $tar -ArgumentList $arguments -WindowStyle Hidden -PassThru -RedirectStandardError $errorFile
    # PowerShell 5 needs a retained handle to expose ExitCode after WaitForExit.
    $null = $process.Handle
    $processes += $process
  }

  foreach ($process in $processes) {
    $process.WaitForExit()
    $process.Refresh()
  }
  $failed = @($processes | Where-Object { $_.ExitCode -ne 0 })
  if ($failed.Count -gt 0) {
    foreach ($file in $errorFiles) {
      if (Test-Path $file) {
        Get-Content $file | Write-Output
      }
    }
    throw "Parallel DSH extraction failed with exit codes: $($processes.ExitCode -join ', ')"
  }
}
finally {
  foreach ($process in $processes) {
    if (-not $process.HasExited) {
      $process.Kill()
    }
  }
  foreach ($file in $errorFiles) {
    Remove-Item $file -Force -ErrorAction SilentlyContinue
  }
}
