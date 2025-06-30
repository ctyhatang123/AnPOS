AnPOS Task Table



ID
Description
Status
Priority
Assigned To
Notes



1
Create a src/lib/db.js file to initialize a SQLite database using sql.js. Define tables: users(id, username, password_hash, last_login), products(id, name, barcode, price), cart(id, user_id, product_id, quantity), and sales(id, user_id, total, created_at). Store the DB in %APPDATA%/anpos/anpos.db. Enable WAL mode. Insert sample data: 1 user (admin, hashed password), 3 products (Coca Cola, Pepsi, Chips).
DONE
High
Cursor
Bundle sql.js WASM locally. Use bcryptjs for the admin password hash.


2
Create src/lib/store.js with Svelte writable stores for user (stores user ID and username) and db (stores SQLite instance). Initialize the DB at app startup and load the last username from localStorage.
DONE
High
Cursor
Ensure offline functionality. Use localStorage for faster username retrieval.


3
Set up Tailwind CSS in the SvelteKit project. Update tailwind.config.js to include Svelte files, create src/app.css with Tailwind directives, and import it in src/routes/+layout.svelte.
DONE
High
Cursor
Keep styling minimal and responsive. Add purple gradient (#7856D1 to #401E98) if possible.


4
Update src-tauri/tauri.conf.json to include Tauri's single-instance plugin to prevent multiple app instances from accessing the SQLite DB.
TODO
Medium
Cursor
Add dependency @tauri-apps/plugin-single-instance to package.json.


5
Implement src/routes/+page.svelte with a full-screen, centered form (white card, gray background, blue button). Include a disabled username input (pre-filled from localStorage, default 'admin'), a password input (auto-focused), and a Login button. Style with Tailwind CSS.
DONE
High
Cursor
Ensure Enter key submits the form. Keep UI minimal and responsive.


6
In src/lib/db.js, add a login function to validate username and password against the users table using bcryptjs. On success, store user ID and username in the Svelte user store, save username to localStorage, and redirect to /create-order. Show an error message for invalid credentials.
DONE
High
Cursor
Use sample admin user from task #1. Wrap DB operations in a transaction.


7
In src/routes/+page.svelte, add a "Change User" link below the username input. When clicked, enable the username input for editing and clear localStorage username.
TODO
Medium
Cursor
Ensure the link is subtle (e.g., smaller font, underline) but accessible.


8
Add a logout button (top-right) in src/routes/+page.svelte and src/routes/create-order/+page.svelte. On click, clear the Svelte user store and redirect to /. Update src/lib/store.js to handle logout logic.
DONE
Medium
Cursor
Ensure logout clears session data but keeps localStorage username.


9
Implement a 15-minute session timeout in src/lib/store.js. Reset the timer on user activity (clicks, keypresses). On timeout, trigger logout (clear user store, redirect to /).
DONE
Medium
Cursor
Use setTimeout and listen for click/keydown events to reset the timer.


10
Implement src/routes/create-order/+page.svelte with a 70/30 split layout (70% left panel, 30% right panel for cart). Left panel: search bar (auto-focused) and data table (columns: Name, Barcode, Price, Actions). Right panel: cart list and Checkout button. Style with Tailwind CSS.
DONE
High
Cursor
Protect the page with a check for logged-in user (redirect to / if not logged in). Use flexbox for the split layout.


11
In src/lib/db.js, add a searchProducts function to query the products table for name or barcode matches. Debounce the search by 300ms in src/routes/create-order/+page.svelte. Limit results to 50 rows. Create a SQLite index on products(barcode, name).
DONE
High
Cursor
Use sql.js for queries. Update table dynamically as users type or scan.


12
In src/routes/create-order/+page.svelte, add "Add Single" (adds 1 unit) and "Add Bulk" (prompts for quantity) buttons to each table row. Call addToCart function in src/lib/db.js to insert into the cart table.
DONE
High
Cursor
Use a simple prompt for bulk quantity. Wrap DB inserts in a transaction.


13
In src/routes/create-order/+page.svelte, display cart items (name, quantity, price) in the right panel. Fetch data from the cart table using a getCart function in src/lib/db.js. Update the cart reactively using a Svelte store.
DONE
High
Cursor
Show total price at the bottom of the cart.


14
In src/routes/create-order/+page.svelte, add a "Cancel Order" button in the cart panel to clear the cart table for the current user. Call a clearCart function in src/lib/db.js.
DONE
Medium
Cursor
Wrap DB delete in a transaction. Show a confirmation dialog before clearing.


15
In src/routes/create-order/+page.svelte, add a Checkout button in the cart panel. On click, save the cart to the sales table (calculate total), clear the cart table, and show a "Checkout complete" alert. Add a checkout function in src/lib/db.js.
TODO
High
Cursor
Use a SQLite transaction to ensure atomicity. Prepare for future VietQR integration.


Status Legend

TODO: Not started
IN_PROGRESS: Being worked on
DONE: Completed
BLOCKED: Waiting on something (e.g., dependency, clarification)
