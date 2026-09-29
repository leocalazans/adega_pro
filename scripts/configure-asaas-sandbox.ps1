$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$envFile = Join-Path $root '.env'

$apiKey = Read-Host 'Cole a API Key do Sandbox Asaas' -AsSecureString
$customerId = Read-Host 'Informe o ID do cliente padrão de balcão (cus_...)'
$webhookToken = Read-Host 'Defina o token do webhook (mínimo 32 caracteres)' -AsSecureString

$ptr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($apiKey)
$webhookPtr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($webhookToken)
try {
  $plainApiKey = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($ptr)
  $plainWebhook = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($webhookPtr)
  if (-not $plainApiKey.StartsWith('$aact_hmlg_')) { throw 'A chave não parece pertencer ao Sandbox Asaas' }
  if (-not $customerId.StartsWith('cus_')) { throw 'ID de cliente Asaas inválido' }
  if ($plainWebhook.Length -lt 32 -or $plainWebhook.Contains(' ')) { throw 'Token do webhook deve ter 32+ caracteres e não conter espaços' }
  $existing = if (Test-Path $envFile) { Get-Content $envFile | Where-Object { $_ -notmatch '^ASAAS_' } } else { @() }
  $content = @($existing) + @(
    'ASAAS_BASE_URL=https://api-sandbox.asaas.com/v3',
    "ASAAS_API_KEY=$plainApiKey",
    "ASAAS_CUSTOMER_ID=$customerId",
    "ASAAS_WEBHOOK_TOKEN=$plainWebhook"
  )
  Set-Content -LiteralPath $envFile -Value $content -Encoding UTF8
  Write-Host 'Asaas Sandbox configurado em .env. Reinicie o backend com: docker compose up -d --build backend' -ForegroundColor Green
} finally {
  [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($ptr)
  [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($webhookPtr)
  $plainApiKey = $null
  $plainWebhook = $null
}
