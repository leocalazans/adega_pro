ALTER TABLE tenant_branding ADD COLUMN IF NOT EXISTS primary_color text;

-- Cor padrão: verde floresta CommerceCTRL
UPDATE tenant_branding SET primary_color = COALESCE(primary_color, '#0D5C3A');
