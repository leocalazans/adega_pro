param(
  [Parameter(Mandatory = $true)][string]$Version,
  [switch]$SkipChecks
)
# Build e publicacao do instalador assinado sem depender do GitHub Actions.
# Uso:
#   1. Preencha SUPABASE_SERVICE_ROLE_KEY em .env.release (arquivo ignorado pelo git).
#   2. powershell -ExecutionPolicy Bypass -File scripts\release-local.ps1 -Version 0.1.4
# A chave de assinatura e lida de %USERPROFILE%\.tauri\commercectrl-updater.key,
# a mesma que assina as versoes ja publicadas (clientes antigos so aceitam essa chave).
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root

$envFile = Join-Path $root ".env.release"
if (Test-Path $envFile) {
  foreach ($line in Get-Content $envFile) {
    $trim = $line.Trim()
    if (-not $trim -or $trim.StartsWith("#")) { continue }
    $idx = $trim.IndexOf("=")
    if ($idx -lt 1) { continue }
    $key = $trim.Substring(0, $idx).Trim()
    $value = $trim.Substring($idx + 1).Trim().Trim('"')
    if (-not (Get-Item "env:$key" -ErrorAction SilentlyContinue)) { Set-Item -Path "env:$key" -Value $value }
  }
}
if (-not $env:SUPABASE_UPDATE_BUCKET) { $env:SUPABASE_UPDATE_BUCKET = "desktop-releases" }

foreach ($name in @("SUPABASE_URL", "SUPABASE_SERVICE_ROLE_KEY")) {
  if (-not (Get-Item "env:$name" -ErrorAction SilentlyContinue).Value) {
    throw "Defina $name em .env.release antes de publicar."
  }
}
if (-not $env:TAURI_SIGNING_PRIVATE_KEY) {
  $signingKey = Join-Path $env:USERPROFILE ".tauri\commercectrl-updater.key"
  if (-not (Test-Path $signingKey)) {
    throw "Chave de assinatura nao encontrada em $signingKey"
  }
  # O CLI espera o conteudo da chave (nao o caminho).
  $env:TAURI_SIGNING_PRIVATE_KEY = ([System.IO.File]::ReadAllText($signingKey)).Trim()
}
if (-not $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) {
  throw "Defina TAURI_SIGNING_PRIVATE_KEY_PASSWORD em .env.release (a chave local e criptografada)."
}

Write-Output "== Versao $Version =="
function Write-Utf8NoBom([string]$Path, [string]$Text) {
  [System.IO.File]::WriteAllText($Path, $Text, (New-Object System.Text.UTF8Encoding($false)))
}
$confPath = Join-Path $root "src-tauri\tauri.conf.json"
$conf = Get-Content $confPath -Raw | ConvertFrom-Json
$conf.version = $Version
$conf.plugins.updater.endpoints = @("$env:SUPABASE_URL/storage/v1/object/public/$env:SUPABASE_UPDATE_BUCKET/latest.json")
Write-Utf8NoBom $confPath ($conf | ConvertTo-Json -Depth 20)

$cargoPath = Join-Path $root "src-tauri\Cargo.toml"
$cargo = Get-Content $cargoPath -Raw
$cargoRegex = [regex]'(?m)^version = "[^"]+"'
$cargo = $cargoRegex.Replace($cargo, "version = `"$Version`"", 1)
Write-Utf8NoBom $cargoPath $cargo

$lockPath = Join-Path $root "src-tauri\Cargo.lock"
if (Test-Path $lockPath) {
  $lock = Get-Content $lockPath -Raw
  $lock = [regex]::Replace($lock, '(\[\[package\]\]\r?\nname = "commercectrl-pdv"\r?\nversion = ")[^"]+(")', ('${1}' + $Version + '${2}'))
  Write-Utf8NoBom $lockPath $lock
}

if (-not $SkipChecks) {
  Write-Output "== Validadando frontend =="
  Push-Location (Join-Path $root "frontend-svelte")
  try {
    npm run check
    if ($LASTEXITCODE -ne 0) { throw "svelte-check falhou" }
    npm test
    if ($LASTEXITCODE -ne 0) { throw "testes falharam" }
  } finally {
    Pop-Location
  }
}

Write-Output "== Compilando instaladores assinados =="
$sdkLib = "C:\Program Files (x86)\Windows Kits\10\Lib\10.0.18362.0\um\x64"
if (Test-Path $sdkLib) { $env:LIB = "$sdkLib;$env:LIB" }
npx --yes @tauri-apps/cli@latest build
if ($LASTEXITCODE -ne 0) { throw "tauri build falhou" }

Write-Output "== Publicando no Supabase Storage =="
& (Join-Path $PSScriptRoot "publish-update.ps1") -Version $Version
if ($LASTEXITCODE -ne 0) { throw "publicacao falhou" }

$manifestUrl = "$env:SUPABASE_URL/storage/v1/object/public/$env:SUPABASE_UPDATE_BUCKET/latest.json"
$manifest = Invoke-RestMethod -Uri $manifestUrl -TimeoutSec 60
if ($manifest.version -ne $Version) { throw "latest.json publicado com versao $($manifest.version), esperada $Version" }
$installerUrl = $manifest.platforms.'windows-x86_64'.url
$head = Invoke-WebRequest -Uri $installerUrl -Method Head -TimeoutSec 60
Write-Output "OK: $manifestUrl"
Write-Output "OK: $installerUrl (HTTP $($head.StatusCode))"
