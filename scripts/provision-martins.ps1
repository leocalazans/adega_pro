param([string]$ApiUrl='https://adega-pro-sndh.onrender.com')
$ErrorActionPreference='Stop'
$email=Read-Host 'E-mail SuperAdmin'
$password=Read-Host 'Senha SuperAdmin (oculta)' -AsSecureString
$credential=New-Object System.Management.Automation.PSCredential($email,$password)
$session=New-Object Microsoft.PowerShell.Commands.WebRequestSession
try {
  $body=@{email=$email;password=$credential.GetNetworkCredential().Password}|ConvertTo-Json
  Invoke-RestMethod "$ApiUrl/api/v1/platform/auth/login" -Method Post -WebSession $session -ContentType 'application/json' -Body $body | Out-Null
  $body=$null
  $tenants=Invoke-RestMethod "$ApiUrl/api/v1/platform/tenants" -WebSession $session
  if(@($tenants|Where-Object name -eq 'Mercadinho Martins').Count){throw 'Martins já existe. Operação interrompida para não duplicar o cliente; consulte a unidade existente.'}
  $tenant=Invoke-RestMethod "$ApiUrl/api/v1/platform/tenants" -Method Post -WebSession $session -ContentType 'application/json' -Body (@{name='Mercadinho Martins';unit_name='Matriz Centro';plan='starter';trial_days=30}|ConvertTo-Json)
  Write-Host "Tenant criado: $($tenant.tenant_id). Unidade: $($tenant.unit_id)."
  $activation=Invoke-RestMethod "$ApiUrl/api/v1/platform/tenants/$($tenant.tenant_id)/units/$($tenant.unit_id)/activation-codes" -Method Post -WebSession $session -ContentType 'application/json' -Body '{"valid_days":7,"max_uses":1}'
  Write-Host 'Código de uso único (copie para o instalador; não publique):'
  Write-Host $activation.code
  Write-Host 'O terminal será criado ao ativar. O administrador local deve trocar a senha no primeiro acesso.'
} finally {
  $body=$null; $credential=$null; $password=$null
  try { Invoke-RestMethod "$ApiUrl/api/v1/platform/auth/logout" -Method Post -WebSession $session | Out-Null } catch {}
}
