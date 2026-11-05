CREATE TABLE IF NOT EXISTS tenant_users (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    unit_id uuid REFERENCES units(id) ON DELETE SET NULL,
    email text NOT NULL,
    password_hash text NOT NULL,
    role text NOT NULL CHECK (role = ANY (ARRAY['owner'::text, 'manager'::text, 'cashier'::text, 'stock'::text, 'finance'::text, 'viewer'::text])),
    active boolean NOT NULL DEFAULT true,
    must_change_password boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    last_login_at timestamptz,
    CONSTRAINT tenant_users_tenant_id_email_key UNIQUE (tenant_id, email)
);

CREATE INDEX IF NOT EXISTS tenant_users_tenant_id_idx ON tenant_users(tenant_id);
CREATE INDEX IF NOT EXISTS tenant_users_email_idx ON tenant_users(email);

CREATE TABLE IF NOT EXISTS tenant_sessions (
    token_hash text PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES tenant_users(id) ON DELETE CASCADE,
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    last_seen_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS tenant_sessions_expiry_idx ON tenant_sessions(expires_at);
CREATE INDEX IF NOT EXISTS tenant_sessions_user_id_idx ON tenant_sessions(user_id);
