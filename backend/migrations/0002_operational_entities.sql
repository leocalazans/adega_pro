ALTER TABLE products ADD COLUMN IF NOT EXISTS min_stock double precision NOT NULL DEFAULT 10;

CREATE TABLE operational_records (
    tenant_id uuid NOT NULL REFERENCES tenants(id),
    unit_id uuid NOT NULL,
    terminal_id uuid NOT NULL,
    entity text NOT NULL,
    local_id bigint NOT NULL,
    payload jsonb NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (tenant_id, unit_id) REFERENCES units(tenant_id, id),
    FOREIGN KEY (tenant_id, terminal_id) REFERENCES terminals(tenant_id, id),
    PRIMARY KEY (tenant_id, terminal_id, entity, local_id)
);

CREATE INDEX idx_operational_unit_entity ON operational_records(tenant_id, unit_id, entity, updated_at DESC);
