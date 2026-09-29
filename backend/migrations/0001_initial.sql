CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE tenants (
    id uuid PRIMARY KEY,
    name text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE units (
    id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL REFERENCES tenants(id),
    name text NOT NULL,
    code text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, code),
    UNIQUE (tenant_id, id)
);

CREATE TABLE terminals (
    id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL REFERENCES tenants(id),
    unit_id uuid NOT NULL,
    name text NOT NULL,
    api_key_hash text NOT NULL,
    active boolean NOT NULL DEFAULT true,
    last_seen_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (tenant_id, unit_id) REFERENCES units(tenant_id, id),
    UNIQUE (tenant_id, id)
);

CREATE TABLE products (
    tenant_id uuid NOT NULL REFERENCES tenants(id),
    ean text NOT NULL,
    part_number text NOT NULL,
    description text NOT NULL,
    brand text,
    price_brl_cents bigint NOT NULL DEFAULT 0 CHECK (price_brl_cents >= 0),
    version bigint NOT NULL DEFAULT 1,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, ean),
    UNIQUE (tenant_id, part_number)
);

CREATE TABLE cloud_events (
    id bigserial PRIMARY KEY,
    tenant_id uuid NOT NULL REFERENCES tenants(id),
    unit_id uuid NOT NULL,
    terminal_id uuid NOT NULL,
    event_uuid uuid NOT NULL,
    entity text NOT NULL,
    operation text NOT NULL,
    payload jsonb NOT NULL,
    client_created_at bigint NOT NULL,
    received_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (tenant_id, unit_id) REFERENCES units(tenant_id, id),
    FOREIGN KEY (tenant_id, terminal_id) REFERENCES terminals(tenant_id, id),
    UNIQUE (tenant_id, event_uuid)
);

CREATE TABLE sales (
    tenant_id uuid NOT NULL REFERENCES tenants(id),
    unit_id uuid NOT NULL,
    terminal_id uuid NOT NULL,
    uuid uuid NOT NULL,
    total_brl_cents bigint NOT NULL CHECK (total_brl_cents >= 0),
    payment_method text NOT NULL,
    client_created_at bigint NOT NULL,
    received_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (tenant_id, unit_id) REFERENCES units(tenant_id, id),
    FOREIGN KEY (tenant_id, terminal_id) REFERENCES terminals(tenant_id, id),
    PRIMARY KEY (tenant_id, uuid)
);

CREATE TABLE sale_items (
    tenant_id uuid NOT NULL,
    sale_uuid uuid NOT NULL,
    line_no integer NOT NULL,
    ean text NOT NULL,
    qty double precision NOT NULL CHECK (qty > 0),
    price_brl_cents bigint NOT NULL CHECK (price_brl_cents >= 0),
    FOREIGN KEY (tenant_id, sale_uuid) REFERENCES sales(tenant_id, uuid) ON DELETE CASCADE,
    PRIMARY KEY (tenant_id, sale_uuid, line_no)
);

CREATE TABLE stock_movements (
    id bigserial PRIMARY KEY,
    tenant_id uuid NOT NULL REFERENCES tenants(id),
    unit_id uuid NOT NULL,
    event_uuid uuid NOT NULL,
    sale_uuid uuid,
    ean text NOT NULL,
    delta double precision NOT NULL,
    reason text NOT NULL,
    client_created_at bigint NOT NULL,
    received_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (tenant_id, unit_id) REFERENCES units(tenant_id, id),
    UNIQUE (tenant_id, event_uuid, ean)
);

CREATE INDEX idx_events_terminal_received ON cloud_events (tenant_id, terminal_id, received_at DESC);
CREATE INDEX idx_sales_unit_created ON sales (tenant_id, unit_id, client_created_at DESC);
CREATE INDEX idx_stock_unit_ean ON stock_movements (tenant_id, unit_id, ean);
CREATE INDEX idx_products_updated ON products (tenant_id, updated_at, ean);

