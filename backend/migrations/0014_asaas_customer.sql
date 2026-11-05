ALTER TABLE tenant_subscriptions ADD COLUMN IF NOT EXISTS asaas_customer_id text;
CREATE INDEX IF NOT EXISTS tenant_subscriptions_asaas_customer_idx ON tenant_subscriptions(asaas_customer_id);
