$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$keyPath=Join-Path $env:USERPROFILE '.tauri\commercectrl-updater.key'
if(-not(Test-Path -LiteralPath $keyPath)){throw 'Chave privada local ausente'}
$config=Get-Content (Join-Path $root 'src-tauri\tauri.conf.json') -Raw|ConvertFrom-Json
$public=(Get-Content "$keyPath.pub" -Raw).Trim()
if($config.plugins.updater.pubkey -ne $public){throw 'Chave pública diferente da configuração; não publicar'}
$previousLicensePublic=$env:COMMERCECTRL_LICENSE_PUBLIC_KEY_B64
function Read-PrivateValue([string]$Prompt){
  $secure=Read-Host $Prompt -AsSecureString
  $pointer=[Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
  try { [Runtime.InteropServices.Marshal]::PtrToStringBSTR($pointer) }
  finally { [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($pointer) }
}
try {
  # O Tauri aceita o conteúdo da chave em TAURI_SIGNING_PRIVATE_KEY; para um
  # arquivo local, a variável correta é a variante *_PATH.
  $env:TAURI_SIGNING_PRIVATE_KEY_PATH=$keyPath
  $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD=Read-PrivateValue 'Senha da chave privada (Enter se vazia)'
  $env:COMMERCECTRL_API_URL='https://adega-pro-sndh.onrender.com'
  if(-not $env:COMMERCECTRL_LICENSE_PUBLIC_KEY_B64){
    $env:COMMERCECTRL_LICENSE_PUBLIC_KEY_B64=Read-Host 'Chave PUBLICA de licenciamento do backend (base64, diferente da chave updater)'
  }
  if([Convert]::FromBase64String($env:COMMERCECTRL_LICENSE_PUBLIC_KEY_B64).Length -ne 32){throw 'Chave pública de licenciamento inválida'}
  & (Join-Path $PSScriptRoot 'build-windows.ps1')
  $env:SUPABASE_URL='https://sqwpkwsjbbkylbdpjrlu.supabase.co'
  $env:SUPABASE_UPDATE_BUCKET='desktop-releases'
  $env:SUPABASE_SERVICE_ROLE_KEY=Read-PrivateValue 'Service Role do Supabase (oculta)'
  & (Join-Path $PSScriptRoot 'publish-update.ps1') -Version $config.version
} finally {
  Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
  Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PATH -ErrorAction SilentlyContinue
  Remove-Item Env:SUPABASE_SERVICE_ROLE_KEY -ErrorAction SilentlyContinue
  if($previousLicensePublic){$env:COMMERCECTRL_LICENSE_PUBLIC_KEY_B64=$previousLicensePublic}
  else {Remove-Item Env:COMMERCECTRL_LICENSE_PUBLIC_KEY_B64 -ErrorAction SilentlyContinue}
}
