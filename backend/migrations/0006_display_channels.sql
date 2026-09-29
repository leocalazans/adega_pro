ALTER TABLE promotions ADD COLUMN subtitle text;
ALTER TABLE promotions ADD COLUMN price_label text;

CREATE TABLE display_channels (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL REFERENCES tenants(id),
    unit_id uuid NOT NULL,
    name text NOT NULL CHECK (length(btrim(name)) > 0),
    token_hash text NOT NULL,
    active boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (tenant_id, unit_id) REFERENCES units(tenant_id, id),
    UNIQUE (token_hash)
);
CREATE INDEX idx_display_channels_scope ON display_channels(tenant_id, unit_id) WHERE active;
