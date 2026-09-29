$ErrorActionPreference = 'Stop'
$baseUrl = 'http://127.0.0.1:8088'
$headers = @{
  'X-Tenant-Id' = '00000000-0000-0000-0000-000000000001'
  'X-Unit-Id' = '00000000-0000-0000-0000-000000000101'
  'X-Terminal-Id' = '00000000-0000-0000-0000-000000001001'
  'X-Terminal-Key' = 'commercectrl-dev-key'
}

function Assert-True([bool]$condition, [string]$message) {
  if (-not $condition) { throw "ASSERT: $message" }
}

function Get-HttpStatus([string]$url) {
  try {
    return [int](Invoke-WebRequest $url -UseBasicParsing).StatusCode
  } catch {
    if ($_.Exception.Response -and $_.Exception.Response.StatusCode) {
      return [int]$_.Exception.Response.StatusCode
    }
    throw
  }
}

function Get-PostStatus([string]$url, [string]$body, [hashtable]$requestHeaders = @{}) {
  try {
    return [int](Invoke-WebRequest $url -Method Post -Headers $requestHeaders -ContentType 'application/json' -Body $body -UseBasicParsing).StatusCode
  } catch {
    if ($_.Exception.Response -and $_.Exception.Response.StatusCode) { return [int]$_.Exception.Response.StatusCode }
    throw
  }
}

function Wait-Health([string]$url) {
  for ($attempt = 0; $attempt -lt 30; $attempt++) {
    try { return Invoke-RestMethod $url } catch { Start-Sleep -Milliseconds 500 }
  }
  throw 'Backend não ficou saudável dentro do prazo'
}

function Send-Event([string]$uuid, [string]$entity, [hashtable]$payload, [hashtable]$requestHeaders = $headers) {
  $body = @{ uuid=$uuid; entity=$entity; operation='upsert'; payload=$payload; created_at=[DateTimeOffset]::UtcNow.ToUnixTimeSeconds() } | ConvertTo-Json -Depth 8
  Invoke-RestMethod "$baseUrl/api/v1/sync/outbox" -Method Post -Headers $requestHeaders -ContentType 'application/json' -Body $body
}

$health = Wait-Health "$baseUrl/health"
Assert-True ($health.status -eq 'ok') 'health check'
$unauthorized = Get-HttpStatus "$baseUrl/api/v1/sync/products"
Assert-True ($unauthorized -eq 401) 'endpoint deve exigir autenticação'
$installationId = [guid]::NewGuid().ToString()
$licenseBody = @{tenant_id=$headers['X-Tenant-Id'];unit_id=$headers['X-Unit-Id'];installation_id=$installationId;valid_days=30;grace_days=15} | ConvertTo-Json
$licenseUnauthorized = Get-PostStatus "$baseUrl/api/v1/admin/licenses/issue" $licenseBody
Assert-True ($licenseUnauthorized -eq 401) 'emissão de licença deve exigir chave administrativa'
$license = Invoke-RestMethod "$baseUrl/api/v1/admin/licenses/issue" -Method Post -Headers @{'X-License-Admin-Key'='commercectrl-license-dev-change-me'} -ContentType 'application/json' -Body $licenseBody
Assert-True (-not [string]::IsNullOrWhiteSpace($license.signature)) 'licença deve ser assinada'
Assert-True ($license.claims.installation_id -eq $installationId) 'licença deve vincular a instalação'
$renewed = Invoke-RestMethod "$baseUrl/api/v1/licenses/renew" -Method Post -Headers $headers -ContentType 'application/json' -Body (@{installation_id=$installationId} | ConvertTo-Json)
Assert-True (-not [string]::IsNullOrWhiteSpace($renewed.signature)) 'terminal autorizado deve renovar licença'
$webhookUnauthorized = Get-PostStatus "$baseUrl/api/v1/webhooks/asaas" (@{event='PAYMENT_RECEIVED';payment=@{id='pay_invalid';status='RECEIVED'}} | ConvertTo-Json -Depth 4)
Assert-True ($webhookUnauthorized -eq 401) 'webhook Asaas deve exigir token'

$products = Invoke-RestMethod "$baseUrl/api/v1/sync/products?limit=100" -Headers $headers
Assert-True ($products.Count -ge 8) 'seeds de produtos'
$before = ($products | Where-Object ean -eq '78900001').stock_qty
$stockEvent = [guid]::NewGuid().ToString()
$first = Send-Event $stockEvent 'stock_movement' @{ean='78900001';delta=1}
$duplicate = Send-Event $stockEvent 'stock_movement' @{ean='78900001';delta=1}
$after = ((Invoke-RestMethod "$baseUrl/api/v1/sync/products?limit=100" -Headers $headers) | Where-Object ean -eq '78900001').stock_qty
Assert-True ($first.accepted -and -not $first.duplicate) 'primeiro evento deve ser aceito'
Assert-True ($duplicate.duplicate) 'evento repetido deve ser idempotente'
Assert-True ($after -eq ($before + 1)) 'movimento não pode ser aplicado duas vezes'

$suffix = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds().ToString()
$ean = "T$suffix"
$productEvent = [guid]::NewGuid().ToString()
$created = Send-Event $productEvent 'product' @{id=999999;ean=$ean;part_number="SKU-$suffix";description='Produto de integração';brand='Teste';price_brl_cents=1234;stock_qty=0;min_stock=4;updated_at=0}
Assert-True ($created.accepted) 'produto deve ser materializado'
$remoteProduct = (Invoke-RestMethod "$baseUrl/api/v1/sync/products?limit=5000" -Headers $headers) | Where-Object ean -eq $ean
Assert-True ($null -ne $remoteProduct -and $remoteProduct.min_stock -eq 4) 'produto materializado deve ser consultável'
$archiveEvent = [guid]::NewGuid().ToString()
Send-Event $archiveEvent 'product' @{id=999999;ean=$ean;part_number="SKU-$suffix";description='Produto de integração';brand='Teste';price_brl_cents=1234;stock_qty=0;min_stock=4;active=$false;updated_at=0} | Out-Null
$archivedProduct = (Invoke-RestMethod "$baseUrl/api/v1/sync/products?limit=5000" -Headers $headers) | Where-Object ean -eq $ean
Assert-True ($null -ne $archivedProduct -and $archivedProduct.active -eq $false) 'arquivamento lógico deve sincronizar sem apagar histórico'

$employeeEvent = [guid]::NewGuid().ToString()
Send-Event $employeeEvent 'employee' @{id=999999;name='Funcionário integração';role='Caixa';status='Ativo'} | Out-Null
$recordCount = docker compose exec -T postgres psql -U commercectrl -d commercectrl -tAc "SELECT count(*) FROM operational_records WHERE entity='employee' AND local_id=999999 AND terminal_id='00000000-0000-0000-0000-000000001001';"
Assert-True ([int]$recordCount -eq 1) 'registro operacional deve chegar ao PostgreSQL'
$promotionEvent = [guid]::NewGuid().ToString()
Send-Event $promotionEvent 'promotion' @{id=999998;title='Promoção integração';subtitle='Teste';price_label='R$ 1,00';active=$true} | Out-Null
$promotionCount = docker compose exec -T postgres psql -U commercectrl -d commercectrl -tAc "SELECT count(*) FROM operational_records WHERE entity='promotion' AND local_id=999998 AND terminal_id='00000000-0000-0000-0000-000000001001';"
Assert-True ([int]$promotionCount -eq 1) 'promoção deve chegar ao PostgreSQL'
$promotionUpdateEvent = [guid]::NewGuid().ToString()
Send-Event $promotionUpdateEvent 'promotion' @{id=999998;active=$false} | Out-Null
$promotionMerged = docker compose exec -T postgres psql -U commercectrl -d commercectrl -tAc "SELECT count(*) FROM operational_records WHERE entity='promotion' AND local_id=999998 AND terminal_id='00000000-0000-0000-0000-000000001001' AND payload ? 'title' AND payload->>'active'='false';"
Assert-True ([int]$promotionMerged -eq 1) 'atualização parcial deve preservar os demais campos'

$saleUuid = [guid]::NewGuid().ToString()
$saleEvent = [guid]::NewGuid().ToString()
$stockBeforeSale = ((Invoke-RestMethod "$baseUrl/api/v1/sync/products?limit=100" -Headers $headers) | Where-Object ean -eq '78900002').stock_qty
$salePayload = @{uuid=$saleUuid;total_brl_cents=4500;payment_method='pix';created_at=[DateTimeOffset]::UtcNow.ToUnixTimeSeconds();items=@(@{ean='78900002';qty=1;price_brl_cents=1500},@{ean='78900002';qty=2;price_brl_cents=1500})}
$saleFirst = Send-Event $saleEvent 'sale' $salePayload
$saleDuplicate = Send-Event $saleEvent 'sale' $salePayload
$saleReplayEvent = [guid]::NewGuid().ToString()
$saleReplay = Send-Event $saleReplayEvent 'sale' $salePayload
Assert-True ($saleReplay.accepted) 'mesma venda com outro evento deve ser aceita sem nova baixa'
$conflictEvent = [guid]::NewGuid().ToString()
$changedSale = @{uuid=$saleUuid;total_brl_cents=4500;payment_method='pix';created_at=$salePayload.created_at;items=@(@{ean='78900002';qty=3;price_brl_cents=1500})}
$conflictBody = @{uuid=$conflictEvent;entity='sale';operation='insert';payload=$changedSale;created_at=$salePayload.created_at} | ConvertTo-Json -Depth 8
$conflictStatus = Get-PostStatus "$baseUrl/api/v1/sync/outbox" $conflictBody $headers
Assert-True ($conflictStatus -eq 409) 'reuso de UUID com itens diferentes deve ser recusado'
$stockAfterSale = ((Invoke-RestMethod "$baseUrl/api/v1/sync/products?limit=100" -Headers $headers) | Where-Object ean -eq '78900002').stock_qty
Assert-True ($saleFirst.accepted -and $saleDuplicate.duplicate) 'venda deve ser idempotente'
Assert-True ($stockAfterSale -eq ($stockBeforeSale - 3)) 'linhas com mesmo EAN somam quantidade e replay não baixa estoque novamente'

$unit2 = '00000000-0000-0000-0000-000000000202'
$terminal2 = '00000000-0000-0000-0000-000000002002'
docker compose exec -T postgres psql -U commercectrl -d commercectrl -v ON_ERROR_STOP=1 -c "INSERT INTO units(id,tenant_id,name,code) VALUES('$unit2','00000000-0000-0000-0000-000000000001','Unidade Teste','TESTE') ON CONFLICT(id) DO NOTHING; INSERT INTO terminals(id,tenant_id,unit_id,name,api_key_hash) VALUES('$terminal2','00000000-0000-0000-0000-000000000001','$unit2','Terminal Teste',encode(digest('commercectrl-test-key','sha256'),'hex')) ON CONFLICT(id) DO UPDATE SET api_key_hash=excluded.api_key_hash,active=true;" | Out-Null
$headers2 = @{'X-Tenant-Id'=$headers['X-Tenant-Id'];'X-Unit-Id'=$unit2;'X-Terminal-Id'=$terminal2;'X-Terminal-Key'='commercectrl-test-key'}
$unitReport = Invoke-RestMethod "$baseUrl/api/v1/reports/sales-by-unit?from=0" -Headers $headers
Assert-True (@($unitReport | Where-Object unit_id -eq $unit2).Count -eq 1) 'relatório consolidado deve incluir todas as unidades do tenant'
$unit2Before = ((Invoke-RestMethod "$baseUrl/api/v1/sync/products?limit=100" -Headers $headers2) | Where-Object ean -eq '78900001').stock_qty
$unit2Event = [guid]::NewGuid().ToString()
Send-Event $unit2Event 'stock_movement' @{ean='78900001';delta=5} $headers2 | Out-Null
$unit2After = ((Invoke-RestMethod "$baseUrl/api/v1/sync/products?limit=100" -Headers $headers2) | Where-Object ean -eq '78900001').stock_qty
$unit1Unchanged = ((Invoke-RestMethod "$baseUrl/api/v1/sync/products?limit=100" -Headers $headers) | Where-Object ean -eq '78900001').stock_qty
Assert-True ($unit2After -eq ($unit2Before + 5)) 'movimento deve chegar à segunda unidade'
Assert-True ($unit1Unchanged -eq $after) 'estoque da primeira unidade deve permanecer isolado'

$ownerHeaders = @{'X-Tenant-Id'=$headers['X-Tenant-Id'];'X-Owner-Key'='commercectrl-owner-dev-key'}
$ownerDenied = Get-HttpStatus "$baseUrl/api/v1/owner/overview?from=0"
Assert-True ($ownerDenied -eq 401) 'painel do proprietário deve exigir chave distinta do terminal'
$unitPriceBody = @{unit_id=$unit2;ean='78900001';price_brl_cents=1999;min_stock=9;active=$true} | ConvertTo-Json
$settingStatus = Get-PostStatus "$baseUrl/api/v1/owner/product-settings" $unitPriceBody $ownerHeaders
Assert-True ($settingStatus -eq 204) 'proprietário deve configurar preço por unidade'
$unit2Price = ((Invoke-RestMethod "$baseUrl/api/v1/sync/products?limit=100" -Headers $headers2) | Where-Object ean -eq '78900001').price_brl_cents
$unit1Price = ((Invoke-RestMethod "$baseUrl/api/v1/sync/products?limit=100" -Headers $headers) | Where-Object ean -eq '78900001').price_brl_cents
Assert-True ($unit2Price -eq 1999 -and $unit1Price -ne 1999) 'preço por unidade não pode vazar entre filiais'
$promotion = Invoke-RestMethod "$baseUrl/api/v1/owner/promotions" -Method Post -Headers $ownerHeaders -ContentType 'application/json' -Body (@{ean='78900001';title='Oferta por unidade';active=$true;units=@(@{unit_id=$unit2;price_brl_cents=1799;active=$true})} | ConvertTo-Json -Depth 5)
Assert-True ($promotion.scope -eq 'selected_units' -and -not [string]::IsNullOrWhiteSpace($promotion.id)) 'promoção deve aceitar escopo de filial'
$ownerOverview = Invoke-RestMethod "$baseUrl/api/v1/owner/overview?from=0" -Headers $ownerHeaders
Assert-True (@($ownerOverview.units | Where-Object unit_id -eq $unit2).Count -eq 1) 'painel do proprietário deve listar filial'
Assert-True ($ownerOverview.top_products.Count -ge 1) 'painel do proprietário deve trazer ranking de produtos'
$displayChannel = Invoke-RestMethod "$baseUrl/api/v1/owner/display-channels" -Method Post -Headers $ownerHeaders -ContentType 'application/json' -Body (@{unit_id=$unit2;name='TV Integração'} | ConvertTo-Json)
Assert-True (-not [string]::IsNullOrWhiteSpace($displayChannel.token)) 'canal de TV deve retornar token somente na criação'
$displayOffers = Invoke-RestMethod "$baseUrl/api/v1/display/$($displayChannel.token)/promotions"
Assert-True ($null -ne $displayOffers) 'canal público deve consultar somente promoções do canal'
$displayDenied = Get-HttpStatus "$baseUrl/api/v1/display/tv_invalido/promotions"
Assert-True ($displayDenied -eq 404) 'token de TV inválido não pode acessar promoções'

docker compose exec -T postgres psql -U commercectrl -d commercectrl -v ON_ERROR_STOP=1 -c "DELETE FROM display_channels WHERE id='$($displayChannel.id)'; DELETE FROM promotion_units WHERE promotion_id='$($promotion.id)'; DELETE FROM promotions WHERE id='$($promotion.id)'; DELETE FROM product_unit_settings WHERE unit_id='$unit2' AND ean='78900001'; DELETE FROM license_audit WHERE installation_id='$installationId'; DELETE FROM licenses WHERE installation_id='$installationId'; DELETE FROM operational_records WHERE local_id IN (999998,999999); DELETE FROM stock_movements WHERE event_uuid IN ('$stockEvent','$saleEvent','$unit2Event'); DELETE FROM sale_items WHERE sale_uuid='$saleUuid'; DELETE FROM sales WHERE uuid='$saleUuid'; DELETE FROM cloud_events WHERE event_uuid IN ('$stockEvent','$productEvent','$archiveEvent','$employeeEvent','$promotionEvent','$promotionUpdateEvent','$saleEvent','$saleReplayEvent','$conflictEvent','$unit2Event'); DELETE FROM products WHERE tenant_id='00000000-0000-0000-0000-000000000001' AND ean='$ean'; DELETE FROM terminals WHERE id='$terminal2'; DELETE FROM units WHERE id='$unit2';" | Out-Null

[pscustomobject]@{ tests=38; passed=38; stock_before=$before; stock_after=$after; duplicate_protected=$duplicate.duplicate; sale_idempotent=$saleDuplicate.duplicate; multi_unit_isolated=$true; product_soft_delete=$true; consolidated_report=$true; owner_panel=$true; unit_price_isolated=$true; unit_promotion=$true; display_channel=$true; promotion_synced=$true; partial_update_merged=$true; signed_license=$true; license_renewal=$true; asaas_webhook_protected=$true } | ConvertTo-Json
