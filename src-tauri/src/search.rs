use serde::{Serialize, Deserialize};
use rusqlite::{params, Connection, Result};
use tauri::{command, Window, Manager};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Product {
    pub product_id: Option<i64>,
    pub name: String,
    pub barcode: Option<String>,
    pub price: f64,
    pub bulk_price: Option<f64>,
    pub bulk_single_conversion: Option<f64>,
    pub unit: Option<String>,
}

#[command]
pub fn search_products(window: Window, query: String) -> Result<Vec<Product>, String> {
    let db_path = tauri::api::path::app_data_dir(&window.app_handle().config())
        .map(|mut dir| { dir.push("inventory.db"); dir })
        .expect("Failed to get app data dir");
    println!("Searching database at: {:?}", db_path);

    let conn = Connection::open(&db_path)
        .map_err(|e| {
            println!("Database open error: {}", e);
            format!("Failed to open DB: {}", e)
        })?;

    let mut stmt = conn.prepare(
        "SELECT rowid, Item_name, Barcode, Retail_price, Bulk_price, Bulk_single_conversion, Unit FROM products WHERE Item_name LIKE ?1 OR Barcode LIKE ?1"
    )
    .map_err(|e| {
        println!("Prepare statement error: {}", e);
        format!("Failed to prepare statement: {}", e)
    })?;

    let products: Vec<_> = stmt.query_map(params![format!("%{}%", query)], |row| {
        let product = Product {
            product_id: row.get(0)?,
            name: row.get(1)?,
            barcode: row.get(2)?,
            price: row.get(3)?,
            bulk_price: row.get(4)?,
            bulk_single_conversion: row.get(5)?,
            unit: row.get(6)?,
        };
        println!("Found product: {:?}", product);
        Ok(product)
    })
    .map_err(|e| {
        println!("Query execution error: {}", e);
        format!("Failed to query: {}", e)
    })?
    .filter_map(|r| r.ok())
    .collect();

    println!("Search returned {} products", products.len());
    Ok(products)
}