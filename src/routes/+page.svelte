<script lang="ts">
  import { onMount } from 'svelte';
  import { user, login } from '$lib/store';
  import { loginUser } from '$lib/db';
  import { goto } from '$app/navigation';
  import { invoke } from '@tauri-apps/api/tauri'; // Add Tauri import

  let username = '';
  let password = '';
  let errorMessage = '';
  let isLoading = true;
  let isLoggingIn = false;

  function isTauri() {
    return typeof window !== 'undefined' && typeof window.__TAURI__ === 'object';
  }

  console.log('TAURI DETECTED (window.__TAURI__):', isTauri());
  console.log('window.__TAURI__:', typeof window !== 'undefined' ? window.__TAURI__ : undefined);
  console.log('window.tauri:', typeof window !== 'undefined' ? window.tauri : undefined);
  console.log('window.invoke:', typeof window !== 'undefined' ? window.invoke : undefined);
  console.log('navigator.userAgent:', typeof window !== 'undefined' ? window.navigator.userAgent : undefined);

  onMount(() => {
    // Auto-focus username input for first-time login
    const usernameInput = document.getElementById('username-input') as HTMLInputElement;
    if (usernameInput) {
      usernameInput.focus();
    }
    isLoading = false;
  });

  async function handleSubmit(event: Event) {
    event.preventDefault();
    
    if (!username.trim()) {
      errorMessage = 'Please enter a username';
      return;
    }

    if (!password.trim()) {
      errorMessage = 'Please enter a password';
      return;
    }

    isLoggingIn = true;
    errorMessage = '';

    try {
      const result = await loginUser(username, password);
      
      if (result.success && result.userId) {
        // Login successful - update user store
        login(result.userId, username);
        
        // Redirect to create-order page
        await goto('/create-order');
        // Debug Tauri invoke after login
        if (isTauri()) {
          console.log('Tauri invoke test:', await invoke('ping')); // Test invoke
        }
      } else {
        errorMessage = result.error || 'Login failed';
      }
    } catch (error) {
      console.error('Login error:', error);
      errorMessage = 'Login failed. Please try again.';
    } finally {
      isLoggingIn = false;
    }
  }

  function handleKeyPress(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      handleSubmit(event);
    }
  }

  function handleChangeUser() {
    // TODO: Implement change user functionality in task 7
    console.log('Change user clicked');
  }
</script>

<svelte:head>
  <title>AnPOS - Login</title>
</svelte:head>

{#if isLoading}
  <div class="loading">
    <div class="spinner"></div>
    <p>Loading...</p>
  </div>
{:else}
  <div class="login-container">
    <div class="login-card">
      <h1 class="title">AnPOS</h1>
      <p class="subtitle">Point of Sale System</p>
      
      <form on:submit={handleSubmit} class="login-form">
        <div class="form-group">
          <label for="username-input">Username</label>
          <input
            id="username-input"
            type="text"
            bind:value={username}
            class="input"
            placeholder="Enter username"
            disabled={isLoggingIn}
          />
        </div>
        
        <div class="form-group">
          <label for="password-input">Password</label>
          <input
            id="password-input"
            type="password"
            bind:value={password}
            on:keypress={handleKeyPress}
            class="input"
            placeholder="Enter password"
            autocomplete="current-password"
            disabled={isLoggingIn}
          />
        </div>
        
        {#if errorMessage}
          <div class="error-message">
            {errorMessage}
          </div>
        {/if}
        
        <button type="submit" class="login-button" disabled={isLoggingIn}>
          {isLoggingIn ? 'Logging in...' : 'Login'}
        </button>
      </form>
      
      <div class="change-user">
        <button type="button" class="change-user-link" on:click={handleChangeUser}>
          Change User
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100vh;
    background: linear-gradient(135deg, #7856D1 0%, #401E98 100%);
    color: white;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 4px solid rgba(255, 255, 255, 0.3);
    border-top: 4px solid white;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 1rem;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .login-container {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    background: linear-gradient(135deg, #7856D1 0%, #401E98 100%);
    padding: 1rem;
  }

  .login-card {
    background: white;
    padding: 2rem;
    border-radius: 12px;
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.2);
    width: 100%;
    max-width: 400px;
  }

  .title {
    text-align: center;
    color: #333;
    margin: 0 0 0.5rem 0;
    font-size: 2rem;
    font-weight: bold;
  }

  .subtitle {
    text-align: center;
    color: #666;
    margin: 0 0 2rem 0;
    font-size: 1rem;
  }

  .login-form {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .form-group label {
    font-weight: 500;
    color: #333;
    font-size: 0.9rem;
  }

  .input {
    padding: 0.75rem;
    border: 2px solid #e1e5e9;
    border-radius: 8px;
    font-size: 1rem;
    transition: border-color 0.2s;
  }

  .input:focus {
    outline: none;
    border-color: #7856D1;
  }

  .input:disabled {
    background-color: #f8f9fa;
    color: #6c757d;
    cursor: not-allowed;
  }

  .login-button {
    background: linear-gradient(135deg, #7856D1 0%, #401E98 100%);
    color: white;
    padding: 0.75rem;
    border: none;
    border-radius: 8px;
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    transition: transform 0.2s;
  }

  .login-button:hover {
    transform: translateY(-1px);
  }

  .login-button:active {
    transform: translateY(0);
  }

  .error-message {
    background-color: #fee;
    color: #c33;
    padding: 0.75rem;
    border-radius: 8px;
    font-size: 0.9rem;
    text-align: center;
  }

  .change-user {
    text-align: center;
    margin-top: 1rem;
  }

  .change-user-link {
    color: #7856D1;
    text-decoration: none;
    font-size: 0.9rem;
    text-decoration: underline;
  }

  .change-user-link:hover {
    color: #401E98;
  }
</style>
