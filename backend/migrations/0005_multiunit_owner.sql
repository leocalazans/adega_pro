-- Product master data stays tenant-wide. These values are the explicit per-store
-- overrides used by checkout, catalog delivery and reporting.
CREATE TABLE product_unit_settings (
    tenant_id uuid NOT NULL,
    unit_id uuid NOT NULL,
    ean text NOT NULL,
    price_brl_cents bigint CHECK (price_brl_cents >= 0),
    min_stock double precision CHECK (min_stock >= 0),
    active boolean,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, unit_id, ean),
    FOREIGN KEY (tenant_id, unit_id) REFERENCES units(tenant_id, id),
    FOREIGN KEY (tenant_id, ean) REFERENCES products(tenant_id, ean)
);
CREATE INDEX idx_product_unit_settings_lookup ON product_unit_settings(tenant_id, unit_id, ean);

CREATE TABLE promotions (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL REFERENCES tenants(id),
    -- The product key is (tenant_id, ean); validate that association in writes.
    ean text,
    title text NOT NULL CHECK (length(btrim(title)) > 0),
    starts_at timestamptz,
    ends_at timestamptz,
    active boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (ends_at IS NULL OR starts_at IS NULL OR ends_at > starts_at)
);
CREATE TABLE promotion_units (
    promotion_id uuid NOT NULL REFERENCES promotions(id) ON DELETE CASCADE,
    tenant_id uuid NOT NULL,
    unit_id uuid NOT NULL,
    price_brl_cents bigint CHECK (price_brl_cents >= 0),
    active boolean NOT NULL DEFAULT true,
    PRIMARY KEY (promotion_id, unit_id),
    FOREIGN KEY (tenant_id, unit_id) REFERENCES units(tenant_id, id)
);
CREATE INDEX idx_promotion_units_unit ON promotion_units(tenant_id, unit_id, active);

-- Owner credentials are distinct from terminal credentials. Store only a hash.
CREATE TABLE tenant_admins (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL REFERENCES tenants(id),
    email text NOT NULL,
    role text NOT NULL CHECK (role IN ('owner', 'manager')),
    api_key_hash text NOT NULL,
    active boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    last_seen_at timestamptz,
    UNIQUE(tenant_id, email),
    UNIQUE(tenant_id, api_key_hash)
);
CREATE INDEX idx_tenant_admins_auth ON tenant_admins(tenant_id, api_key_hash) WHERE active;
