CREATE TABLE tenant_subscriptions (
    tenant_id uuid PRIMARY KEY REFERENCES tenants(id) ON DELETE CASCADE,
    plan text NOT NULL CHECK(plan IN ('essencial','profissional','rede')),
    status text NOT NULL CHECK(status IN ('trial','active','past_due','suspended','cancelled')),
    trial_ends_at timestamptz,
    current_period_ends_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX tenant_subscriptions_status_idx ON tenant_subscriptions(status,current_period_ends_at);
