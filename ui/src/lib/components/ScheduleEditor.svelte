<script lang="ts">
  import { onMount } from "svelte";

  interface ScheduleRule {
    name: string;
    days: string[];
    start_time: string;
    end_time: string;
  }

  interface Schedule {
    enabled: boolean;
    rules: ScheduleRule[];
  }

  let schedule = $state<Schedule | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let toggling = $state(false);

  const dayNames: Record<string, string> = {
    mon: "Mon",
    tue: "Tue",
    wed: "Wed",
    thu: "Thu",
    fri: "Fri",
    sat: "Sat",
    sun: "Sun",
  };

  async function fetchSchedule() {
    try {
      // @ts-ignore
      const result = await window.__TAURI__.core.invoke("get_schedule");
      schedule = result;
      error = null;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function toggleEnabled() {
    if (!schedule) return;
    toggling = true;
    try {
      // @ts-ignore
      await window.__TAURI__.core.invoke("set_schedule_enabled", { enabled: !schedule.enabled });
      await fetchSchedule();
    } catch (e) {
      error = String(e);
    } finally {
      toggling = false;
    }
  }

  function formatDays(days: string[]): string {
    return days.map((d) => dayNames[d.toLowerCase()] || d).join(", ");
  }

  function formatTime(time: string): string {
    const [hours, minutes] = time.split(":");
    const h = parseInt(hours);
    const ampm = h >= 12 ? "PM" : "AM";
    const hour12 = h % 12 || 12;
    return `${hour12}:${minutes} ${ampm}`;
  }

  onMount(() => {
    fetchSchedule();
  });
</script>

<div class="schedule-editor">
  {#if loading}
    <div class="loading">Loading schedule...</div>
  {:else if !schedule}
    <div class="error">Failed to load schedule</div>
  {:else}
    <div class="toggle-section">
      <div class="toggle-info">
        <strong>Schedule-based Blocking</strong>
        <p>When enabled, blocking only activates during scheduled times</p>
      </div>
      <button
        class="toggle-btn"
        class:active={schedule.enabled}
        onclick={toggleEnabled}
        disabled={toggling}
      >
        {schedule.enabled ? "Enabled" : "Disabled"}
      </button>
    </div>

    {#if error}
      <div class="error">{error}</div>
    {/if}

    {#if schedule.rules.length === 0}
      <div class="empty">
        <p>No schedule rules configured</p>
        <p class="hint">Edit config.toml to add schedule rules</p>
      </div>
    {:else}
      <div class="rules-list">
        <h3>Schedule Rules</h3>
        {#each schedule.rules as rule}
          <div class="rule-item" class:inactive={!schedule.enabled}>
            <div class="rule-header">
              <span class="rule-name">{rule.name}</span>
            </div>
            <div class="rule-details">
              <div class="rule-days">
                {formatDays(rule.days)}
              </div>
              <div class="rule-time">
                {formatTime(rule.start_time)} - {formatTime(rule.end_time)}
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}

    <div class="info">
      <p>
        To add or modify schedule rules, edit the configuration file at:<br />
        <code>~/.config/blockandfocus/config.toml</code>
      </p>
    </div>
  {/if}
</div>

<style>
  .schedule-editor {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .toggle-section {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: #16213e;
    border-radius: 8px;
    padding: 1rem;
  }

  .toggle-info strong {
    display: block;
    margin-bottom: 0.25rem;
  }

  .toggle-info p {
    margin: 0;
    font-size: 0.8rem;
    color: #888;
  }

  .toggle-btn {
    padding: 0.5rem 1rem;
    border: 2px solid #888;
    border-radius: 20px;
    background: transparent;
    color: #888;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .toggle-btn.active {
    border-color: #4caf50;
    background: #4caf50;
    color: white;
  }

  .toggle-btn:disabled {
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

  .rules-list h3 {
    margin: 0 0 1rem 0;
    font-size: 0.875rem;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .rule-item {
    background: #16213e;
    border-radius: 8px;
    padding: 1rem;
    margin-bottom: 0.75rem;
    transition: opacity 0.2s;
  }

  .rule-item.inactive {
    opacity: 0.5;
  }

  .rule-header {
    margin-bottom: 0.5rem;
  }

  .rule-name {
    font-weight: 600;
    color: #e94560;
  }

  .rule-details {
    display: flex;
    gap: 1.5rem;
    font-size: 0.875rem;
    color: #ccc;
  }

  .rule-days {
    color: #aaa;
  }

  .rule-time {
    font-family: monospace;
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
    display: block;
    margin-top: 0.5rem;
    background: #16213e;
    padding: 0.5rem;
    border-radius: 4px;
    font-family: monospace;
    color: #e94560;
    font-size: 0.75rem;
  }
</style>
