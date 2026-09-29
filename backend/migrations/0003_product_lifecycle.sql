ALTER TABLE products ADD COLUMN IF NOT EXISTS active boolean NOT NULL DEFAULT true;
CREATE INDEX IF NOT EXISTS idx_products_tenant_active ON products(tenant_id,active,updated_at DESC);
