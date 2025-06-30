-- Create carts table
CREATE TABLE IF NOT EXISTS carts (
    cart_id INTEGER PRIMARY KEY AUTOINCREMENT,
    cart_name VARCHAR(100) NOT NULL,
    status VARCHAR(20) NOT NULL,
    added_at DATETIME NOT NULL
);

-- Create cart_items table
CREATE TABLE IF NOT EXISTS cart_items (
    cart_id INTEGER NOT NULL,
    product_id VARCHAR(36) NOT NULL,
    scanned_barcode VARCHAR(255),
    quantity INTEGER NOT NULL,
    price FLOAT NOT NULL,
    purchasing_type VARCHAR(10) NOT NULL,
    discount FLOAT DEFAULT 0,
    FOREIGN KEY (cart_id) REFERENCES carts(cart_id)
);

-- Add product_id to products if not exists
ALTER TABLE products ADD COLUMN product_id VARCHAR(36);

-- Populate product_id for existing products (SQLite does not support UPDATE ... WHERE ... IS NULL in one step, so use a script)
UPDATE products SET product_id = (lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-' || lower(hex(randomblob(2))) || '-' || lower(hex(randomblob(2))) || '-' || lower(hex(randomblob(6)))) WHERE product_id IS NULL OR product_id = '';

-- Add unique constraint to product_id
CREATE UNIQUE INDEX IF NOT EXISTS idx_products_product_id ON products(product_id); 