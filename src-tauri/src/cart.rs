use serde::{Serialize, Deserialize};
use rusqlite::{params, Connection, Result};
use tauri::{command, Window, Manager}; // Ensure Manager is imported
use chrono::{Utc, Duration};
use tauri::api::path::app_data_dir;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Cart {
    pub cart_id: i64,
    pub cart_name: String,
    pub status: String,
    pub added_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CartItem {
    pub cart_id: i64,
    pub product_id: i64, // Changed to i64 for 5-digit IDs
    pub scanned_barcode: Option<String>,
    pub quantity: i32,
    pub price: f64,
    pub purchasing_type: String,
    pub discount: f64,
}

fn get_db_path(window: &Window) -> PathBuf {
    // Use the window to get the app handle and config
    let config = window.app_handle().config();
    app_data_dir(&config)
        .map(|mut dir| { dir.push("inventory.db"); dir })
        .unwrap_or_else(|| PathBuf::from("inventory.db"))
}

pub fn initialize_tables(conn: &Connection) -> Result<(), rusqlite::Error> {
    println!("Initializing tables..."); // Debug output
    std::thread::sleep(std::time::Duration::from_secs(1)); // Pause to observe
    conn.execute(
        "CREATE TABLE IF NOT EXISTS carts (
            cart_id INTEGER PRIMARY KEY AUTOINCREMENT,
            cart_name VARCHAR(100) NOT NULL DEFAULT '',
            status VARCHAR(20) NOT NULL DEFAULT 'active',
            added_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        params![],
    )?;
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
        params![],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS products (
            product_id INTEGER PRIMARY KEY AUTOINCREMENT,
            name VARCHAR(100) NOT NULL,
            barcode VARCHAR(255),
            price FLOAT NOT NULL
        )",
        params![],
    )?;
    println!("Tables initialized.");
    Ok(())
}

#[command]
pub fn create_cart(window: Window, cart_name: String) -> Result<Cart, String> {
    let db_path = get_db_path(&window);
    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    let now = Utc::now().naive_utc();
    let added_at = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let sql = "INSERT INTO carts (cart_name, status, added_at) VALUES (?1, ?2, ?3)";
    conn.execute(sql, params![cart_name, "active", added_at])
        .map_err(|e| format!("Failed to create cart: {}", e))?;
    let cart_id = conn.last_insert_rowid();
    let cart = Cart {
        cart_id,
        cart_name,
        status: "active".to_string(),
        added_at,
    };
    Ok(cart)
}

#[command]
pub fn update_cart_name(window: Window, cart_id: i64, cart_name: String) -> Result<(), String> {
    let db_path = get_db_path(&window);
    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    let sql = "UPDATE carts SET cart_name = ?1 WHERE cart_id = ?2";
    conn.execute(sql, params![cart_name, cart_id])
        .map_err(|e| format!("Failed to update cart name: {}", e))?;
    Ok(())
}

#[command]
pub fn add_cart_item(window: Window, cart_id: i64, product_id: i64, scanned_barcode: Option<String>, quantity: i32, price: f64, purchasing_type: String, discount: f64) -> Result<CartItem, String> {
    let db_path = get_db_path(&window);
    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    let mut stmt = conn.prepare("SELECT status FROM carts WHERE cart_id = ?1")
        .map_err(|e| e.to_string())?; // Convert rusqlite::Error to String
    let status: String = stmt.query_row(params![cart_id], |row| row.get(0))
        .map_err(|e| e.to_string())?; // Convert rusqlite::Error to String
    if status != "active" {
        return Err("Cart is not active".to_string());
    }
    let sql = "INSERT INTO cart_items (cart_id, product_id, scanned_barcode, quantity, price, purchasing_type, discount) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";
    conn.execute(sql, params![cart_id, product_id, scanned_barcode, quantity, price, purchasing_type, discount])
        .map_err(|e| format!("Failed to add cart item: {}", e))?;
    let item = CartItem {
        cart_id,
        product_id,
        scanned_barcode,
        quantity,
        price,
        purchasing_type,
        discount,
    };
    Ok(item)
}

#[command]
pub fn remove_cart_item(window: Window, cart_id: i64, product_id: i64, purchasing_type: String) -> Result<(), String> {
    let db_path = get_db_path(&window);
    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    let sql = "DELETE FROM cart_items WHERE cart_id = ?1 AND product_id = ?2 AND purchasing_type = ?3";
    conn.execute(sql, params![cart_id, product_id, purchasing_type])
        .map_err(|e| format!("Failed to remove cart item: {}", e))?;
    Ok(())
}

#[command]
pub fn update_cart_item_quantity(window: Window, cart_id: i64, product_id: i64, purchasing_type: String, quantity: i32) -> Result<(), String> {
    let db_path = get_db_path(&window);
    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    let sql = "UPDATE cart_items SET quantity = ?1 WHERE cart_id = ?2 AND product_id = ?3 AND purchasing_type = ?4";
    conn.execute(sql, params![quantity, cart_id, product_id, purchasing_type])
        .map_err(|e| format!("Failed to update quantity: {}", e))?;
    if quantity == 0 {
        remove_cart_item(window, cart_id, product_id, purchasing_type)?;
    }
    Ok(())
}

#[command]
pub fn park_cart(window: Window, cart_id: i64, cart_name: String) -> Result<(), String> {
    let db_path = get_db_path(&window);
    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    let sql = "UPDATE carts SET status = 'parked', cart_name = ?1 WHERE cart_id = ?2 AND status = 'active'";
    let rows = conn.execute(sql, params![cart_name, cart_id])
        .map_err(|e| format!("Failed to park cart: {}", e))?;
    if rows == 0 {
        Err("No active cart found with that ID".to_string())
    } else {
        Ok(())
    }
}

#[command]
pub fn activate_cart(window: Window, cart_id: i64) -> Result<(), String> {
    let db_path = get_db_path(&window);
    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    conn.execute("UPDATE carts SET status = 'parked' WHERE status = 'active'", params![])
        .map_err(|e| format!("Failed to park other carts: {}", e))?;
    let sql = "UPDATE carts SET status = 'active' WHERE cart_id = ?1 AND status = 'parked'";
    let rows = conn.execute(sql, params![cart_id])
        .map_err(|e| format!("Failed to activate cart: {}", e))?;
    if rows == 0 {
        Err("No parked cart found with that ID".to_string())
    } else {
        Ok(())
    }
}

#[command]
pub fn checkout_cart(window: Window, cart_id: i64, store_id: String, storeman_id: String) -> Result<String, String> {
    let db_path = get_db_path(&window);
    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    conn.execute("UPDATE carts SET status = 'pending checkout' WHERE cart_id = ?1", params![cart_id])
        .map_err(|e| format!("Failed to set pending checkout: {}", e))?;
    let now = Utc::now();
    let date_str = now.format("%Y%m%d").to_string();
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM carts WHERE added_at LIKE ?1 AND status = 'pending checkout'")
        .map_err(|e| format!("Failed to prepare count: {}", e))?;
    let like_pattern = format!("{}%", date_str);
    let count: i64 = stmt.query_row(params![like_pattern], |row| row.get(0))
        .map_err(|e| format!("Failed to count carts: {}", e))?;
    let sequence = format!("{:03}", count + 1);
    let _timestamp = now.format("%H%M").to_string(); // Prefixed with _ to silence warning
    let invoice_id = format!("{}_{}_{}_{}", store_id, storeman_id, date_str, sequence);
    Ok(invoice_id)
}

#[command]
pub fn confirm_payment(window: Window, cart_id: i64) -> Result<(), String> {
    let db_path = get_db_path(&window);
    let mut conn = Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    conn.execute("UPDATE carts SET status = 'processed' WHERE cart_id = ?1", params![cart_id])
        .map_err(|e| format!("Failed to set processed: {}", e))?;
    let tx = conn.transaction().map_err(|e| format!("Failed to start transaction: {}", e))?;
    tx.execute("DELETE FROM cart_items WHERE cart_id = ?1", params![cart_id])
        .map_err(|e| format!("Failed to delete cart items: {}", e))?;
    tx.execute("DELETE FROM carts WHERE cart_id = ?1", params![cart_id])
        .map_err(|e| format!("Failed to delete cart: {}", e))?;
    tx.commit().map_err(|e| format!("Failed to commit transaction: {}", e))?;
    Ok(())
}

#[command]
pub fn cancel_cart(window: Window, cart_id: i64) -> Result<(), String> {
    let db_path = get_db_path(&window);
    let mut conn = Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    let tx = conn.transaction().map_err(|e| format!("Failed to start transaction: {}", e))?;
    tx.execute("DELETE FROM cart_items WHERE cart_id = ?1", params![cart_id])
        .map_err(|e| format!("Failed to delete cart items: {}", e))?;
    tx.execute("DELETE FROM carts WHERE cart_id = ?1", params![cart_id])
        .map_err(|e| format!("Failed to delete cart: {}", e))?;
    tx.commit().map_err(|e| format!("Failed to commit transaction: {}", e))?;
    Ok(())
}

#[command]
pub fn list_active_cart(window: Window) -> Result<Option<Cart>, String> {
    let db_path = get_db_path(&window);
    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    let mut stmt = conn.prepare("SELECT cart_id, cart_name, status, added_at FROM carts WHERE status = 'active' LIMIT 1")
        .map_err(|e| format!("Failed to prepare: {}", e))?;
    let cart = stmt.query_map(params![], |row| {
        Ok(Cart {
            cart_id: row.get(0)?,
            cart_name: row.get(1)?,
            status: row.get(2)?,
            added_at: row.get(3)?,
        })
    })
    .map_err(|e| format!("Failed to query: {}", e))?
    .filter_map(|r| r.ok())
    .next();
    Ok(cart)
}

#[command]
pub fn list_parked_carts(window: Window) -> Result<Vec<Cart>, String> {
    let db_path = get_db_path(&window);
    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    let mut stmt = conn.prepare("SELECT cart_id, cart_name, status, added_at FROM carts WHERE status = 'parked'")
        .map_err(|e| format!("Failed to prepare: {}", e))?;
    let carts = stmt.query_map(params![], |row| {
        Ok(Cart {
            cart_id: row.get(0)?,
            cart_name: row.get(1)?,
            status: row.get(2)?,
            added_at: row.get(3)?,
        })
    })
    .map_err(|e| format!("Failed to query: {}", e))?
    .filter_map(|r| r.ok())
    .collect();
    Ok(carts)
}

#[command]
pub fn list_cart_items(window: Window, cart_id: i64) -> Result<Vec<CartItem>, String> {
    let db_path = get_db_path(&window);
    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    let mut stmt = conn.prepare("SELECT cart_id, product_id, scanned_barcode, quantity, price, purchasing_type, discount FROM cart_items WHERE cart_id = ?1")
        .map_err(|e| format!("Failed to prepare: {}", e))?;
    let items = stmt.query_map(params![cart_id], |row| {
        Ok(CartItem {
            cart_id: row.get(0)?,
            product_id: row.get(1)?,
            scanned_barcode: row.get(2).ok(),
            quantity: row.get(3)?,
            price: row.get(4)?,
            purchasing_type: row.get(5)?,
            discount: row.get(6)?,
        })
    })
    .map_err(|e| format!("Failed to query: {}", e))?
    .filter_map(|r| r.ok())
    .collect();
    Ok(items)
}

#[command]
pub fn cleanup_expired_carts(window: Window, ttl_minutes: i64) -> Result<(), String> {
    let db_path = get_db_path(&window);
    let mut conn = Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    let now = Utc::now().naive_utc();
    let cutoff = now - Duration::minutes(ttl_minutes);
    let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();
    let tx = conn.transaction().map_err(|e| format!("Failed to start transaction: {}", e))?;
    tx.execute("DELETE FROM carts WHERE status = 'active' AND added_at < ?1", params![cutoff_str])
        .map_err(|e| format!("Failed to cleanup expired carts: {}", e))?;
    tx.commit().map_err(|e| format!("Failed to commit transaction: {}", e))?;
    Ok(())
}