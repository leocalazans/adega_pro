CREATE TABLE IF NOT EXISTS login_attempts (
    id bigserial PRIMARY KEY,
    tenant_id uuid REFERENCES tenants(id) ON DELETE CASCADE,
    email text NOT NULL,
    ip text NOT NULL,
    succeeded boolean NOT NULL DEFAULT false,
    attempted_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS login_attempts_email_idx ON login_attempts(email, attempted_at);
CREATE INDEX IF NOT EXISTS login_attempts_ip_idx ON login_attempts(ip, attempted_at);

-- Limpeza automática de tentativas antigas pode ser feita por job ou manualmente.
-- Exemplo: DELETE FROM login_attempts WHERE attempted_at < now() - interval '30 days';
