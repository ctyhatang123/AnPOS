<script lang="ts">
import { onMount } from 'svelte';

let fontSize = 16;

onMount(() => {
  const saved = localStorage.getItem('anpos_font_size');
  if (saved) {
    fontSize = parseInt(saved, 10);
    setFontSize(fontSize);
  }
});

function setFontSize(size: number) {
  document.documentElement.style.fontSize = `${size}px`;
  localStorage.setItem('anpos_font_size', String(size));
}

function handleFontSizeChange(e: Event) {
  fontSize = parseInt((e.target as HTMLInputElement).value, 10);
  setFontSize(fontSize);
}
</script>

<div class="settings-container">
  <h2>Settings</h2>
  <div class="setting-row">
    <label for="font-size-slider">Font Size: {fontSize}px</label>
    <input
      id="font-size-slider"
      type="range"
      min="12"
      max="16"
      step="1"
      bind:value={fontSize}
      on:input={handleFontSizeChange}
    />
  </div>
</div>

<style>
.settings-container {
  max-width: 480px;
  margin: 2rem auto;
  padding: 2rem;
  background: #fff;
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(64,30,152,0.08);
}
.setting-row {
  display: flex;
  align-items: center;
  gap: 1.5rem;
  margin-bottom: 2rem;
}
label {
  font-size: 1.1em;
  font-weight: 500;
}
input[type='range'] {
  width: 200px;
}
</style> 