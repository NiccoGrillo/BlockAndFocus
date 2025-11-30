<script lang="ts">
  import { onMount } from "svelte";

  interface Props {
    onRequestBypass: (data: { challengeId: string; questions: string[]; expiresAt: number }) => void;
  }

  let { onRequestBypass }: Props = $props();

  let status = $state({
    blocking_active: false,
    schedule_enabled: false,
    schedule_active: false,
    bypass_active: false,
    bypass_remaining_seconds: null as number | null,
    blocked_count: 0,
    daemon_connected: false,
  });

  let loading = $state(true);
  let error = $state<string | null>(null);
  let bypassDuration = $state(15);
  let requestingBypass = $state(false);

  async function fetchStatus() {
    try {
      // @ts-ignore - Tauri is available globally
      const result = await window.__TAURI__.core.invoke("get_status");
      status = result;
      error = null;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function requestBypass() {
    requestingBypass = true;
    try {
      // @ts-ignore
      const quiz = await window.__TAURI__.core.invoke("request_bypass", {
        durationMinutes: bypassDuration,
      });
      onRequestBypass({
        challengeId: quiz.challenge_id,
        questions: quiz.questions,
        expiresAt: quiz.expires_at,
      });
    } catch (e) {
      error = String(e);
    } finally {
      requestingBypass = false;
    }
  }

  async function cancelBypass() {
    try {
      // @ts-ignore
      await window.__TAURI__.core.invoke("cancel_bypass");
      await fetchStatus();
    } catch (e) {
      error = String(e);
    }
  }

  onMount(() => {
    fetchStatus();
    const interval = setInterval(fetchStatus, 2000);
    return () => clearInterval(interval);
  });

  function formatTime(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}:${secs.toString().padStart(2, "0")}`;
  }
</script>

<div class="status-panel">
  {#if loading}
    <div class="loading">Loading...</div>
  {:else if !status.daemon_connected}
    <div class="warning">
      <span class="icon">⚠️</span>
      <div>
        <strong>Daemon not connected</strong>
        <p>The BlockAndFocus daemon is not running. Start it to enable blocking.</p>
      </div>
    </div>
  {:else}
    <div class="status-grid">
      <div class="status-item">
        <span class="label">Blocking</span>
        <span class="value" class:active={status.blocking_active}>
          {status.blocking_active ? "Active" : "Inactive"}
        </span>
      </div>

      <div class="status-item">
        <span class="label">Schedule</span>
        <span class="value" class:active={status.schedule_enabled && status.schedule_active}>
          {#if !status.schedule_enabled}
            Disabled
          {:else if status.schedule_active}
            Active
          {:else}
            Inactive
          {/if}
        </span>
      </div>

      <div class="status-item">
        <span class="label">Blocked Queries</span>
        <span class="value">{status.blocked_count}</span>
      </div>

      {#if status.bypass_active && status.bypass_remaining_seconds}
        <div class="status-item bypass">
          <span class="label">Bypass Active</span>
          <span class="value">{formatTime(status.bypass_remaining_seconds)}</span>
        </div>
      {/if}
    </div>

    <div class="actions">
      {#if status.bypass_active}
        <button class="btn-danger" onclick={cancelBypass}>Cancel Bypass</button>
      {:else}
        <div class="bypass-request">
          <label>
            Bypass for
            <select bind:value={bypassDuration}>
              <option value={5}>5 min</option>
              <option value={15}>15 min</option>
              <option value={30}>30 min</option>
              <option value={60}>1 hour</option>
            </select>
          </label>
          <button class="btn-primary" onclick={requestBypass} disabled={requestingBypass}>
            {requestingBypass ? "Loading..." : "Request Bypass"}
          </button>
        </div>
      {/if}
    </div>

    {#if error}
      <div class="error">{error}</div>
    {/if}
  {/if}
</div>

<style>
  .status-panel {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .loading {
    text-align: center;
    color: #888;
    padding: 2rem;
  }

  .warning {
    display: flex;
    gap: 1rem;
    align-items: flex-start;
    background: #3d2914;
    border: 1px solid #6b4423;
    border-radius: 8px;
    padding: 1rem;
  }

  .warning .icon {
    font-size: 1.5rem;
  }

  .warning strong {
    color: #ffc107;
  }

  .warning p {
    margin: 0.5rem 0 0 0;
    color: #ccc;
    font-size: 0.875rem;
  }

  .status-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 1rem;
  }

  .status-item {
    background: #16213e;
    border-radius: 8px;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .status-item.bypass {
    grid-column: span 2;
    background: #2d1f3d;
    border: 1px solid #e94560;
  }

  .label {
    font-size: 0.75rem;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .value {
    font-size: 1.25rem;
    font-weight: 600;
    color: #888;
  }

  .value.active {
    color: #4caf50;
  }

  .actions {
    margin-top: 1rem;
  }

  .bypass-request {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .bypass-request label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: #ccc;
    font-size: 0.875rem;
  }

  select {
    padding: 0.5rem;
    border-radius: 6px;
    border: 1px solid #0f3460;
    background: #16213e;
    color: #eee;
  }

  button {
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-primary {
    background: #e94560;
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    background: #d63b55;
  }

  .btn-danger {
    background: #dc3545;
    color: white;
  }

  .btn-danger:hover {
    background: #c82333;
  }

  .error {
    background: #3d1f1f;
    border: 1px solid #dc3545;
    border-radius: 8px;
    padding: 1rem;
    color: #ff6b6b;
    font-size: 0.875rem;
  }
</style>
