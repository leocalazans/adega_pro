param(
  [Parameter(Mandatory=$true)][string]$InstallationId,
  [int]$ValidDays = 30,
  [int]$GraceDays = 15,
  [string]$OutputPath = '.\commercectrl-license.json'
)
$ErrorActionPreference = 'Stop'
$adminKey = $env:LICENSE_ADMIN_KEY
if ([string]::IsNullOrWhiteSpace($adminKey)) { throw 'Defina LICENSE_ADMIN_KEY no ambiente.' }
$body = @{
  tenant_id = $env:COMMERCECTRL_TENANT_ID
  unit_id = $env:COMMERCECTRL_UNIT_ID
  installation_id = $InstallationId
  valid_days = $ValidDays
  grace_days = $GraceDays
} | ConvertTo-Json
if ([string]::IsNullOrWhiteSpace($env:COMMERCECTRL_API_URL)) { throw 'Defina COMMERCECTRL_API_URL.' }
$token = Invoke-RestMethod "$($env:COMMERCECTRL_API_URL)/api/v1/admin/licenses/issue" -Method Post -Headers @{'X-License-Admin-Key'=$adminKey} -ContentType 'application/json' -Body $body
$token | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $OutputPath -Encoding UTF8
Write-Host "Licença assinada gravada em $OutputPath"
