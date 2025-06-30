// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cart;
mod search;

use rusqlite::Connection;
use tauri::{Manager, api::path};

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            search::search_products,
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
        .setup(|app| {
            let app_handle = app.handle();
            let mut db_path = path::app_data_dir(&app_handle.config())
                .map(|mut dir| { dir.push("inventory.db"); dir })
                .expect("Failed to get app data dir");

            // Create directory if it doesn't exist
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent).expect("Failed to create app data directory");
            }

            // Copy inventory.db from bundle if it doesn't exist
            if !db_path.exists() {
                if let Some(resource_path) = app_handle.path_resolver().resolve_resource("inventory.db") {
                    std::fs::copy(&resource_path, &db_path).expect("Failed to copy database");
                } else {
                    return Err(Box::new(tauri::Error::Io(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "Bundled inventory.db not found",
                    ))));
                }
            }

            let conn = Connection::open(&db_path)
                .map_err(|e| format!("Failed to open database: {}", e))?;
            cart::initialize_tables(&conn)
                .map_err(|e| format!("Failed to initialize database tables: {}", e))?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}