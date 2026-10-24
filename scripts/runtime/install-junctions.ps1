param(
  [Parameter(Mandatory = $true)]
  [string]$Root
)

$ErrorActionPreference = 'Stop'
$rootPath = [IO.Path]::GetFullPath($Root).TrimEnd([char]92)
$prefix = $rootPath + [char]92
$manifest = Get-Content (Join-Path $rootPath 'runtime-junctions.json') -Raw | ConvertFrom-Json

foreach ($item in $manifest.links) {
  $link = [IO.Path]::GetFullPath((Join-Path $rootPath $item.path))
  $target = [IO.Path]::GetFullPath((Join-Path $rootPath $item.target))
  if (-not $link.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) -or
      -not $target.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Runtime junction path escapes the DSH directory'
  }
  if (-not (Test-Path $target -PathType Container)) {
    throw "Runtime junction target is missing: $target"
  }
  if (Test-Path $link) {
    throw "Runtime junction path already exists: $link"
  }
  try {
    New-Item -ItemType Junction -Path $link -Target $target | Out-Null
  }
  catch {
    # Non-NTFS install volumes cannot hold junctions; preserve compatibility.
    if (Test-Path $link) {
      Remove-Item $link -Recurse -Force
    }
    Copy-Item $target $link -Recurse
  }
  if (-not (Test-Path (Join-Path $link 'package.json') -PathType Leaf)) {
    throw "Runtime junction verification failed: $link"
  }
}
