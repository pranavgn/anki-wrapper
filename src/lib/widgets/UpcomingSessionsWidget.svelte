<script lang="ts">
  import { onMount } from "svelte";
  import { scheduleStore } from "../scheduleStore.svelte";
  import {
    SESSION_TYPE_THEMES,
    formatTime12h,
    formatDateLabel,
    formatDuration,
    type SessionType,
  } from "../types/scheduler";
  import FloatingScheduler from "./FloatingScheduler.svelte";

  let schedulerOpen = $state(false);

  onMount(() => { scheduleStore.init(); });

  /** Group upcoming sessions by date */
  let grouped = $derived.by(() => {
    const map = new Map<string, typeof scheduleStore.upcoming>();
    for (const s of scheduleStore.upcoming) {
      const arr = map.get(s.date) || [];
      arr.push(s);
      map.set(s.date, arr);
    }
    return Array.from(map.entries());
  });

  function getTypeTheme(session: { session_type?: string | null }) {
    const t = (session.session_type as SessionType) || 'review';
    return SESSION_TYPE_THEMES[t];
  }
</script>

<div class="upcoming-widget">
  <div class="upcoming-header">
    <div class="upcoming-schedule-wrapper">
      <button class="upcoming-schedule-btn" onclick={() => schedulerOpen = true}>
        + Schedule
      </button>
      <FloatingScheduler
        bind:open={schedulerOpen}
        onclose={() => schedulerOpen = false}
      />
    </div>
  </div>

  {#if scheduleStore.loading}
    <div class="upcoming-empty">
      <div class="upcoming-spinner"></div>
    </div>
  {:else if grouped.length === 0}
    <div class="upcoming-empty">
      <p class="upcoming-empty-text">No upcoming sessions</p>
    </div>
  {:else}
    <div class="upcoming-list">
      {#each grouped as [date, sessions]}
        <div class="upcoming-date-group">
          <div class="upcoming-date-label">{formatDateLabel(date)}</div>
          {#each sessions as session (session.id)}
            {@const theme = getTypeTheme(session)}
            <div class="session-row">
              <div class="session-accent" style="background: {theme.text}"></div>
              <div class="session-info">
                <span class="session-deck">{session.deck_name || 'All decks'}</span>
                <span class="session-meta">{formatTime12h(session.time)} · {formatDuration(session.duration_mins)}</span>
              </div>
              <span class="session-badge" style="background: {theme.bg}; color: {theme.text};">
                {theme.label}
              </span>
            </div>
          {/each}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .upcoming-widget {
    display: flex;
    flex-direction: column;
    height: 100%;
    gap: 8px;
  }

  .upcoming-header {
    display: flex;
    justify-content: flex-end;
  }

  .upcoming-schedule-wrapper {
    position: relative;
  }

  .upcoming-schedule-btn {
    padding: 6px 14px;
    font-size: 12px;
    font-weight: 500;
    font-family: var(--sans);
    background: var(--accent);
    color: #fff;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    box-shadow: 3px 3px 8px rgba(196,113,79,0.2), -2px -2px 6px rgba(255,255,255,0.1);
  }
  .upcoming-schedule-btn:hover { opacity: 0.92; }

  .upcoming-empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .upcoming-empty-text {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-secondary);
  }

  .upcoming-spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--bg-subtle);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .upcoming-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
    scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
  }

  .upcoming-list::-webkit-scrollbar {
    width: 3px;
  }
  .upcoming-list::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 2px;
  }

  .upcoming-date-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .upcoming-date-label {
    font-family: var(--sans);
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.03em;
    padding: 2px 0;
  }

  .session-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 8px;
    border-radius: var(--radius-sm);
    transition: background 0.1s;
    cursor: pointer;
  }
  .session-row:hover {
    background: var(--bg-subtle);
  }

  .session-accent {
    width: 3px;
    height: 24px;
    border-radius: 2px;
    flex-shrink: 0;
  }

  .session-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .session-deck {
    font-family: var(--sans);
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .session-meta {
    font-family: var(--sans);
    font-size: 11px;
    color: var(--text-secondary);
  }

  .session-badge {
    font-family: var(--sans);
    font-size: 11px;
    font-weight: 500;
    padding: 2px 8px;
    border-radius: 12px;
    white-space: nowrap;
  }
</style>
