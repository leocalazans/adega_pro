CREATE TABLE IF NOT EXISTS platform_plan_catalog (
    code text PRIMARY KEY CHECK (code IN ('starter','fiscal','premium')),
    name text NOT NULL,
    monthly_brl_cents integer NOT NULL CHECK (monthly_brl_cents >= 0),
    description text NOT NULL,
    fiscal_eligible boolean NOT NULL DEFAULT false,
    premium_pdv boolean NOT NULL DEFAULT false,
    active boolean NOT NULL DEFAULT true,
    updated_at timestamptz NOT NULL DEFAULT now()
);
ALTER TABLE tenant_subscriptions DROP CONSTRAINT IF EXISTS tenant_subscriptions_plan_check;
ALTER TABLE tenant_subscriptions ADD CONSTRAINT tenant_subscriptions_plan_check CHECK(plan IN ('essencial','profissional','rede','starter','fiscal','premium'));
INSERT INTO platform_plan_catalog(code,name,monthly_brl_cents,description,fiscal_eligible,premium_pdv) VALUES
('starter','Comecar',10000,'PDV, estoque, relatorios basicos e comprovante nao fiscal.',false,false),
('fiscal','Fiscal',18000,'Recursos fiscais liberados somente apos homologacao.',true,false),
('premium','PDV Premium',25000,'Operacao multiunidade e recursos premium liberados.',true,true)
ON CONFLICT (code) DO UPDATE SET name=excluded.name, monthly_brl_cents=excluded.monthly_brl_cents, description=excluded.description, fiscal_eligible=excluded.fiscal_eligible, premium_pdv=excluded.premium_pdv, updated_at=now();

CREATE TABLE IF NOT EXISTS platform_feature_flags (
    key text PRIMARY KEY,
    name text NOT NULL,
    description text NOT NULL,
    requires_homologation boolean NOT NULL DEFAULT false,
    active boolean NOT NULL DEFAULT true
);
INSERT INTO platform_feature_flags(key,name,description,requires_homologation) VALUES
('fiscal','Emissao fiscal','Emissao fiscal por UF e credencial.',true),
('delivery_ifood','iFood','Pedidos e catalogo via integracao iFood.',true),
('delivery_99food','99Food','Pedidos e catalogo via integracao 99Food.',true),
('delivery_ze','Ze Delivery','Pedidos e catalogo via integracao Ze Delivery.',true),
('balanca_prix','Balanca Prix','Leitura por rede apos validacao do protocolo.',true),
('android_companion','Aplicativo Android','Companheiro Android apos publicacao assinada.',true),
('multiunidade','Multiunidade','Gestao de multiplas unidades.',false),
('tv_ofertas','TV de ofertas','Canal de ofertas por unidade.',false)
ON CONFLICT (key) DO UPDATE SET name=excluded.name, description=excluded.description, requires_homologation=excluded.requires_homologation, active=excluded.active;

CREATE TABLE IF NOT EXISTS tenant_feature_flags (
    tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    key text NOT NULL REFERENCES platform_feature_flags(key),
    enabled boolean NOT NULL DEFAULT false,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY(tenant_id,key)
);
CREATE TABLE IF NOT EXISTS platform_equipment_catalog (
    id uuid PRIMARY KEY,
    sku text NOT NULL UNIQUE,
    name text NOT NULL,
    category text NOT NULL,
    sale_brl_cents integer CHECK (sale_brl_cents IS NULL OR sale_brl_cents >= 0),
    rental_brl_cents integer CHECK (rental_brl_cents IS NULL OR rental_brl_cents >= 0),
    source_url text,
    active boolean NOT NULL DEFAULT true,
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS commercial_leads (
    id uuid PRIMARY KEY,
    store_name text NOT NULL,
    contact_name text,
    email text,
    phone text,
    city text,
    plan_code text REFERENCES platform_plan_catalog(code),
    modality text NOT NULL DEFAULT 'purchase' CHECK(modality IN ('purchase','loan')),
    details text,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS tenant_integration_configs (
    tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    provider text NOT NULL CHECK(provider IN ('ifood','99food','ze_delivery','fiscal')),
    enabled boolean NOT NULL DEFAULT false,
    status text NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','pending_homologation','active','disabled')),
    public_config jsonb NOT NULL DEFAULT '{}'::jsonb,
    secret_encrypted bytea,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY(tenant_id,provider)
);
-- migration 0013 reescrita em UTF-8 valido e idempotente para sqlx-migrate
