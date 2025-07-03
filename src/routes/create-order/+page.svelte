<script lang="ts">
  import { onMount, afterUpdate } from 'svelte';
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
  import QRCode from 'qrcode';
  import { invoke } from '@tauri-apps/api/tauri';

  let searchQuery = '';
  let products: { product_id: number, name: string, barcode: string | null, price: number, bulk_price?: number, bulk_single_conversion?: number, unit?: string }[] = [];
  let tempCart: { product_id: number, product_name: string, product_barcode: string | null, quantity: number, unit_price: number, purchasing_type: string, discount?: number, bulk_single_conversion?: number, unit?: string }[] = [];
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
  let receiptTimestamp: Date | null = null;
  let receiptNumber: string | null = null;
  let showReceiptPopup = false;
  const posCode = '123'; // Placeholder, replace with real POS code
  const sellerCode = '5678'; // Placeholder, replace with real seller code
  let orderNumber = 1; // Placeholder, should increment per day
  let barcodeSvg = '';
  let vietQrDataUrl = '';
  let dailyReceiptCount = 0;
  let lastReceiptDate = '';
  let vietQrError = '';

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
        discount: 0,
        bulk_single_conversion: product.bulk_single_conversion,
        unit: product.unit
      });
    }
    tempCart = updatedCart;
    console.log('Added to tempCart:', tempCart);
    searchQuery = '';
    setTimeout(() => {
      const searchInput = document.getElementById('search-input') as HTMLInputElement;
      if (searchInput) searchInput.focus();
    }, 0);
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

  function formatReceiptTimestamp(date: Date) {
    // Format: dd/MM/yyyy HH:mm
    return `${date.getDate().toString().padStart(2, '0')}/` +
      `${(date.getMonth() + 1).toString().padStart(2, '0')}/` +
      `${date.getFullYear()} ` +
      `${date.getHours().toString().padStart(2, '0')}:` +
      `${date.getMinutes().toString().padStart(2, '0')}`;
  }

  function generateReceiptId(date: Date): string {
    const pad = (n: number, l = 2) => n.toString().padStart(l, '0');
    const y = date.getFullYear().toString().slice(-2);
    const m = pad(date.getMonth() + 1);
    const d = pad(date.getDate());
    const today = y + m + d;
    if (lastReceiptDate !== today) {
      lastReceiptDate = today;
      dailyReceiptCount = 1;
    } else {
      dailyReceiptCount++;
    }
    return `RE${today}${pad(dailyReceiptCount, 3)}`;
  }

  function handleCheckoutCash() {
    if (tempCart.length > 0) {
      // Capture timestamp and generate receipt number
      receiptTimestamp = new Date();
      receiptNumber = generateReceiptId(receiptTimestamp);
      showReceiptPopup = true;
    }
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

  $: subtotal = tempCart.reduce((total, item) => total + (item.quantity * item.unit_price - (item.discount || 0)), 0);
  $: vat = subtotal * vatRate;
  $: total = subtotal + vat;

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

  function closeReceiptPopup() {
    showReceiptPopup = false;
  }

  function renderBarcode() {
    if (receiptNumber) {
      // Use JsBarcode if available, else fallback to placeholder
      try {
        // @ts-ignore
        if (window.JsBarcode) {
          const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
          // @ts-ignore
          window.JsBarcode(svg, receiptNumber, { format: 'CODE128', width: 2, height: 40, displayValue: false });
          barcodeSvg = svg.outerHTML;
        } else {
          barcodeSvg = `<svg width='180' height='40'><rect width='180' height='40' fill='#eee'/><text x='50%' y='50%' dominant-baseline='middle' text-anchor='middle' fill='#888' font-size='14'>[Barcode]</text></svg>`;
        }
      } catch {
        barcodeSvg = `<svg width='180' height='40'><rect width='180' height='40' fill='#eee'/><text x='50%' y='50%' dominant-baseline='middle' text-anchor='middle' fill='#888' font-size='14'>[Barcode]</text></svg>`;
      }
    }
  }

  function generateVietQRUrl(amount: number, message: string): string {
    const bin = '970423';
    const accountNumber = '0886668890';
    const accountName = encodeURIComponent('NGUYEN XUAN DUONG');
    const encodedMessage = encodeURIComponent(message);
    return `https://img.vietqr.io/image/${bin}-${accountNumber}-compact2.jpg?amount=${amount}&addInfo=${encodedMessage}&accountName=${accountName}`;
  }

  afterUpdate(() => {
    if (showReceiptPopup) {
      renderBarcode();
    }
  });

  function closeCheckoutWindow() {
    showReceiptPopup = false;
  }

  function truncateName(name: string, maxLength: number): string {
    if (!name) return '';
    return name.length > maxLength ? name.slice(0, maxLength - 3) + '...' : name;
  }

  // Ensure vietQrDataUrl is always set using a string for message
  $: vietQrDataUrl = generateVietQRUrl(Math.round(total), receiptNumber || '');

  // Update the helper to show price/unit or price/bulk_single_conversion unit in receipt
  function formatReceiptUnitPrice(item: any) {
    if (item.purchasing_type === 'bulk' && item.unit_price && item.bulk_single_conversion && item.unit) {
      return `${formatPrice(item.unit_price)}/${item.bulk_single_conversion} ${item.unit}`;
    } else if (item.purchasing_type === 'single' && item.unit_price && item.unit) {
      return `${formatPrice(item.unit_price)}/${item.unit}`;
    } else {
      return formatPrice(item.unit_price);
    }
  }
</script>

<svelte:head>
  <title>AnPOS - Create Order</title>
  <style>
    :global(body) {
      font-family: Arial, sans-serif;
      font-size: 12px; /* Default, adjustable 10px-14px via settings */
      background: #f5f5f5;
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
                  <div class="product-name" title={product.name}>{product.name}</div>
                  <div class="product-barcode" title={product.barcode || 'No barcode'}>{product.barcode || 'N/A'}</div>
                </td>
                <td class="price-col">
                  <div class="price">{formatPrice(product.price)}</div>
                  <div class="unit">{product.unit || 'unit'}</div>
                </td>
                <td class="add-col">
                  <button class="add-btn rounded" title="Add single" on:click={() => handleAddToCart(product, 'single')}>
                    <span class="plus-icon">+</span>
                  </button>
                </td>
                <td class="price-col">
                  <div class="price">{formatPrice(product.bulk_price || product.price)}</div>
                  <div class="unit">{product.bulk_single_conversion && product.unit ? `${product.bulk_single_conversion} ${product.unit}` : '-'}</div>
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
    <div class="cart-content">
      <div class="cart-left">
        <div class="cart-table-wrapper">
          <table class="cart-table">
            <thead>
              <tr>
                <th>Product</th>
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
                    <td>
                      <div class="cart-name">{item.product_name}</div>
                      <div class="cart-price" style="color: #aaa; font-size: 0.9em;">{formatPrice(item.unit_price)}</div>
                    </td>
                    <td>
                      <div class="qty-controls">
                        <span class="qty-mod" style="color: #f44336;" on:click={() => handleUpdateQuantity(item, item.quantity - 1)}>-</span>
                        <input type="number" class="qty-input" value={item.quantity} min="1" on:change={(e) => {
                          const target = e.target;
                          if (target && target instanceof HTMLInputElement) handleUpdateQuantity(item, Number(target.value));
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
      </div>
      <div class="cart-right">
        <div class="cart-summary">
          <div class="cart-summary-row"><span>Subtotal</span><span>{formatPrice(subtotal)}</span></div>
          <div class="cart-summary-row"><span>VAT ({(vatRate * 100).toFixed(0)}%)</span><span>{formatPrice(vat)}</span></div>
          <div class="cart-summary-row cart-summary-final"><span>TOTAL</span><span>{formatPrice(total)}</span></div>
        </div>
        <div class="cart-actions">
          <button class="checkout-btn solid-green" on:click={handleCheckoutCash}>
            <span class="btn-icon icon-btn-icon" style="width:24px;height:24px;display:flex;align-items:center;">
              <svg class="svg-icon" width="24" height="24" viewBox="0 0 1024 1024" fill="#fff" xmlns="http://www.w3.org/2000/svg"><path d="M298.666667 640h128a42.666667 42.666667 0 0 0 0-85.333333H298.666667a42.666667 42.666667 0 0 0 0 85.333333zM810.666667 213.333333H213.333333a128 128 0 0 0-128 128v384a128 128 0 0 0 128 128h597.333334a128 128 0 0 0 128-128V341.333333a128 128 0 0 0-128-128z m42.666666 512a42.666667 42.666667 0 0 1-42.666666 42.666667H213.333333a42.666667 42.666667 0 0 1-42.666666-42.666667v-256h682.666666z m0-341.333333H170.666667V341.333333a42.666667 42.666667 0 0 1 42.666666-42.666666h597.333334a42.666667 42.666667 0 0 1 42.666666 42.666666z"/></svg>
            </span>
            Checkout
          </button>
          <button class="parkcart-btn solid-gray" on:click={handleParkCart}>
            <span class="btn-icon icon-btn-icon" style="width:24px;height:24px;display:flex;align-items:center;">
              <svg class="svg-icon" width="24" height="24" viewBox="0 0 24 24" fill="#fff" xmlns="http://www.w3.org/2000/svg"><path d="M7 18c-1.104 0-2 .896-2 2s.896 2 2 2 2-.896 2-2-.896-2-2-2zm10 0c-1.104 0-2 .896-2 2s.896 2 2 2 2-.896 2-2-.896-2-2-2zM7.334 16h9.334c.828 0 1.54-.672 1.658-1.488l1.334-8A1.5 1.5 0 0 0 18.167 5H5.833l-.167-1A1.5 1.5 0 0 0 4.167 3H2v2h1.333l2.6 15.39A2.001 2.001 0 0 0 7.334 21h9.334a2.001 2.001 0 0 0 1.401-.61l.001-.001A2.001 2.001 0 0 0 18.667 19H7.334a2.001 2.001 0 0 0-1.401.61l-.001.001A2.001 2.001 0 0 0 7.334 21z"/></svg>
            </span>
            Park Cart
          </button>
          <button class="cancel-btn solid-red" on:click={handleClearCart}>
            <span class="btn-icon icon-btn-icon" style="width:24px;height:24px;display:flex;align-items:center;">
              <svg viewBox="0 0 512 512" width="24" height="24" fill="#fff" xmlns="http://www.w3.org/2000/svg"><g id="Page-1" stroke="none" stroke-width="1" fill="none" fill-rule="evenodd"><g id="work-case" fill="#fff" transform="translate(91.520000, 91.520000)"><polygon id="Close" points="328.96 30.2933333 298.666667 1.42108547e-14 164.48 134.4 30.2933333 1.42108547e-14 1.42108547e-14 30.2933333 134.4 164.48 1.42108547e-14 298.666667 30.2933333 328.96 164.48 194.56 298.666667 328.96 328.96 298.666667 194.56 164.48"/></g></g></svg>
            </span>
            Cancel Order
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
      <p>Total: {formatPrice(subtotal)}</p>
      <button on:click={confirmPaymentInPopup}>Confirm</button>
      <button on:click={() => showPaymentPopup = false}>Cancel</button>
    </div>
  </div>
{/if}

{#if showReceiptPopup}
  <div class="checkout-modal">
    <div class="checkout-content checkout-content-small">
      <div class="checkout-body-row">
        <div class="checkout-receipt-part compact-receipt">
          <div class="checkout-header-row">
            <h2 class="invoice-title">Invoice</h2>
          </div>
          <div class="receipt-meta-row">
            <div class="receipt-id">Receipt ID: <span class="receipt-number">{receiptNumber}</span></div>
            <div class="receipt-date-cashier">{receiptTimestamp ? formatReceiptTimestamp(receiptTimestamp) : ''} | Cashier: admin</div>
          </div>
          <div class="receipt-barcode small-barcode left-barcode">{@html barcodeSvg}</div>
          <table class="receipt-table compact-table">
            <thead>
              <tr>
                <th style="width: 54%">Product</th>
                <th style="width: 18%; text-align: center;">Qtt</th>
                <th style="width: 28%; text-align: right;">Total</th>
              </tr>
            </thead>
            <tbody>
              {#each tempCart.slice(0, 5) as item}
                <tr>
                  <td>
                    <div class="receipt-product-name ellipsis" title={item.product_name}>{truncateName(item.product_name, 22)}</div>
                    <div class="receipt-product-detail">
                      {formatReceiptUnitPrice(item)}
                    </div>
                  </td>
                  <td style="text-align: center;">{item.quantity}</td>
                  <td style="text-align: right;">{formatPrice(item.quantity * item.unit_price - (item.discount || 0))}</td>
                </tr>
              {/each}
              {#if tempCart.length > 5}
                <tr><td colspan="3" class="receipt-more-items">... and {tempCart.length - 5} more items ...</td></tr>
              {/if}
            </tbody>
          </table>
          <div class="receipt-summary">
            <div class="receipt-summary-row small-summary"><span>Subtotal ({tempCart.length} items):</span><span>{formatPrice(subtotal)}</span></div>
            <div class="receipt-summary-row small-summary"><span>VAT (10%):</span><span>{formatPrice(vat)}</span></div>
            <hr class="receipt-summary-separator" />
            <div class="receipt-summary-row receipt-summary-final"><span>Total:</span><span>{formatPrice(total)}</span></div>
          </div>
        </div>
        <div class="vietqr-section payment-section">
          <button class="checkout-close-x" on:click={closeCheckoutWindow} title="Close">×</button>
          <div class="payment-title">Payment Method</div>
          {#if vietQrDataUrl}
            <div class="vietqr-frame wide-qr-frame">
              <img class="vietqr-img full-width-qr" src={vietQrDataUrl} alt="VietQR Payment" on:error={() => vietQrDataUrl = ''} />
            </div>
          {/if}
          <div class="checkout-btn-row horizontal-btn-row icon-only-btn-row">
            <div class="icon-btn-col">
              <button class="checkout-btn solid-green icon-only-btn" on:click={closeCheckoutWindow}>
                <span class="btn-icon icon-btn-icon" style="width:24px;height:24px;display:flex;align-items:center;">
                  <svg class="svg-icon" width="24" height="24" viewBox="0 0 1024 1024" fill="#fff" xmlns="http://www.w3.org/2000/svg"><path d="M298.666667 640h128a42.666667 42.666667 0 0 0 0-85.333333H298.666667a42.666667 42.666667 0 0 0 0 85.333333zM810.666667 213.333333H213.333333a128 128 0 0 0-128 128v384a128 128 0 0 0 128 128h597.333334a128 128 0 0 0 128-128V341.333333a128 128 0 0 0-128-128z m42.666666 512a42.666667 42.666667 0 0 1-42.666666 42.666667H213.333333a42.666667 42.666667 0 0 1-42.666666-42.666667v-256h682.666666z m0-341.333333H170.666667V341.333333a42.666667 42.666667 0 0 1 42.666666-42.666666h597.333334a42.666667 42.666667 0 0 1 42.666666 42.666666z"/></svg>
                </span>
                <span class="icon-btn-text">Cash</span>
              </button>
            </div>
            <div class="icon-btn-col">
              <button class="checkout-btn solid-green icon-only-btn" on:click={closeCheckoutWindow}>
                <span class="btn-icon icon-btn-icon" style="width:24px;height:24px;display:flex;align-items:center;">
                  <svg class="svg-icon" width="24" height="24" viewBox="0 0 1024 1024" fill="#fff" xmlns="http://www.w3.org/2000/svg"><path d="M426.666667 469.333333H128c-25.6 0-42.666667-17.066667-42.666667-42.666666V128c0-25.6 17.066667-42.666667 42.666667-42.666667h298.666667c25.6 0 42.666667 17.066667 42.666666 42.666667v298.666667c0 25.6-17.066667 42.666667-42.666666 42.666666zM170.666667 384h213.333333V170.666667H170.666667v213.333333zM896 469.333333h-298.666667c-25.6 0-42.666667-17.066667-42.666666-42.666666V128c0-25.6 17.066667-42.666667 42.666666-42.666667h298.666667c25.6 0 42.666667 17.066667 42.666667 42.666667v298.666667c0 25.6-17.066667 42.666667-42.666667 42.666666z m-256-85.333333h213.333333V170.666667h-213.333333v213.333333zM426.666667 938.666667H128c-25.6 0-42.666667-17.066667-42.666667-42.666667v-298.666667c0-25.6 17.066667-42.666667 42.666667-42.666666h298.666667c25.6 0 42.666667 17.066667 42.666666 42.666666v298.666667c0 25.6-17.066667 42.666667-42.666666 42.666667z m-256-85.333334h213.333333v-213.333333H170.666667v213.333333zM896 789.333333h-149.333333c-25.6 0-42.666667-17.066667-42.666667-42.666666V640H640v106.666667c0 25.6-17.066667 42.666667-42.666667 42.666666s-42.666667-17.066667-42.666666-42.666666V597.333333c0-25.6 17.066667-42.666667 42.666666-42.666666h149.333334c25.6 0 42.666667 17.066667 42.666666 42.666666v106.666667H853.333333V597.333333c0-25.6 17.066667-42.666667 42.666667-42.666666s42.666667 17.066667 42.666667 42.666666v149.333334c0 25.6-17.066667 42.666667-42.666667 42.666666zM746.666667 938.666667H597.333333c-25.6 0-42.666667-17.066667-42.666666-42.666667s17.066667-42.666667 42.666666-42.666667h149.333334c25.6 0 42.666667 17.066667 42.666666 42.666667s-17.066667 42.666667-42.666666 42.666667z"/><path d="M302.933333 789.333333H251.733333c-8.533333 0-17.066667-8.533333-17.066666-17.066666v-51.2c0-8.533333 8.533333-17.066667 17.066666-17.066667h51.2c8.533333 0 17.066667 8.533333 17.066667 17.066667v51.2c0 8.533333-8.533333 17.066667-17.066667 17.066666zM302.933333 320H251.733333c-8.533333 0-17.066667-8.533333-17.066666-17.066667V251.733333c0-8.533333 8.533333-17.066667 17.066666-17.066666h51.2c8.533333 0 17.066667 8.533333 17.066667 17.066666v51.2c0 8.533333-8.533333 17.066667-17.066667 17.066667zM772.266667 320h-51.2c-8.533333 0-17.066667-8.533333-17.066667-17.066667V251.733333c0-8.533333 8.533333-17.066667 17.066667-17.066666h51.2c8.533333 0 17.066667 8.533333 17.066666 17.066666v51.2c0 8.533333-8.533333 17.066667-17.066666 17.066667zM896 938.666667c-12.8 0-21.333333-4.266667-29.866667-12.8-8.533333-8.533333-12.8-17.066667-12.8-29.866667 0-12.8 4.266667-21.333333 12.8-29.866667 17.066667-17.066667 42.666667-17.066667 59.733334 0 8.533333 8.533333 12.8 21.333333 12.8 29.866667 0 12.8-4.266667 21.333333-12.8 29.866667-8.533333 8.533333-17.066667 12.8-29.866667 12.8z"/></svg>
                </span>
                <span class="icon-btn-text">QR</span>
              </button>
            </div>
            <div class="icon-btn-col">
              <button class="cancel-btn solid-red icon-only-btn" on:click={closeCheckoutWindow}>
                <span class="btn-icon icon-btn-icon" style="width:24px;height:24px;display:flex;align-items:center;">
                  <svg viewBox="0 0 512 512" width="24" height="24" fill="#fff" xmlns="http://www.w3.org/2000/svg"><g id="Page-1" stroke="none" stroke-width="1" fill="none" fill-rule="evenodd"><g id="work-case" fill="#fff" transform="translate(91.520000, 91.520000)"><polygon id="Close" points="328.96 30.2933333 298.666667 1.42108547e-14 164.48 134.4 30.2933333 1.42108547e-14 1.42108547e-14 30.2933333 134.4 164.48 1.42108547e-14 298.666667 30.2933333 328.96 164.48 194.56 298.666667 328.96 328.96 298.666667 194.56 164.48"/></g></g></svg>
                </span>
                <span class="icon-btn-text">Cancel</span>
              </button>
            </div>
          </div>
        </div>
      </div>
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
    width: calc(100vw - 50px);
    height: calc(100vh - 50px);
    gap: 1%;
    margin: 0 0 5px 1vw;
    background: #ffffff;
    box-sizing: border-box;
    overflow: hidden;
  }
  .search-area {
    width: calc(47vw - 40px);
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    border: 1px solid #ddd;
    padding: 5px 5px 5px 10px;
    box-sizing: border-box;
    flex-shrink: 0;
    height: calc(100vh - 80px);
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
    height: calc(100vh - 290px);
    box-sizing: border-box;
  }
  .search-results {
    width: 100%;
    border-collapse: collapse;
    table-layout: fixed;
  }
  .search-results thead {
    position: sticky;
    top: 0;
    z-index: 2;
    background: #f5f5f5;
  }
  .search-results th,
  .search-results td {
    padding: 0.5rem;
    text-align: left;
    border-bottom: 1px solid #ddd;
    background: #fff;
  }
  .search-results th.product-col, .search-results td.product-col {
    width: calc(58% - 40px);
    max-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .search-results th.price-col, .search-results td.price-col {
    width: 13%;
    min-width: 70px;
    max-width: 90px;
    text-align: right;
  }
  .search-results th.add-col, .search-results td.add-col {
    width: 5%;
    min-width: 40px;
    max-width: 50px;
  }
  .product-name, .product-barcode {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: block;
  }
  .product-name {
    font-weight: 500;
    color: #222;
  }
  .product-barcode {
    color: #aaa;
    font-weight: 300;
  }
  .unit {
    color: #aaa;
    font-weight: 300;
  }
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
  .cart-area {
    width: calc(51vw - 40px);
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    border: 1px solid #ddd;
    padding: 5px 10px 5px 5px;
    box-sizing: border-box;
    height: calc(100vh - 80px);
  }
  .cart-content {
    display: flex;
    flex-direction: row;
    gap: 2%;
    flex: 1;
    min-height: 0;
    box-sizing: border-box;
  }
  .cart-left {
    flex: 0 0 70%;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    box-sizing: border-box;
  }
  .cart-right {
    flex: 0 0 28%;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    min-width: 0;
    max-width: 220px;
    box-sizing: border-box;
  }
  .cart-table-wrapper {
    flex: 1;
    overflow-y: auto;
    height: 100%;
    min-height: 0;
    max-height: 100%;
    box-sizing: border-box;
  }
  .cart-table {
    width: 100%;
    border-collapse: collapse;
    table-layout: fixed;
  }
  .cart-table thead {
    position: sticky;
    top: 0;
    background: #f5f5f5;
    z-index: 2;
  }
  .cart-table th,
  .cart-table td {
    padding: 0.5rem;
    text-align: left;
    border-bottom: 1px solid #ddd;
  }
  .cart-table th:first-child, .cart-table td:first-child {
    width: 60%;
  }
  .cart-table th:nth-child(2), .cart-table td:nth-child(2) {
    width: 20%;
  }
  .cart-table th:nth-child(3), .cart-table td:nth-child(3) {
    width: 15%;
    text-align: right;
  }
  .cart-table th:last-child, .cart-table td:last-child {
    width: 5%;
  }
  .cart-name {
    font-weight: 500;
    color: #222;
  }
  .cart-price {
    color: #aaa;
    font-size: 0.9em;
    font-weight: 300;
  }
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
    padding-top: 0.5rem;
  }
  .cart-summary-row {
    display: flex;
    flex-direction: row;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    box-sizing: border-box;
  }
  .cart-summary-row span:first-child {
    text-align: left;
    flex: 1;
    box-sizing: border-box;
  }
  .cart-summary-row span:last-child {
    text-align: right;
    flex: 1;
    box-sizing: border-box;
  }
  .cart-summary-row.cart-summary-final {
    margin-bottom: 30px;
  }
  .cart-actions {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .checkout-btn.solid-green {
    background: #22c55e;
    color: #fff;
    border: none;
    border-radius: 8px;
    font-weight: 600;
    font-size: 1.1rem;
    padding: 0.7rem 2.2rem;
    display: flex;
    align-items: center;
    gap: 0.7rem;
    box-shadow: 0 2px 8px rgba(34,197,94,0.08);
    transition: background 0.2s;
  }
  .checkout-btn.solid-green:hover {
    background: #16a34a;
  }
  .parkcart-btn.solid-gray {
    background: #6b7280;
    color: #fff;
    border: none;
    border-radius: 8px;
    font-weight: 600;
    font-size: 1.1rem;
    padding: 0.7rem 2.2rem;
    display: flex;
    align-items: center;
    gap: 0.7rem;
    box-shadow: 0 2px 8px rgba(160,160,170,0.08);
    transition: background 0.2s;
  }
  .parkcart-btn.solid-gray:hover {
    background: #4b5563;
  }
  .cancel-btn.solid-red {
    background: #ef4444;
    color: #fff;
    border: none;
    border-radius: 8px;
    font-weight: 600;
    font-size: 1.1rem;
    padding: 0.7rem 2.2rem;
    display: flex;
    align-items: center;
    gap: 0.7rem;
    box-shadow: 0 2px 8px rgba(239,68,68,0.08);
    transition: background 0.2s;
  }
  .cancel-btn.solid-red:hover {
    background: #b91c1c;
  }
  .btn-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.3em;
    height: 1.3em;
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
  .checkout-modal {
    position: fixed;
    top: 0; left: 0; width: 100vw; height: 100vh;
    background: rgba(0,0,0,0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
  .checkout-content {
    background: #fff;
    border-radius: 8px;
    padding: 2rem;
    min-width: 350px;
    max-width: 95vw;
    max-height: 600px;
    box-shadow: 0 2px 16px rgba(0,0,0,0.2);
    display: flex;
    flex-direction: column;
    gap: 1rem;
    overflow: auto;
    font-size: 0.95rem;
  }
  .checkout-content-small {
    font-size: 0.92rem;
  }
  .checkout-header {
    text-align: center;
    border-bottom: none;
    padding-bottom: 0.5rem;
    background: #fff;
    position: static;
  }
  .checkout-header h2 {
    margin: 0 0 0.5rem 0;
    text-align: center;
    font-size: 1.5rem;
    font-weight: bold;
  }
  .checkout-body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    justify-content: flex-start;
  }
  .receipt-meta {
    font-size: 0.95rem;
    color: #444;
    margin-bottom: 0.5rem;
    text-align: center;
  }
  .receipt-number {
    font-family: monospace;
    font-size: 1rem;
    color: #333;
  }
  .receipt-barcode {
    text-align: center;
    font-size: 1rem;
    margin-bottom: 1rem;
    color: #888;
    font-family: monospace;
    min-height: 40px;
  }
  .receipt-table {
    width: 100%;
    border-collapse: collapse;
    margin-bottom: 1rem;
  }
  .receipt-table th, .receipt-table td {
    padding: 0.5rem;
    border-bottom: 1px solid #eee;
    font-size: 1rem;
  }
  .receipt-more-items {
    text-align: center;
    color: #888;
    font-style: italic;
    font-size: 1rem;
  }
  .receipt-product-name {
    font-weight: 500;
    color: #222;
  }
  .receipt-product-detail {
    color: #888;
    font-size: 0.95em;
    font-weight: 300;
    margin-top: 2px;
  }
  .receipt-summary {
    margin-top: 1rem;
    border-top: 1px solid #ddd;
    padding-top: 0.5rem;
    font-size: 1.1rem;
  }
  .receipt-summary-row {
    display: flex;
    justify-content: space-between;
    margin-bottom: 0.25rem;
  }
  .receipt-summary-separator {
    border: none;
    border-top: 1px solid #bbb;
    margin: 0.25rem 0 0.5rem 0;
  }
  .receipt-summary-final {
    font-weight: bold;
    font-size: 1.2rem;
    margin-top: 0.5rem;
  }
  .receipt-close-btn {
    margin-top: 1rem;
    align-self: center;
    padding: 0.5rem 2rem;
    font-size: 1rem;
    border: none;
    border-radius: 4px;
    background: #4caf50;
    color: #fff;
    cursor: pointer;
    font-weight: 500;
  }
  .receipt-close-btn:hover {
    background: #388e3c;
  }
  .vietqr-section {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    margin-top: 0.5rem;
    gap: 1rem;
    position: relative;
  }
  .checkout-close-x {
    position: absolute;
    top: 8px;
    right: 16px;
    background: none;
    border: none;
    font-size: 1.5rem;
    color: #888;
    cursor: pointer;
    z-index: 10;
    padding: 0;
    line-height: 1;
    transition: color 0.2s;
  }
  .checkout-close-x:hover {
    color: #f44336;
  }
  .checkout-body-row {
    display: flex;
    flex-direction: row;
    gap: 2rem;
    width: 100%;
    min-height: 0;
    flex: 1;
    align-items: flex-start;
  }
  .checkout-receipt-part {
    flex: 2;
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: flex-start;
  }
  .receipt-product-type {
    color: #888;
    font-size: 0.95em;
    font-weight: 300;
    margin-left: 0.25em;
  }
  .checkout-btn-row {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    width: 100%;
    margin-top: 1.5rem;
  }
  .receipt-confirm-btn {
    padding: 0.5rem 1.5rem;
    font-size: 1rem;
    border: none;
    border-radius: 4px;
    background: #4caf50;
    color: #fff;
    cursor: pointer;
    font-weight: 500;
    transition: background 0.2s;
  }
  .receipt-confirm-btn:hover {
    background: #388e3c;
  }
  .vietqr-title {
    text-align: center;
    font-size: 1.2rem;
    font-weight: bold;
    margin-bottom: 0.5rem;
    color: #222;
    letter-spacing: 1px;
  }
  .vietqr-frame {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    border: 2px solid #e0e0e0;
    border-radius: 12px;
    padding: 10px 10px 0 10px;
    background: #fff;
    position: relative;
    margin-bottom: 0.5rem;
    width: 220px;
    margin-left: auto;
    margin-right: auto;
  }
  .vietqr-logo {
    position: absolute;
    left: 50%;
    bottom: -18px;
    transform: translateX(-50%);
    background: #fff;
    border-radius: 50%;
    padding: 2px 6px;
    box-shadow: 0 1px 4px rgba(0,0,0,0.08);
  }
  .vietqr-logo img {
    width: 36px;
    height: 36px;
    display: block;
  }
  .compact-receipt {
    max-width: 303px;
    min-width: 295px;
    width: 100%;
    border: 1.5px solid #d1d5db;
    border-radius: 10px;
    background: #fff;
    box-sizing: border-box;
    padding-left: 3px;
    padding-right: 3px;
    display: flex;
    flex-direction: column;
    justify-content: stretch;
  }
  .invoice-title {
    font-size: 1.1rem;
    font-weight: bold;
    margin: 0 0 0.5rem 0;
    text-align: left;
  }
  .receipt-meta-row {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    margin-bottom: 0.2rem;
  }
  .receipt-date-cashier {
    font-size: 0.95rem;
    color: #444;
    text-align: left;
  }
  .small-barcode {
    width: 120px;
    margin: 0.2rem auto 0.5rem auto;
  }
  .compact-table th, .compact-table td {
    padding: 0.18rem 0.3rem;
    font-size: 0.92rem;
  }
  .compact-table th:first-child, .compact-table td:first-child {
    width: 54%;
    max-width: 110px;
  }
  .ellipsis {
    max-width: 110px;
  }
  .payment-section {
    min-width: 320px;
    max-width: 380px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: stretch;
    background: #fafbfc;
    border-radius: 12px;
    box-shadow: 0 2px 8px rgba(0,0,0,0.04);
    padding: 1.5rem 1.5rem 2rem 1.5rem;
    margin-left: 2rem;
    border: 1.5px solid #d1d5db;
    box-sizing: border-box;
  }
  .payment-title {
    font-size: 1.2rem;
    font-weight: bold;
    margin-bottom: 0.25rem;
    color: #222;
    text-align: center;
  }
  .payment-subtitle {
    font-size: 1rem;
    color: #444;
    margin-bottom: 0.5rem;
    text-align: center;
  }
  .vietqr-logo-top {
    display: flex;
    justify-content: center;
    align-items: center;
    margin-bottom: 0.5rem;
  }
  .vietqr-logo-top img {
    width: 90px;
    height: auto;
  }
  .vietqr-img.big-qr {
    width: 200px;
    height: 200px;
  }
  .vietqr-desc {
    text-align: center;
    font-size: 1rem;
    color: #222;
    margin-bottom: 0.25rem;
  }
  .vietqr-error {
    text-align: center;
    color: red;
    font-size: 0.95rem;
    margin-bottom: 1rem;
  }
  .big-green-btn {
    background: #22c55e;
    color: #fff;
    font-size: 1.1rem;
    font-weight: bold;
    padding: 0.75rem 2.5rem;
    border-radius: 8px;
    margin-top: 1rem;
    box-shadow: 0 2px 8px rgba(34,197,94,0.08);
    transition: background 0.2s;
  }
  .big-green-btn:hover {
    background: #16a34a;
  }
  .left-barcode {
    text-align: left;
    margin-left: 0;
  }
  .small-summary {
    font-size: 0.92rem;
    font-weight: 400;
  }
  .vietqr-frame.wide-qr-frame {
    width: 100%;
    max-width: 100%;
    padding: 0;
    margin: 0 0 0.5rem 0;
    border: none;
    background: none;
    display: flex;
    justify-content: center;
  }
  .full-width-qr {
    width: 100%;
    max-width: 320px;
    height: auto;
    display: block;
    margin: 0 auto;
  }
  .btn-icon img,
  .icon-btn-icon img {
    width: 64px !important;
    height: 64px !important;
    filter: brightness(0) invert(1);
    display: block;
    margin: auto;
  }
  .payment-info-2row {
    text-align: center;
    font-size: 1rem;
    color: #222;
    margin-bottom: 0.25rem;
    line-height: 1.4;
  }
  .horizontal-btn-row {
    display: flex;
    flex-direction: row;
    gap: 1.25rem;
    justify-content: center;
    margin-top: 0.7rem;
  }
  .small-btn-row {
    margin-top: 0.7rem;
  }
  .small-btn {
    font-size: 0.8rem;
    font-weight: 400;
    padding: 0.38rem 0.9rem;
    min-width: 100px;
    max-width: 120px;
    height: 34px;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    justify-content: center;
    border-radius: 8px;
  }
  .small-btn-icon img {
    width: 28px !important;
    height: 28px !important;
    filter: brightness(0) invert(1);
    display: block;
  }
  .icon-only-btn-row {
    display: flex;
    flex-direction: row;
    gap: 1.25rem;
    justify-content: center;
    margin-top: 0.7rem;
  }
  .icon-btn-col {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 120px;
  }
  .icon-only-btn {
    width: 110px;
    height: 44px;
    padding: 0 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 12px;
    gap: 0.5rem;
  }
  .icon-btn-text {
    font-size: 1rem;
    color: #fff;
    font-weight: 500;
    line-height: 1;
    display: inline-block;
  }
</style>