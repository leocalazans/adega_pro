CREATE TABLE platform_activation_codes (
    id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    unit_id uuid NOT NULL,
    code_hash text NOT NULL UNIQUE,
    expires_at timestamptz NOT NULL,
    max_uses integer NOT NULL DEFAULT 1 CHECK (max_uses > 0),
    uses integer NOT NULL DEFAULT 0 CHECK (uses >= 0),
    revoked_at timestamptz,
    created_by uuid REFERENCES platform_admins(id),
    created_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (tenant_id, unit_id) REFERENCES units(tenant_id, id)
);
CREATE INDEX platform_activation_codes_tenant_idx ON platform_activation_codes(tenant_id,unit_id,expires_at);
