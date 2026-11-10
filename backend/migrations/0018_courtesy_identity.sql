-- Cortesia no nível da loja + identidade única entre nuvem e PDV.

-- Slug público da loja (usado no login quando o e-mail existe em mais de um tenant).
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS slug text;

WITH base AS (
    SELECT
        id,
        NULLIF(
            trim(both '-' FROM lower(regexp_replace(coalesce(nullif(trim(name), ''), 'loja'), '[^a-zA-Z0-9]+', '-', 'g'))),
            ''
        ) AS slug_base
    FROM tenants
), numbered AS (
    SELECT id, coalesce(slug_base, 'loja') AS slug_base, row_number() OVER (PARTITION BY coalesce(slug_base, 'loja') ORDER BY created_at, id) AS position
    FROM base
)
UPDATE tenants t
SET slug = CASE WHEN n.position = 1 THEN n.slug_base ELSE n.slug_base || '-' || n.position END
FROM numbered n
WHERE t.id = n.id
  AND (t.slug IS NULL OR t.slug = '');

CREATE UNIQUE INDEX IF NOT EXISTS tenants_slug_key ON tenants(slug);

-- Cortesia da assinatura (loja inteira: dono e todos os funcionários).
ALTER TABLE tenant_subscriptions ADD COLUMN IF NOT EXISTS billing_mode text NOT NULL DEFAULT 'paid';
ALTER TABLE tenant_subscriptions DROP CONSTRAINT IF EXISTS tenant_subscriptions_billing_mode_check;
ALTER TABLE tenant_subscriptions ADD CONSTRAINT tenant_subscriptions_billing_mode_check
    CHECK (billing_mode IN ('paid', 'courtesy'));
ALTER TABLE tenant_subscriptions ADD COLUMN IF NOT EXISTS courtesy_until timestamptz;
ALTER TABLE tenant_subscriptions ADD COLUMN IF NOT EXISTS courtesy_note text;

-- Identidade do usuário espelhada no PDV offline.
ALTER TABLE tenant_users ADD COLUMN IF NOT EXISTS display_name text;
ALTER TABLE tenant_users ADD COLUMN IF NOT EXISTS source text NOT NULL DEFAULT 'cloud';
ALTER TABLE tenant_users ADD COLUMN IF NOT EXISTS revoked_at timestamptz;
ALTER TABLE tenant_users ADD COLUMN IF NOT EXISTS updated_at timestamptz NOT NULL DEFAULT now();

CREATE INDEX IF NOT EXISTS tenant_users_email_active_idx ON tenant_users (email) WHERE active = true;
