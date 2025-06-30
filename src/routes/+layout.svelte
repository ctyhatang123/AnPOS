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
    height: 30px;
    background: white;
    color: #333;
    font-weight: 600;
    font-size: 0.9rem;
    letter-spacing: 0.05em;
    box-shadow: 0 1px 3px rgba(0,0,0,0.1);
    border-bottom: 1px solid #ddd;
  }
  .anpos-title {
    flex: 0 0 auto;
    font-size: 1.1em;
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
    color: #333;
    text-decoration: none;
    padding: 0.2em 0.8em;
    border-radius: 4px;
    transition: background 0.2s, color 0.2s;
    font-size: 0.9em;
    opacity: 0.8;
  }
  .anpos-tabs a.active-tab {
    background: #f0f0f0;
    color: #333;
    opacity: 1;
    font-weight: bold;
  }
  .logout-btn {
    flex: 0 0 auto;
    margin-left: auto;
    background: #f0f0f0;
    color: #333;
    border: none;
    border-radius: 4px;
    padding: 0.2em 0.8em;
    font-size: 0.9em;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
    opacity: 0.8;
  }
  .logout-btn:hover {
    background: #e0e0e0;
    opacity: 1;
  }
  .anpos-main {
    width: 100vw;
    height: calc(100vh - 30px);
    margin: 0;
    padding-top: 0.5rem;
    box-sizing: border-box;
    min-width: 300px;
    max-width: 100vw;
  }
</style> 