-- Operacao do SuperAdmin: arquivamento de tenant, dados cadastrais,
-- historico de plano, gestao de unidades/terminais/usuarios, leads e indices.

-- Dados cadastrais que antes so existiam no Asaas.
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS cnpj_cpf text;
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS phone text;

-- Arquivamento: reversivel, nunca apaga dados de venda.
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS archived_at timestamptz;
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS archived_by uuid REFERENCES platform_admins(id) ON DELETE SET NULL;
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS archive_reason text;
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS archive_previous_status text;

CREATE INDEX IF NOT EXISTS tenants_archived_idx ON tenants(archived_at);

-- Unidade pode ser desativada sem perder historico de vendas.
ALTER TABLE units ADD COLUMN IF NOT EXISTS active boolean NOT NULL DEFAULT true;

-- Historico de upgrade/downgrade de plano.
CREATE TABLE IF NOT EXISTS tenant_plan_history (
    id bigserial PRIMARY KEY,
    tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    from_plan text,
    to_plan text NOT NULL,
    changed_by uuid REFERENCES platform_admins(id) ON DELETE SET NULL,
    note text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS tenant_plan_history_tenant_idx
    ON tenant_plan_history(tenant_id, created_at DESC);

-- Funil comercial dos leads.
ALTER TABLE commercial_leads ADD COLUMN IF NOT EXISTS status text NOT NULL DEFAULT 'new';
ALTER TABLE commercial_leads ADD COLUMN IF NOT EXISTS notes text;
ALTER TABLE commercial_leads DROP CONSTRAINT IF EXISTS commercial_leads_status_check;
ALTER TABLE commercial_leads ADD CONSTRAINT commercial_leads_status_check
    CHECK (status IN ('new', 'contacted', 'qualified', 'won', 'lost'));

CREATE INDEX IF NOT EXISTS platform_audit_events_created_idx
    ON platform_audit_events(created_at DESC);
CREATE INDEX IF NOT EXISTS payment_intents_tenant_created_idx
    ON payment_intents(tenant_id, created_at DESC);
