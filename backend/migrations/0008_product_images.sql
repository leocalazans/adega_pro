ALTER TABLE products ADD COLUMN IF NOT EXISTS image_data_url text;
ALTER TABLE products ADD CONSTRAINT products_image_size CHECK (image_data_url IS NULL OR length(image_data_url) <= 700000);
