<script lang="ts">
  import { onMount } from "svelte";

  let domains = $state<string[]>([]);
  let newDomain = $state("");
  let loading = $state(true);
  let error = $state<string | null>(null);
  let adding = $state(false);

  async function fetchBlocklist() {
    try {
      // @ts-ignore
      const result = await window.__TAURI__.core.invoke("get_blocklist");
      domains = result;
      error = null;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function addDomain() {
    if (!newDomain.trim()) return;

    const domain = newDomain.trim().toLowerCase();
    adding = true;

    try {
      // @ts-ignore
      await window.__TAURI__.core.invoke("add_domain", { domain });
      newDomain = "";
      await fetchBlocklist();
    } catch (e) {
      error = String(e);
    } finally {
      adding = false;
    }
  }

  async function removeDomain(domain: string) {
    try {
      // @ts-ignore
      await window.__TAURI__.core.invoke("remove_domain", { domain });
      await fetchBlocklist();
    } catch (e) {
      error = String(e);
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      addDomain();
    }
  }

  onMount(() => {
    fetchBlocklist();
  });
</script>

<div class="blocklist-editor">
  <div class="add-domain">
    <input
      type="text"
      placeholder="Enter domain (e.g., example.com)"
      bind:value={newDomain}
      onkeydown={handleKeydown}
      disabled={adding}
    />
    <button onclick={addDomain} disabled={adding || !newDomain.trim()}>
      {adding ? "Adding..." : "Add"}
    </button>
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if loading}
    <div class="loading">Loading blocklist...</div>
  {:else if domains.length === 0}
    <div class="empty">
      <p>No domains in blocklist</p>
      <p class="hint">Add domains above to start blocking</p>
    </div>
  {:else}
    <div class="domain-list">
      {#each domains as domain}
        <div class="domain-item">
          <span class="domain-name">{domain}</span>
          <button class="remove-btn" onclick={() => removeDomain(domain)} title="Remove">
            ×
          </button>
        </div>
      {/each}
    </div>
  {/if}

  <div class="info">
    <p>Subdomains are automatically blocked. Adding <code>example.com</code> also blocks <code>www.example.com</code>, <code>api.example.com</code>, etc.</p>
  </div>
</div>

<style>
  .blocklist-editor {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .add-domain {
    display: flex;
    gap: 0.5rem;
  }

  input {
    flex: 1;
    padding: 0.75rem;
    border: 1px solid #0f3460;
    border-radius: 6px;
    background: #16213e;
    color: #eee;
    font-size: 0.875rem;
  }

  input:focus {
    outline: none;
    border-color: #e94560;
  }

  input::placeholder {
    color: #666;
  }

  button {
    padding: 0.75rem 1.25rem;
    border: none;
    border-radius: 6px;
    background: #e94560;
    color: white;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  button:hover:not(:disabled) {
    background: #d63b55;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .loading, .empty {
    text-align: center;
    color: #888;
    padding: 2rem;
  }

  .empty .hint {
    font-size: 0.875rem;
    color: #666;
  }

  .domain-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-height: 250px;
    overflow-y: auto;
  }

  .domain-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: #16213e;
    border-radius: 6px;
    padding: 0.75rem 1rem;
  }

  .domain-name {
    font-family: monospace;
    font-size: 0.875rem;
  }

  .remove-btn {
    width: 28px;
    height: 28px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: 1px solid #dc3545;
    color: #dc3545;
    font-size: 1.25rem;
    line-height: 1;
  }

  .remove-btn:hover {
    background: #dc3545;
    color: white;
  }

  .error {
    background: #3d1f1f;
    border: 1px solid #dc3545;
    border-radius: 8px;
    padding: 1rem;
    color: #ff6b6b;
    font-size: 0.875rem;
  }

  .info {
    margin-top: 1rem;
    padding: 1rem;
    background: #0f3460;
    border-radius: 8px;
    font-size: 0.8rem;
    color: #aaa;
  }

  .info p {
    margin: 0;
  }

  code {
    background: #16213e;
    padding: 0.125rem 0.375rem;
    border-radius: 4px;
    font-family: monospace;
    color: #e94560;
  }
</style>
