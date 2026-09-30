CREATE TABLE platform_audit_events (
    id bigserial PRIMARY KEY,
    admin_id uuid REFERENCES platform_admins(id) ON DELETE SET NULL,
    action text NOT NULL,
    tenant_id uuid REFERENCES tenants(id) ON DELETE SET NULL,
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX platform_audit_events_tenant_created_idx ON platform_audit_events(tenant_id,created_at DESC);
