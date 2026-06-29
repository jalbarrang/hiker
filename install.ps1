<#
.SYNOPSIS
  hiker installer for Windows — channel-aware, mirrors the POSIX `install` UX.

.EXAMPLE
  # stable (default)
  irm https://raw.githubusercontent.com/jalbarrang/hiker/stable/install.ps1 | iex

.EXAMPLE
  # pass options: download + run with args
  & ([scriptblock]::Create((irm https://raw.githubusercontent.com/jalbarrang/hiker/stable/install.ps1))) -Channel beta

.EXAMPLE
  & ([scriptblock]::Create((irm https://raw.githubusercontent.com/jalbarrang/hiker/stable/install.ps1))) -Version 1.2.3

.NOTES
  Env overrides:
    HIKER_REPO         GitHub "owner/repo" to pull releases from
    HIKER_INSTALL_DIR  where to drop the binary (default: $HOME\.hiker\bin)
#>
[CmdletBinding()]
param(
  [ValidateSet('stable', 'beta')]
  [string]$Channel = 'stable',
  [string]$Version = ''
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Write-Info { param([string]$Msg) Write-Host $Msg -ForegroundColor DarkGray }
function Die       { param([string]$Msg) Write-Host "error: $Msg" -ForegroundColor Red; exit 1 }

# ── Config (EDIT ME: the GitHub repo that publishes hiker releases) ──────────
$Repo       = if ($env:HIKER_REPO)        { $env:HIKER_REPO }        else { 'jalbarrang/hiker' }
$InstallDir = if ($env:HIKER_INSTALL_DIR) { $env:HIKER_INSTALL_DIR } else { Join-Path $HOME '.hiker\bin' }
$Version    = $Version -replace '^v', ''

# ── Detect target triple (only x86_64 Windows is published) ──────────────────
$arch = $env:PROCESSOR_ARCHITECTURE
switch ($arch) {
  'AMD64' { $target = 'x86_64-pc-windows-msvc' }
  default { Die "unsupported architecture: $arch (only x86_64 Windows is published)" }
}
$ext = 'zip'

# GitHub API requires a TLS 1.2 handshake on older Windows PowerShell.
try { [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 } catch {}

# ── Resolve the release tag for the chosen channel ───────────────────────────
$api = "https://api.github.com/repos/$Repo"
if ($Version) {
  $tag = "v$Version"
}
elseif ($Channel -eq 'stable') {
  $tag = (Invoke-RestMethod -Uri "$api/releases/latest").tag_name
}
else {
  # First release flagged prerelease.
  $rel = Invoke-RestMethod -Uri "$api/releases?per_page=30" | Where-Object { $_.prerelease } | Select-Object -First 1
  if ($rel) { $tag = $rel.tag_name }
}
if (-not $tag) { Die "no release found for channel '$Channel' in $Repo" }
$resolved = $tag -replace '^v', ''

# ── Skip if already installed ────────────────────────────────────────────────
$existing = Get-Command hiker -ErrorAction SilentlyContinue
if ($existing) {
  $cur = (& $existing.Source --version 2>$null) -replace '^\S+\s+', ''
  if ($cur -eq $resolved) { Write-Info "hiker $resolved already installed"; exit 0 }
}

# ── Download + extract ───────────────────────────────────────────────────────
$asset = "hiker-$target.$ext"
$url   = "https://github.com/$Repo/releases/download/$tag/$asset"
$tmp   = Join-Path ([IO.Path]::GetTempPath()) ("hiker-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $tmp -Force | Out-Null
try {
  $zip = Join-Path $tmp $asset
  Write-Info "downloading $asset ($tag)"
  try { Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing }
  catch { Die "download failed: $url" }

  # Verify checksum if published alongside the asset.
  try {
    $sumFile = "$zip.sha256"
    Invoke-WebRequest -Uri "$url.sha256" -OutFile $sumFile -UseBasicParsing -ErrorAction Stop
    $expected = ((Get-Content $sumFile -Raw).Trim() -split '\s+')[0].ToLower()
    $actual   = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
    if ($expected -ne $actual) { Die "checksum mismatch" }
    Write-Info "checksum ok"
  } catch {
    if ($_.Exception.Message -eq 'checksum mismatch') { throw }
    # no .sha256 published — skip verification
  }

  Expand-Archive -Path $zip -DestinationPath $tmp -Force
  New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
  Copy-Item -Path (Join-Path $tmp 'hiker.exe') -Destination (Join-Path $InstallDir 'hiker.exe') -Force
  Write-Info "installed hiker $resolved -> $InstallDir\hiker.exe"
}
finally {
  Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

# ── Add to the user PATH (persisted) + current session ───────────────────────
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (($userPath -split ';') -notcontains $InstallDir) {
  $newPath = if ($userPath) { "$userPath;$InstallDir" } else { $InstallDir }
  [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
  Write-Info "added $InstallDir to your user PATH (restart the terminal to pick it up)"
}
if (($env:Path -split ';') -notcontains $InstallDir) { $env:Path = "$env:Path;$InstallDir" }

Write-Host "run: hiker --version"
