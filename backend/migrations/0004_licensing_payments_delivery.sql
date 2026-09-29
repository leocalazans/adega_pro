CREATE TABLE licenses (
    tenant_id uuid NOT NULL REFERENCES tenants(id),
    unit_id uuid NOT NULL,
    installation_id text NOT NULL,
    status text NOT NULL DEFAULT 'active' CHECK (status IN ('active','suspended','cancelled')),
    expires_at bigint NOT NULL,
    grace_until bigint NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (tenant_id, unit_id) REFERENCES units(tenant_id, id),
    PRIMARY KEY (tenant_id, unit_id, installation_id)
);

CREATE TABLE license_audit (
    id bigserial PRIMARY KEY,
    tenant_id uuid NOT NULL REFERENCES tenants(id),
    unit_id uuid NOT NULL,
    installation_id text NOT NULL,
    action text NOT NULL,
    detail jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE payment_intents (
    id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL REFERENCES tenants(id),
    unit_id uuid NOT NULL,
    terminal_id uuid NOT NULL,
    provider text NOT NULL,
    provider_id text,
    external_reference text NOT NULL,
    amount_brl_cents bigint NOT NULL CHECK (amount_brl_cents > 0),
    status text NOT NULL,
    qr_payload text,
    expires_at timestamptz,
    raw jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (tenant_id, unit_id) REFERENCES units(tenant_id, id),
    FOREIGN KEY (tenant_id, terminal_id) REFERENCES terminals(tenant_id, id),
    UNIQUE(provider,provider_id),
    UNIQUE(tenant_id,external_reference)
);
CREATE INDEX idx_payment_intents_lookup ON payment_intents(tenant_id,unit_id,status,created_at DESC);

CREATE TABLE external_orders (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL REFERENCES tenants(id),
    unit_id uuid NOT NULL,
    provider text NOT NULL,
    external_id text NOT NULL,
    status text NOT NULL,
    payload jsonb NOT NULL,
    sale_uuid uuid,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (tenant_id, unit_id) REFERENCES units(tenant_id, id),
    UNIQUE(tenant_id,provider,external_id)
);
CREATE INDEX idx_external_orders_unit_status ON external_orders(tenant_id,unit_id,status,created_at);
