<script lang="ts">
  import { onMount } from 'svelte';
  import { initializeStores, setupActivityListeners } from '$lib/store';
  import '../app.css';
  import { page } from '$app/stores';
  import { derived } from 'svelte/store';
  import { logout } from '$lib/store';
  import { goto } from '$app/navigation';

  onMount(async () => {
    try {
      await initializeStores();
      setupActivityListeners();
    } catch (error) {
      console.error('Failed to initialize app:', error);
    }
  });

  // Tab definitions
  const tabs = [
    { name: 'Create order', path: '/create-order' },
    { name: 'Inventory', path: '/inventory' },
    { name: 'Statistics', path: '/statistics' },
    { name: 'Settings', path: '/settings' }
  ];

  // Determine active tab
  const activeTab = derived(page, $page => {
    const current = $page.url.pathname;
    return tabs.find(tab => current.startsWith(tab.path))?.path || '/create-order';
  });

  function handleLogout() {
    logout();
    goto('/');
  }
</script>

{#if $page.url.pathname !== '/'}
  <div class="anpos-header">
    <div class="anpos-title">AnPOS</div>
    <div class="anpos-tabs">
      {#each tabs as tab}
        <a
          href={tab.path}
          class:active-tab={$activeTab === tab.path}
        >
          {tab.name}
        </a>
      {/each}
    </div>
    <button class="logout-btn" on:click={handleLogout}>Logout</button>
  </div>
{/if}
<main class="anpos-main">
  <slot />
</main>

<style>
  :global(html) {
    font-size: 12px;
  }
  .anpos-header {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    padding: 0 2rem;
    height: 40px;
    background: linear-gradient(90deg, #7856D1 0%, #401E98 100%);
    color: white;
    font-weight: 600;
    font-size: 1rem;
    letter-spacing: 0.05em;
    box-shadow: 0 2px 8px rgba(64,30,152,0.08);
  }
  .anpos-title {
    flex: 0 0 auto;
    font-size: 1.2em;
    margin-right: 2rem;
    font-weight: bold;
    letter-spacing: 0.1em;
  }
  .anpos-tabs {
    flex: 1 1 auto;
    display: flex;
    justify-content: center;
    gap: 2rem;
  }
  .anpos-tabs a {
    color: white;
    text-decoration: none;
    padding: 0.3em 1em;
    border-radius: 999px;
    transition: background 0.2s, color 0.2s;
    font-size: 1em;
    opacity: 0.85;
  }
  .anpos-tabs a.active-tab {
    background: rgba(255,255,255,0.18);
    color: #fff;
    opacity: 1;
    font-weight: bold;
  }
  .logout-btn {
    flex: 0 0 auto;
    margin-left: auto;
    background: rgba(255,255,255,0.18);
    color: #fff;
    border: none;
    border-radius: 999px;
    padding: 0.3em 1.2em;
    font-size: 1em;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
    opacity: 0.85;
  }
  .logout-btn:hover {
    background: rgba(255,255,255,0.32);
    opacity: 1;
  }
  .anpos-main {
    width: 100vw;
    height: calc(100vh - 40px);
    margin: 0;
    padding-top: 0.5rem;
    box-sizing: border-box;
    min-width: 300px;
    max-width: 100vw;
  }
</style> 