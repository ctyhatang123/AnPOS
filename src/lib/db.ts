import initSqlJs from 'sql.js';
import bcryptjs from 'bcryptjs';
import { writable, type Writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/tauri';

// Initialize SQL.js with WASM
let SQL: any;
let db: any;

// Database store
export const dbStore: Writable<any> = writable(null);

// Initialize database
export async function initializeDatabase(): Promise<any> {
    try {
        // Load SQL.js WASM
        SQL = await initSqlJs({
            locateFile: (file: string) => `/node_modules/sql.js/dist/${file}`
        });

        // Use the existing inventory.db file
        const dbPath = './inventory.db';
        
        // For now, we'll use an in-memory database and load data from the file
        // In a real Tauri app, we'd use Tauri's fs API to read the existing file
        db = new SQL.Database();
        
        // Enable WAL mode for better concurrency
        db.run('PRAGMA journal_mode=WAL;');
        
        // Create tables if they don't exist (matching your schema)
        createTables();
        
        // Insert sample data if tables are empty
        await insertSampleData();
        
        // Store database instance
        dbStore.set(db);
        
        console.log('Database initialized successfully');
        return db;
    } catch (error) {
        console.error('Failed to initialize database:', error);
        throw error;
    }
}

function createTables(): void {
    // Users table (matching your schema)
    db.run(`
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT UNIQUE NOT NULL,
            password TEXT NOT NULL,
            role TEXT NOT NULL CHECK (role IN ('admin', 'cashier')),
            last_login DATETIME,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
    `);

    // Products table (matching your schema)
    db.run(`
        CREATE TABLE IF NOT EXISTS products (
            Barcode VARCHAR,
            Item_name VARCHAR,
            Category VARCHAR,
            Unit VARCHAR,
            Bulk_unit VARCHAR,
            Bulk_code VARCHAR,
            Bulk_single_conversion FLOAT,
            Retail_price FLOAT,
            Bulk_price FLOAT,
            Cost FLOAT
        )
    `);

    // Sales table (matching your schema)
    db.run(`
        CREATE TABLE IF NOT EXISTS sales (
            id TEXT PRIMARY KEY,
            local_id TEXT UNIQUE NOT NULL,
            date DATETIME NOT NULL,
            operator TEXT NOT NULL,
            subtotal REAL NOT NULL,
            vat_rate REAL NOT NULL,
            vat_amount REAL NOT NULL,
            discount REAL NOT NULL,
            total REAL NOT NULL,
            payment_method TEXT NOT NULL CHECK (payment_method IN ('cash', 'qr')),
            status TEXT NOT NULL CHECK (status IN ('completed', 'pending')),
            synced BOOLEAN DEFAULT FALSE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
    `);

    // Sale items table (matching your schema)
    db.run(`
        CREATE TABLE IF NOT EXISTS sale_items (
            id TEXT PRIMARY KEY,
            sale_id TEXT NOT NULL,
            product_id TEXT NOT NULL,
            product_name TEXT NOT NULL,
            product_barcode TEXT NOT NULL,
            quantity INTEGER NOT NULL,
            price_type TEXT NOT NULL,
            unit_price REAL NOT NULL,
            discount REAL NOT NULL,
            total REAL NOT NULL
        )
    `);

    // Settings table (matching your schema)
    db.run(`
        CREATE TABLE IF NOT EXISTS settings (
            id TEXT PRIMARY KEY,
            shop_name TEXT NOT NULL,
            vat_rate REAL NOT NULL,
            default_printer TEXT NOT NULL,
            offline_mode BOOLEAN NOT NULL,
            sync_interval INTEGER NOT NULL,
            qr_expiry INTEGER NOT NULL,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
    `);

    // Cart table for temporary cart items (optional, can be removed if using RAM only)
    db.run(`
        CREATE TABLE IF NOT EXISTS cart (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            product_barcode TEXT NOT NULL,
            product_name TEXT NOT NULL,
            price_type TEXT NOT NULL CHECK (price_type IN ('single', 'bulk')),
            unit_price REAL NOT NULL,
            unit TEXT NOT NULL,
            quantity INTEGER NOT NULL DEFAULT 1,
            total REAL NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
    `);

    // Create indexes for better performance
    db.run('CREATE INDEX IF NOT EXISTS idx_products_barcode ON products(Barcode);');
    db.run('CREATE INDEX IF NOT EXISTS idx_products_name ON products(Item_name);');
    db.run('CREATE INDEX IF NOT EXISTS idx_products_category ON products(Category);');
    db.run('CREATE INDEX IF NOT EXISTS idx_sales_date ON sales(date);');
    db.run('CREATE INDEX IF NOT EXISTS idx_sale_items_sale_id ON sale_items(sale_id);');
}

async function insertSampleData(): Promise<void> {
    // Check if admin user already exists
    const userCount = db.exec('SELECT COUNT(*) as count FROM users')[0];
    if (userCount && userCount.values[0][0] > 0) {
        console.log('Users already exist, skipping user creation');
        return;
    }

    // Create admin user with password '1'
    const adminPassword = '1';
    const passwordHash = await bcryptjs.hash(adminPassword, 10);
    
    // Generate a unique ID for the admin user
    const adminId = 'admin_' + Date.now();
    
    db.run('INSERT INTO users (id, username, password, role) VALUES (?, ?, ?, ?)', 
           [adminId, 'admin', passwordHash, 'admin']);

    // Insert sample products if products table is empty
    const productCount = db.exec('SELECT COUNT(*) as count FROM products')[0];
    if (productCount && productCount.values[0][0] === 0) {
        const products = [
            ['1234567890123', 'Coca Cola', 'Beverages', 'Can', 'Pack', '1234567890124', 6.0, 2.50, 12.00, 1.80],
            ['1234567890125', 'Pepsi', 'Beverages', 'Can', 'Pack', '1234567890126', 6.0, 2.25, 11.00, 1.60],
            ['1234567890127', 'Chips', 'Snacks', 'Pack', 'Box', '1234567890128', 12.0, 1.75, 18.00, 1.20]
        ];

        const insertProduct = db.prepare('INSERT INTO products (Barcode, Item_name, Category, Unit, Bulk_unit, Bulk_code, Bulk_single_conversion, Retail_price, Bulk_price, Cost) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)');
        products.forEach(product => {
            insertProduct.run(product);
        });
        insertProduct.free();
    }

    // Insert default settings if settings table is empty
    const settingsCount = db.exec('SELECT COUNT(*) as count FROM settings')[0];
    if (settingsCount && settingsCount.values[0][0] === 0) {
        const settingsId = 'default_' + Date.now();
        db.run('INSERT INTO settings (id, shop_name, vat_rate, default_printer, offline_mode, sync_interval, qr_expiry) VALUES (?, ?, ?, ?, ?, ?, ?)', 
               [settingsId, 'AnPOS Shop', 0.10, 'Default Printer', true, 300, 300]);
    }

    console.log('Sample data inserted successfully');
}

// Login function to validate username and password
export async function loginUser(username: string, password: string): Promise<{ success: boolean; userId?: string; error?: string }> {
    try {
        // Query user by username
        const result = db.exec('SELECT id, username, password, role FROM users WHERE username = ?', [username]);
        
        if (result.length === 0 || result[0].values.length === 0) {
            return { success: false, error: 'Invalid username or password' };
        }
        
        const user = result[0].values[0];
        const userId = user[0] as string;
        const storedPasswordHash = user[2] as string;
        const role = user[3] as string;
        
        // Verify password using bcryptjs
        const isValidPassword = await bcryptjs.compare(password, storedPasswordHash);
        
        if (!isValidPassword) {
            return { success: false, error: 'Invalid username or password' };
        }
        
        // Update last login time
        db.run('UPDATE users SET last_login = CURRENT_TIMESTAMP WHERE id = ?', [userId]);
        
        return { success: true, userId };
    } catch (error) {
        console.error('Login error:', error);
        return { success: false, error: 'Login failed. Please try again.' };
    }
}

// Get database instance
export function getDatabase(): any {
    return db;
}

// Close database
export function closeDatabase(): void {
    if (db) {
        db.close();
        dbStore.set(null);
    }
}

// Helper: Remove Vietnamese diacritics for search
function removeDiacritics(str: string): string {
    return str.normalize('NFD').replace(/\p{Diacritic}/gu, '').toLowerCase();
}

// Search products function (robust, case-insensitive, diacritic-insensitive)
export function searchProducts(searchTerm: string): any[] {
    try {
        if (!searchTerm.trim()) {
            return [];
        }
        // Prepare search pattern
        const pattern = `%${searchTerm.trim()}%`;
        // Try direct LIKE search first
        let query = `
            SELECT 
                Barcode,
                Item_name,
                Category,
                Unit,
                Bulk_unit,
                Bulk_code,
                Bulk_single_conversion,
                Retail_price,
                Bulk_price,
                Cost
            FROM products 
            WHERE 
                Barcode LIKE ? OR 
                Item_name LIKE ?
            ORDER BY Item_name COLLATE NOCASE
            LIMIT 50
        `;
        let result = db.exec(query, [pattern, pattern]);
        if (result.length > 0 && result[0].values.length > 0) {
            const columns = result[0].columns;
            const values = result[0].values;
            return values.map((row: any) => {
                const product: any = {};
                columns.forEach((col: string, index: number) => {
                    product[col] = row[index];
                });
                return product;
            });
        }
        // If not found, try diacritic-insensitive search in JS
        // (fetch all, then filter in JS)
        query = `SELECT * FROM products LIMIT 500`;
        result = db.exec(query);
        if (result.length === 0) return [];
        const columns = result[0].columns;
        const values = result[0].values;
        const normSearch = removeDiacritics(searchTerm);
        return values
            .map((row: any) => {
                const product: any = {};
                columns.forEach((col: string, index: number) => {
                    product[col] = row[index];
                });
                return product;
            })
            .filter((product: any) =>
                removeDiacritics(product.Barcode || '').includes(normSearch) ||
                removeDiacritics(product.Item_name || '').includes(normSearch)
            );
    } catch (error) {
        console.error('Search products error:', error);
        return [];
    }
}

// Fetch VAT from settings table
export function getVATRate(): number {
    try {
        const result = db.exec('SELECT vat_rate FROM settings LIMIT 1');
        if (result.length > 0 && result[0].values.length > 0) {
            return Number(result[0].values[0][0]);
        }
        return 0.1; // default 10%
    } catch (error) {
        console.error('Get VAT error:', error);
        return 0.1;
    }
}

// Cart management functions using Tauri backend
export async function createCart(cartName: string) {
    const response = await invoke('create_cart', { cartName });
    return response; // Should return { cart_id, cart_name }
}

export async function addCartItem(cartId: number, productId: string, scannedBarcode: string | null, quantity: number, price: number, purchasingType: string, discount: number): Promise<any> {
    return await invoke('add_cart_item', { cart_id: cartId, product_id: productId, scanned_barcode: scannedBarcode, quantity, price, purchasing_type: purchasingType, discount });
}

export async function removeCartItem(cartId: number, productId: string, purchasingType: string): Promise<void> {
    return await invoke('remove_cart_item', { cart_id: cartId, product_id: productId, purchasing_type: purchasingType });
}

export async function updateCartItemQuantity(cartId: number, productId: string, purchasingType: string, quantity: number): Promise<void> {
    return await invoke('update_cart_item_quantity', { cart_id: cartId, product_id: productId, purchasing_type: purchasingType, quantity });
}

export async function parkCart(cartId: number, cartName: string): Promise<void> {
    return await invoke('park_cart', { cart_id: cartId, cart_name: cartName });
}

export async function activateCart(cartId: number): Promise<void> {
    return await invoke('activate_cart', { cart_id: cartId });
}

export async function checkoutCart(cartId: number, storeId: string, storemanId: string): Promise<string> {
    return await invoke('checkout_cart', { cart_id: cartId, store_id: storeId, storeman_id: storemanId });
}

export async function confirmPayment(cartId: number): Promise<void> {
    return await invoke('confirm_payment', { cart_id: cartId });
}

export async function cancelCart(cartId: number): Promise<void> {
    return await invoke('cancel_cart', { cart_id: cartId });
}

export async function listActiveCart(): Promise<any> {
    return await invoke('list_active_cart', {});
}

export async function listParkedCarts(): Promise<any[]> {
    return await invoke('list_parked_carts', {});
}

export async function listCartItems(cartId: number): Promise<any[]> {
    return await invoke('list_cart_items', { cart_id: cartId });
}

export async function cleanupExpiredCarts(ttlMinutes: number): Promise<void> {
    return await invoke('cleanup_expired_carts', { ttl_minutes: ttlMinutes });
}