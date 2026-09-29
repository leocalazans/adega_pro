CREATE TABLE tenant_branding (
    tenant_id uuid PRIMARY KEY REFERENCES tenants(id) ON DELETE CASCADE,
    display_name text NOT NULL,
    logo_data_url text,
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (char_length(display_name) BETWEEN 1 AND 80),
    CHECK (logo_data_url IS NULL OR char_length(logo_data_url) <= 2800000)
);
