$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$target = if ($env:TAURI_TARGET) { $env:TAURI_TARGET } else { 'x86_64-pc-windows-msvc' }
$opencodeVersion = if ($env:OPENCODE_VERSION) { $env:OPENCODE_VERSION } else { 'v1.19.4' }
$officeCliVersion = if ($env:OFFICECLI_VERSION) { $env:OFFICECLI_VERSION } else { 'v1.0.152' }

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '../..')
$binaries = Join-Path $repoRoot 'src-tauri/binaries'
$staging = Join-Path $env:RUNNER_TEMP 'kubera-sidecars'
New-Item -ItemType Directory -Force -Path $binaries, $staging | Out-Null

$opencodeZip = Join-Path $staging 'opencode-windows-x64-baseline.zip'
$opencodeUrl = "https://github.com/anomalyco/opencode/releases/download/$opencodeVersion/opencode-windows-x64-baseline.zip"
$officeCliUrl = "https://github.com/iOfficeAI/OfficeCLI/releases/download/$officeCliVersion/officecli-win-x64.exe"
$officeCliDownload = Join-Path $staging 'officecli-win-x64.exe'

Write-Host "Downloading pinned OpenCode $opencodeVersion"
Invoke-WebRequest -Uri $opencodeUrl -OutFile $opencodeZip
$opencodeExtract = Join-Path $staging 'opencode'
Expand-Archive -Path $opencodeZip -DestinationPath $opencodeExtract -Force
$opencodeBinary = Get-ChildItem -Path $opencodeExtract -Recurse -File -Filter 'opencode.exe' | Select-Object -First 1
if (-not $opencodeBinary) { throw "Pinned OpenCode archive did not contain opencode.exe." }
Copy-Item $opencodeBinary.FullName (Join-Path $binaries "opencode-cli-$target.exe") -Force

Write-Host "Downloading pinned OfficeCLI $officeCliVersion"
Invoke-WebRequest -Uri $officeCliUrl -OutFile $officeCliDownload
if ((Get-Item $officeCliDownload).Length -lt 1000000) { throw "OfficeCLI download is unexpectedly small." }
Copy-Item $officeCliDownload (Join-Path $binaries "officecli-$target.exe") -Force

$manifest = [ordered]@{
  target = $target
  opencode = [ordered]@{
    version = $opencodeVersion
    source = $opencodeUrl
    file = "opencode-cli-$target.exe"
    sha256 = (Get-FileHash (Join-Path $binaries "opencode-cli-$target.exe") -Algorithm SHA256).Hash.ToLower()
  }
  officecli = [ordered]@{
    version = $officeCliVersion
    source = $officeCliUrl
    file = "officecli-$target.exe"
    sha256 = (Get-FileHash (Join-Path $binaries "officecli-$target.exe") -Algorithm SHA256).Hash.ToLower()
  }
}
$manifest | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $binaries 'sidecar-manifest.json') -Encoding utf8

Write-Host "Prepared bundled sidecars:"
Get-ChildItem $binaries | Select-Object Name, Length
