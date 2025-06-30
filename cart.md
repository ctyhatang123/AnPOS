# Task: Create SQLite tables and implement Create Order page logic for AnPOS

# Description:
# Set up a multi-row cart system for an offline SQLite POS app (AnPOS). Create 'carts' and 'cart_items' tables, add a 'product_id' UUID column to the existing 'products' table, and implement functions for the Create Order page based on storeman workflow.

# Context:
# - Offline SQLite, single-device POS app.
# - Storeman scans barcodes or searches products in the Create Order page, triggering cart creation.
# - UI has: Search bar (barcode/scanner or manual search), Search Result table (add items), Active Cart area, new Park Cart area below it.
# - 'products' table has 10k rows, needs 'product_id' added and populated with UUIDs.

# Requirements:
# 1. Create tables:
#    - 'carts': 
#      - cart_id: INTEGER PRIMARY KEY AUTOINCREMENT
#      - cart_name: VARCHAR(100) NOT NULL
#      - status: VARCHAR(20) NOT NULL (e.g., 'active', 'parked', 'pending checkout', 'processed')
#      - added_at: DATETIME NOT NULL
#    - 'cart_items': 
#      - cart_id: INTEGER
#      - product_id: VARCHAR(36) NOT NULL (UUID)
#      - scanned_barcode: VARCHAR(255)
#      - quantity: INTEGER NOT NULL
#      - price: FLOAT NOT NULL
#      - purchasing_type: VARCHAR(10) NOT NULL (e.g., 'single' or 'bulk')
#      - discount: FLOAT DEFAULT 0
#      - FOREIGN KEY (cart_id) REFERENCES carts(cart_id)
#    - Update 'products': 
#      - Add 'product_id': VARCHAR(36) UNIQUE
#      - Script to generate UUIDs for 10k existing rows, auto-generate for new rows
# 2. Implement Create Order page logic (functions):
#    - Generate cart_id when first item is added (scan/search), set status to 'active', prompt for cart_name.
#    - Delete cart_id when: 'Confirm Payment' (after checkout), 'Cancel Order', or last item quantity reduced to 0.
#    - Set status: 'active' on creation, 'parked' on 'Park Cart', 'pending checkout' on 'Checkout', 'processed' on payment confirmation (then delete).
#    - Track active cart in Active Cart area, show parked carts in Park Cart area, allow 'Activate' button to move parked cart to active.
#    - Add items to active cart_id, enforce TTL (10 min default, max 30 min, adjustable in Settings) to delete 'active' carts.
#    - Generate invoice_id at 'Checkout' (format: store_id + storeman_id + timestamp + sequence, e.g., 'STORE1_SMAN1_20250626_1652_001'), allow adjustments pre-payment, delete cart after confirmation.
# 3. Notes:
#    - No stock tracking, customer IDs, or multi-sessions.
#    - Handle surge with parked carts, ensure one active cart at a time.

# Action:
# - Generate SQL CREATE TABLE statements for 'carts' and 'cart_items'
# - Provide SQL ALTER TABLE and UPDATE script for 'products' with UUIDs
# - Implement JavaScript/TypeScript functions for Create Order page logic
# - Include comments explaining each step and UI integration

# Output:
# - SQL commands for table creation and 'products' update
# - JavaScript/TypeScript functions for cart management
# - UI notes for Active Cart and Park Cart areas