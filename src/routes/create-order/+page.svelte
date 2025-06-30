<script lang="ts">
  import { onMount } from 'svelte';
  import { user, logout } from '$lib/store';
  import { goto } from '$app/navigation';
  import {
    parkCart,
    activateCart,
    checkoutCart,
    confirmPayment,
    listParkedCarts,
    listCartItems,
    cleanupExpiredCarts,
    getVATRate
  } from '$lib/db';
  import { invoke } from '@tauri-apps/api/tauri';

  let searchQuery = '';
  let products: { product_id: number, name: string, barcode: string | null, price: number, bulk_price?: number, bulk_single_conversion?: number, unit?: string }[] = [];
  let tempCart: { product_id: number, product_name: string, product_barcode: string | null, quantity: number, unit_price: number, purchasing_type: string, discount?: number }[] = [];
  let parkedCarts: { cart_id: number, cart_name: string, added_at: string }[] = [];
  let isSearching = false;
  let searchTimeout: NodeJS.Timeout | null = null;
  let vatRate = 0.1;
  let searchError: string | null = null;
  let addError: string | null = null;
  let cartNamePrompt: string = '';
  let showCartNamePrompt = false;
  let showPaymentPopup = false;
  let invoiceId: string | null = null;
  let ttlMinutes = 10;
  let ttlTimer: any = null;

  onMount(async () => {
    if (!$user.isLoggedIn) {
      goto('/');
      return;
    }
    const searchInput = document.getElementById('search-input') as HTMLInputElement;
    if (searchInput) searchInput.focus();
    await loadParkedCarts();
    vatRate = getVATRate();
    startTTLTimer();
    console.log('Tauri detected on mount:', typeof window !== 'undefined' && typeof window.__TAURI__ === 'object');
    if (typeof window !== 'undefined' && window.__TAURI__) {
      console.log('Tauri invoke test:', await invoke('ping').catch(e => 'Error: ' + e));
    }
  });

  function handleLogout() {
    logout();
    goto('/');
  }

  function handleSearch() {
    if (searchTimeout) clearTimeout(searchTimeout);
    searchTimeout = setTimeout(() => performSearch(), 300);
  }

  async function performSearch() {
    if (!searchQuery.trim()) {
      products = [];
      searchError = null;
      return;
    }
    isSearching = true;
    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        const result = await invoke('search_products', { query: searchQuery });
        if (Array.isArray(result)) {
          products = result.map(p => ({
            product_id: p.product_id,
            name: p.name,
            barcode: p.barcode,
            price: p.price,
            bulk_price: p.bulk_price,
            bulk_single_conversion: p.bulk_single_conversion,
            unit: p.unit
          }));
          searchError = null;
        } else if (typeof result === 'string') {
          products = [];
          searchError = result;
        } else {
          products = [];
          searchError = 'Unknown error from backend';
        }
      } else {
        products = [];
        searchError = 'Search only works in the desktop app.';
      }
    } catch (error) {
      products = [];
      searchError = error?.toString() || 'Unknown error';
    } finally {
      isSearching = false;
    }
  }

  function handleAddToCart(product: { product_id: number, name: string, barcode: string | null, price: number, bulk_price?: number, bulk_single_conversion?: number, unit?: string }, priceType: 'single' | 'bulk') {
    addError = null;
    const unitPrice = priceType === 'single' ? product.price : (product.bulk_price || product.price);
    let updatedCart = [...tempCart];
    const existingItem = updatedCart.find(item => item.product_id === product.product_id && item.purchasing_type === priceType);
    if (existingItem) {
      existingItem.quantity += 1;
    } else {
      updatedCart.push({
        product_id: product.product_id,
        product_name: product.name,
        product_barcode: product.barcode,
        quantity: 1,
        unit_price: unitPrice,
        purchasing_type: priceType,
        discount: 0
      });
    }
    tempCart = updatedCart;
    console.log('Added to tempCart:', tempCart);
  }

  function handleUpdateQuantity(item: { product_id: number, purchasing_type: string, quantity: number, discount?: number }, newQuantity: number) {
    if (newQuantity < 0) newQuantity = 0;
    const updatedCart = tempCart.map(i => 
      i.product_id === item.product_id && i.purchasing_type === item.purchasing_type ? { ...i, quantity: newQuantity } : i
    );
    tempCart = updatedCart;
  }

  function handleUpdateDiscount(item: { product_id: number, purchasing_type: string, discount?: number }, newDiscount: number) {
    if (newDiscount < 0) newDiscount = 0;
    const updatedCart = tempCart.map(i => 
      i.product_id === item.product_id && i.purchasing_type === item.purchasing_type ? { ...i, discount: newDiscount } : i
    );
    tempCart = updatedCart;
  }

  function handleRemoveItem(item: { product_id: number, purchasing_type: string }) {
    tempCart = tempCart.filter(i => !(i.product_id === item.product_id && i.purchasing_type === item.purchasing_type));
  }

  async function handleParkCart() {
    if (tempCart.length === 0) return;
    showCartNamePrompt = true;
    cartNamePrompt = 'Parked Cart';
  }

  async function confirmCartName() {
    if (!cartNamePrompt.trim()) return;
    try {
      const cartId = (await invoke('create_cart', { cartName: cartNamePrompt.trim() }) as { cart_id: number }).cart_id;
      for (const item of tempCart) {
        await invoke('add_cart_item', {
          cartId,
          productId: item.product_id,
          barcode: item.product_barcode,
          quantity: item.quantity,
          unitPrice: item.unit_price,
          purchasingType: item.purchasing_type,
          discount: item.discount || 0
        });
      }
      await parkCart(cartId, cartNamePrompt.trim());
      tempCart = [];
      await loadParkedCarts();
      showCartNamePrompt = false;
    } catch (e) {
      addError = 'Failed to park cart: ' + (e?.toString() || 'Unknown error');
    }
  }

  async function handleActivateCart(cartId: number) {
    try {
      const items = await listCartItems(cartId);
      tempCart = items.map(item => ({
        product_id: item.product_id,
        product_name: item.product_name || 'Unknown',
        product_barcode: item.product_barcode || null,
        quantity: item.quantity,
        unit_price: item.unit_price,
        purchasing_type: item.purchasing_type,
        discount: item.discount || 0
      }));
      await activateCart(cartId);
      await loadParkedCarts();
    } catch (e) {
      addError = 'Failed to activate cart: ' + (e?.toString() || 'Unknown error');
    }
  }

  function handleClearCart() {
    if (confirm('Are you sure you want to cancel this order?')) {
      tempCart = [];
    }
  }

  function handleCheckoutCash() {
    if (tempCart.length > 0) showPaymentPopup = true; // Placeholder for cash payment
  }

  function handleCheckoutQR() {
    if (tempCart.length > 0) showPaymentPopup = true; // Placeholder for QR payment
  }

  async function confirmPaymentInPopup() {
    try {
      const cartId = (await invoke('create_cart', { cartName: 'Checkout Temp' }) as { cart_id: number }).cart_id;
      for (const item of tempCart) {
        await invoke('add_cart_item', {
          cartId,
          productId: item.product_id,
          barcode: item.product_barcode,
          quantity: item.quantity,
          unitPrice: item.unit_price,
          purchasingType: item.purchasing_type,
          discount: item.discount || 0
        });
      }
      invoiceId = await checkoutCart(cartId, 'default_store', 'default_storeman');
      await confirmPayment(cartId);
      tempCart = [];
      showPaymentPopup = false;
    } catch (e) {
      addError = 'Failed to complete payment: ' + (e?.toString() || 'Unknown error');
    }
  }

  async function loadParkedCarts() {
    parkedCarts = await listParkedCarts();
  }

  function startTTLTimer() {
    if (ttlTimer) clearInterval(ttlTimer);
    ttlTimer = setInterval(async () => {
      await cleanupExpiredCarts(ttlMinutes);
      await loadParkedCarts();
    }, 60000);
  }

  function calculateTotal(): number {
    return tempCart.reduce((total, item) => total + (item.quantity * item.unit_price) - (item.discount || 0), 0);
  }
  function calculateVAT(): number {
    return calculateTotal() * vatRate;
  }
  function calculateFinalTotal(): number {
    return calculateTotal() + calculateVAT();
  }
  function formatPrice(price: number): string {
    return price.toLocaleString('vi-VN');
  }
  function formatUnitPrice(product: { price: number, unit?: string }, priceType: 'single' | 'bulk'): string {
    const unit = product.unit || 'unit';
    return `${formatPrice(product.price)}/${unit}`;
  }
  function formatBulkPrice(product: { bulk_price?: number, bulk_single_conversion?: number, unit?: string }): string {
    if (product.bulk_price && product.bulk_single_conversion && product.unit) {
      return `${formatPrice(product.bulk_price)}/${product.bulk_single_conversion} ${product.unit}`;
    }
    return '-';
  }
</script>

<svelte:head>
  <title>AnPOS - Create Order</title>
  <style>
    :global(body) {
      font-family: Arial, sans-serif;
      font-size: 12px; /* Default, adjustable 10px-14px via settings */
    }
  </style>
</svelte:head>

<div class="create-order-container">
  <div class="search-area">
    <div class="search-table-wrapper">
      <table class="search-results">
        <thead>
          <tr>
            <th class="product-col">
              <div class="search-header">
                <img src="/scan_barcode.svg" alt="Scan Barcode" class="icon-guide" />
                <input
                  id="search-input"
                  type="text"
                  bind:value={searchQuery}
                  placeholder="Scan barcode or search by name..."
                  class="search-bar"
                  on:input={handleSearch}
                />
              </div>
            </th>
            <th class="price-col">Single Price</th>
            <th class="add-col"> </th>
            <th class="price-col">Bulk Price</th>
            <th class="add-col"> </th>
          </tr>
        </thead>
        <tbody>
          {#if products.length === 0}
            <tr><td colspan="5" class="no-products">{searchQuery ? 'No products found.' : 'Scan or type to search.'}</td></tr>
          {:else}
            {#each products as product}
              <tr>
                <td class="product-col">
                  <div class="product-name" title={product.name}><strong>{product.name}</strong></div>
                  <div class="product-barcode" title={product.barcode || 'No barcode'}>{product.barcode || 'N/A'}</div>
                </td>
                <td class="price-col">
                  <div class="price"><strong style="float: right;">{formatPrice(product.price)}</strong></div>
                  <div class="unit" style="color: #aaa; float: right;">{product.unit || 'unit'}</div>
                </td>
                <td class="add-col">
                  <button class="add-btn rounded" title="Add single" on:click={() => handleAddToCart(product, 'single')}>
                    <span class="plus-icon">+</span>
                  </button>
                </td>
                <td class="price-col">
                  <div class="price"><strong style="float: right;">{formatPrice(product.bulk_price || product.price)}</strong></div>
                  <div class="unit" style="color: #aaa; float: right;">{product.bulk_single_conversion && product.unit ? `${product.bulk_single_conversion} ${product.unit}` : '-'}</div>
                </td>
                <td class="add-col">
                  {#if product.bulk_price && product.bulk_single_conversion}
                    <button class="add-btn rounded" title="Add bulk" on:click={() => handleAddToCart(product, 'bulk')}>
                      <span class="plus-icon">+</span>
                    </button>
                  {:else}
                    <button class="add-btn rounded" title="Add bulk" disabled>
                      <span class="plus-icon">+</span>
                    </button>
                  {/if}
                </td>
              </tr>
            {/each}
          {/if}
        </tbody>
      </table>
    </div>
    {#if searchError}
      <div class="search-error">{searchError}</div>
    {/if}
  </div>
  <div class="cart-area">
    <div class="cart-header">
      <span>CART</span>
    </div>
    <div class="cart-content">
      <div class="cart-left">
        <div class="cart-table-wrapper">
          <table class="cart-table">
            <thead>
              <tr>
                <th>Product</th>
                <th>Price</th>
                <th>Quantity</th>
                <th style="text-align: right;">Total</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {#if tempCart.length === 0}
                <tr><td colspan="5" class="empty-cart">Cart is empty</td></tr>
              {:else}
                {#each tempCart.slice().reverse() as item}
                  <tr style="height: 50px;">
                    <td><span class="cart-name">{item.product_name}</span></td>
                    <td>{formatPrice(item.unit_price)}</td>
                    <td>
                      <div class="qty-controls">
                        <span class="qty-mod" style="color: #f44336;" on:click={() => handleUpdateQuantity(item, item.quantity - 1)}>-</span>
                        <input type="number" class="qty-input" value={item.quantity} min="1" on:change={(e) => {
                          const target = e.target as HTMLInputElement;
                          if (target) handleUpdateQuantity(item, Number(target.value));
                        }} />
                        <span class="qty-mod" style="color: #4CAF50;" on:click={() => handleUpdateQuantity(item, item.quantity + 1)}>+</span>
                      </div>
                    </td>
                    <td style="text-align: right;">{formatPrice(item.quantity * item.unit_price - (item.discount || 0))}</td>
                    <td><span class="delete-mod" style="color: #f44336; font-size: 1.5rem;" on:click={() => handleRemoveItem(item)}>×</span></td>
                  </tr>
                {/each}
              {/if}
            </tbody>
          </table>
        </div>
        <div class="cart-summary">
          <div class="cart-summary-row"><span>Subtotal</span><span>{formatPrice(calculateTotal())}</span></div>
          <div class="cart-summary-row"><span>VAT ({(vatRate * 100).toFixed(0)}%)</span><span>{formatPrice(calculateVAT())}</span></div>
          <div class="cart-summary-row cart-summary-final"><span>Total</span><span>{formatPrice(calculateFinalTotal())}</span></div>
        </div>
      </div>
      <div class="cart-right">
        <div class="cart-actions">
          <button class="cart-btn" on:click={handleCheckoutCash}>
            <img src="/pay_with_cash.svg" alt="Cash" class="action-icon" /> Cash
          </button>
          <button class="cart-btn" on:click={handleCheckoutQR}>
            <img src="/pay_with_QR.svg" alt="Payment QR" class="action-icon" /> Payment QR
          </button>
          <button class="cart-btn" on:click={handleParkCart}>
            <img src="/cart.svg" alt="Park" class="action-icon" /> Park Cart
          </button>
          <button class="cart-btn" on:click={handleClearCart}>
            <img src="/cancel.svg" alt="Cancel" class="action-icon" /> Cancel Order
          </button>
        </div>
        <div class="parked-carts-area">
          <h3>Parked Carts</h3>
          {#each parkedCarts as parked}
            <div class="parked-cart-row">
              <span>{parked.cart_name} ({parked.added_at})</span>
              <button on:click={() => handleActivateCart(parked.cart_id)}>Activate</button>
            </div>
          {/each}
        </div>
      </div>
    </div>
    {#if addError}
      <div class="add-error">{addError}</div>
    {/if}
  </div>
</div>

{#if showCartNamePrompt}
  <div class="cart-name-prompt-modal">
    <div class="cart-name-prompt-content">
      <label>Enter Cart Name:</label>
      <input type="text" bind:value={cartNamePrompt} />
      <button on:click={confirmCartName}>Park</button>
      <button on:click={() => showCartNamePrompt = false}>Cancel</button>
    </div>
  </div>
{/if}

{#if showPaymentPopup}
  <div class="payment-modal">
    <div class="payment-content">
      <h3>Payment</h3>
      <p>Total: {formatPrice(calculateTotal())}</p>
      <button on:click={confirmPaymentInPopup}>Confirm</button>
      <button on:click={() => showPaymentPopup = false}>Cancel</button>
    </div>
  </div>
{/if}

<style>
  :global(html) {
    font-family: Arial, sans-serif;
    font-size: 12px; /* Default, adjustable 10px-14px via settings */
  }
  .create-order-container {
    display: flex;
    flex-direction: row;
    width: 100%;
    height: calc(100vh - 60px);
    gap: 2rem;
    margin: 0 30px;
  }
  .search-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    border: 1px solid #ddd;
    padding: 10px;
  }
  .search-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .icon-guide {
    width: 24px;
    height: 24px;
    pointer-events: none; /* Unclickable */
  }
  .search-bar {
    flex: 1;
    padding: 0.5rem;
    font-size: 1rem;
    border: 2px solid #4CAF50;
    border-radius: 8px;
    outline: none;
  }
  .search-table-wrapper {
    flex: 1;
    overflow-y: auto;
    height: calc(100vh - 200px);
  }
  .search-results {
    width: 100%;
    border-collapse: collapse;
  }
  .search-results th,
  .search-results td {
    padding: 0.5rem;
    text-align: left;
    border-bottom: 1px solid #ddd;
  }
  .product-col { width: calc(40% - 50px); }
  .price-col { width: 20%; }
  .add-col { width: 10%; }
  .no-products { text-align: center; color: #666; }
  .search-error { color: red; }
  .add-btn {
    padding: 0.25rem 0.5rem;
    background-color: #4CAF50;
    color: white;
    border: none;
    cursor: pointer;
  }
  .add-btn.rounded {
    border-radius: 8px;
  }
  .add-btn:disabled { background-color: #cccccc; cursor: not-allowed; }
  .product-name { font-weight: bold; color: #333; }
  .product-barcode { color: #aaa; }
  .price { font-weight: bold; }
  .unit { color: #aaa; }
  .cart-area {
    width: 400px;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    border: 1px solid #ddd;
    padding: 10px;
    margin-right: 30px;
  }
  .cart-content {
    display: flex;
    flex-direction: row;
    gap: 1rem;
    flex: 1;
  }
  .cart-left {
    flex: 1;
    display: flex;
    flex-direction: column;
  }
  .cart-right {
    width: 200px;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .cart-header {
    background-color: white;
    color: #333;
    padding: 0.5rem;
    font-size: 1.2rem;
    font-weight: bold;
    border-bottom: 1px solid #ddd;
    text-align: center;
  }
  .cart-table-wrapper {
    flex: 1;
    overflow-y: auto;
    height: calc(100vh - 300px);
  }
  .cart-table {
    width: 100%;
    border-collapse: collapse;
  }
  .cart-table thead {
    position: sticky;
    top: 0;
    background-color: #f5f5f5;
    z-index: 1;
  }
  .cart-table th,
  .cart-table td {
    padding: 0.5rem;
    text-align: left;
    border-bottom: 1px solid #ddd;
  }
  .cart-name { font-weight: bold; }
  .qty-controls {
    display: flex;
    gap: 0.25rem;
  }
  .qty-mod {
    font-size: 1.2rem;
    cursor: pointer;
  }
  .qty-input {
    width: 3rem;
    padding: 0.25rem;
    text-align: center;
  }
  .delete-mod {
    font-size: 1.5rem;
    cursor: pointer;
  }
  .cart-summary {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    margin-top: 1rem;
  }
  .cart-summary-row {
    display: flex;
    justify-content: space-between;
  }
  .cart-summary-final {
    font-weight: bold;
  }
  .cart-actions {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .cart-btn {
    padding: 0.75rem 1rem;
    color: #333;
    border: 1px solid #ccc;
    background: white;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    font-size: 0.9rem;
    border-radius: 4px;
    transition: background-color 0.2s;
  }
  .cart-btn:hover {
    background-color: #f5f5f5;
  }
  .cart-btn img {
    width: 24px;
    height: 24px;
  }
  .cart-btn:nth-child(1) img {
    filter: brightness(0) saturate(100%) invert(48%) sepia(79%) saturate(2476%) hue-rotate(86deg) brightness(118%) contrast(119%);
  }
  .cart-btn:nth-child(2) img {
    filter: brightness(0) saturate(100%) invert(27%) sepia(51%) saturate(2878%) hue-rotate(346deg) brightness(104%) contrast(97%);
  }
  .cart-btn:nth-child(3) img {
    filter: brightness(0) saturate(100%) invert(27%) sepia(0%) saturate(0%) hue-rotate(93deg) brightness(104%) contrast(86%);
  }
  .cart-btn:nth-child(4) img {
    filter: brightness(0) saturate(100%) invert(27%) sepia(51%) saturate(2878%) hue-rotate(346deg) brightness(104%) contrast(97%);
  }
  .action-icon {
    width: 24px;
    height: 24px;
    vertical-align: middle;
  }
  .empty-cart { text-align: center; color: #666; }
  .add-error { color: red; }
  .parked-carts-area {
    border-top: 1px solid #ddd;
    padding-top: 10px;
  }
  .parked-carts-area h3 {
    margin: 0 0 0.5rem 0;
    font-size: 1rem;
  }
  .parked-cart-row {
    display: flex;
    justify-content: space-between;
    padding: 0.5rem;
    border-bottom: 1px solid #ddd;
    font-size: 0.8rem;
  }
  .cart-name-prompt-modal {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    justify-content: center;
    align-items: center;
  }
  .cart-name-prompt-content {
    background: white;
    padding: 1rem;
    border-radius: 5px;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .payment-modal {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    justify-content: center;
    align-items: center;
  }
  .payment-content {
    background: white;
    padding: 1rem;
    border-radius: 5px;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
</style>