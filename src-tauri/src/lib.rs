use tauri::Manager;
use serde::{Serialize, Deserialize};
use rusqlite::{Connection};
use std::path::PathBuf;
use std::env;
use tauri::api::path::app_data_dir;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;
mod cart;

pub fn initialize_tables(conn: &Connection) -> Result<(), rusqlite::Error> {
    // Products table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS products (
            product_id INTEGER PRIMARY KEY AUTOINCREMENT,
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
        )",
        [],
    )?;

    // Carts table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS carts (
            cart_id INTEGER PRIMARY KEY AUTOINCREMENT,
            cart_name VARCHAR(100) NOT NULL DEFAULT '',
            status VARCHAR(20) NOT NULL DEFAULT 'active',
            added_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Cart items table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS cart_items (
            cart_id INTEGER,
            product_id INTEGER NOT NULL,
            scanned_barcode VARCHAR(255),
            quantity INTEGER NOT NULL,
            price FLOAT NOT NULL,
            purchasing_type VARCHAR(10) NOT NULL,
            discount FLOAT DEFAULT 0,
            FOREIGN KEY (cart_id) REFERENCES carts(cart_id)
        )",
        [],
    )?;

    // You can add other tables like users, settings here as well
    // For example:
    // conn.execute("...", [])?;

    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct Product {
    pub barcode: String,
    pub item_name: String,
    pub category: String,
    pub unit: String,
    pub bulk_unit: Option<String>,
    pub bulk_code: Option<String>,
    pub bulk_single_conversion: Option<f64>,
    pub retail_price: Option<f64>,
    pub bulk_price: Option<f64>,
    pub cost: Option<f64>,
}

#[derive(Deserialize)]
pub struct CartAddRequest {
    pub user_id: String,
    pub product: Product,
    pub price_type: String, // "single" or "bulk"
    pub quantity: i64,
}

fn remove_accents(s: &str) -> String {
    s.nfd()
        
    .filter(|c| !is_combining_mark(*c))
        .collect::<String>()
}

#[tauri::command]
fn search_products(query: String, window: tauri::Window) -> Result<Vec<Product>, String> {
    let db_path = app_data_dir(&window.config())
        .map(|mut dir| { dir.push("inventory.db"); dir })
        .unwrap_or_else(|| PathBuf::from("inventory.db"));
    println!("[AnPOS DEBUG] Using DB path: {}", db_path.display());
    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;

    // Register a custom SQL function for accent-insensitive search
    conn.create_scalar_function(
        "remove_accents",
        1,
        rusqlite::functions::FunctionFlags::SQLITE_UTF8,
        |ctx| {
            let input: String = ctx.get(0)?;
            Ok(remove_accents(&input))
        },
    ).map_err(|e| format!("Failed to register accent function: {}", e))?;

    let pattern = format!("%{}%", remove_accents(&query.to_lowercase()));
    let mut products = Vec::new();
    let (sql, params): (String, Vec<&dyn rusqlite::ToSql>) = if query.trim().is_empty() {
        ("SELECT Barcode, Item_name, Category, Unit, Bulk_unit, Bulk_code, Bulk_single_conversion, Retail_price, Bulk_price, Cost FROM products LIMIT 50".to_string(), vec![])
    } else {
        ("SELECT Barcode, Item_name, Category, Unit, Bulk_unit, Bulk_code, Bulk_single_conversion, Retail_price, Bulk_price, Cost FROM products WHERE remove_accents(LOWER(Barcode)) LIKE ?1 OR remove_accents(LOWER(Item_name)) LIKE ?1 LIMIT 50".to_string(), vec![&pattern])
    };
    let mut stmt = conn.prepare(&sql).map_err(|e| format!("Prepare error: {}", e))?;
    let rows = stmt.query_map(params.as_slice(), |row| {
        Ok(Product {
            barcode: row.get(0)?,
            item_name: row.get(1)?,
            category: row.get(2)?,
            unit: row.get(3)?,
            bulk_unit: row.get(4).ok(),
            bulk_code: row.get(5).ok(),
            bulk_single_conversion: row.get(6).ok(),
            retail_price: row.get(7).ok(),
            bulk_price: row.get(8).ok(),
            cost: row.get(9).ok(),
        })
    });
    match rows {
        Ok(iter) => {
            for prod in iter {
                match prod {
                    Ok(p) => products.push(p),
                    Err(e) => println!("[search_products] Row error: {}", e),
                }
            }
            Ok(products)
        },
        Err(e) => {
            let msg = format!("Query error: {}", e);
            println!("[search_products] {}", msg);
            Err(msg)
        }
    }
}

#[tauri::command]
fn add_to_cart(request: CartAddRequest, window: tauri::Window) -> Result<(), String> {
    let db_path = app_data_dir(&window.config())
        .map(|mut dir| { dir.push("inventory.db"); dir })
        .unwrap_or_else(|| PathBuf::from("inventory.db"));
    let mut conn = Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    let tx = conn.transaction().map_err(|e| format!("Failed to start transaction: {}", e))?;
    let (unit_price, unit, total) = if request.price_type == "single" {
        let price = request.product.retail_price.unwrap_or(0.0);
        let unit = request.product.unit.clone();
        let total = price * request.quantity as f64;
        (price, unit, total)
    } else {
        let price = request.product.bulk_price.unwrap_or(0.0);
        let unit = request.product.bulk_unit.clone().unwrap_or_else(|| "bulk".to_string());
        let total = price * request.quantity as f64;
        (price, unit, total)
    };
    let id = format!("cart_{}_{}_{}", request.user_id, request.product.barcode, request.price_type);
    tx.execute(
        "INSERT INTO cart (id, user_id, product_barcode, product_name, price_type, unit_price, unit, quantity, total) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            id,
            request.user_id,
            request.product.barcode,
            request.product.item_name,
            request.price_type,
            unit_price,
            unit,
            request.quantity,
            total
        ]
    ).map_err(|e| format!("Failed to insert into cart: {}", e))?;
    tx.commit().map_err(|e| format!("Failed to commit transaction: {}", e))?;
    Ok(())
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            greet,
            search_products,
            add_to_cart,
            // Cart commands
            cart::create_cart,
            cart::add_cart_item,
            cart::remove_cart_item,
            cart::update_cart_item_quantity,
            cart::park_cart,
            cart::activate_cart,
            cart::checkout_cart,
            cart::confirm_payment,
            cart::cancel_cart,
            cart::list_active_cart,
            cart::list_parked_carts,
            cart::list_cart_items,
            cart::cleanup_expired_carts,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
