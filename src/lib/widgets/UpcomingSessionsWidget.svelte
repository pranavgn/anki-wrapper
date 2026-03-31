<script lang="ts">
  import { onMount } from "svelte";
  import { loadSessions, upcomingSessions, type StudySession } from "../studySchedule";
  import { addToast } from "../toast";

  let sessions: StudySession[] = $state([]);
  let isLoading = $state(true);

  onMount(async () => {
    await loadSessionsData();
  });

  async function loadSessionsData() {
    isLoading = true;
    try {
      sessions = await loadSessions();
    } catch (error) {
      console.error("Error loading sessions:", error);
      addToast("Failed to load study sessions", "error");
    } finally {
      isLoading = false;
    }
  }

  let upcoming = $derived.by(() => {
    return upcomingSessions(sessions).slice(0, 3);
  });

  // FIX: Use local date string, not UTC
  function getLocalDateStr(d: Date = new Date()): string {
    const year = d.getFullYear();
    const month = String(d.getMonth() + 1).padStart(2, "0");
    const day = String(d.getDate()).padStart(2, "0");
    return `${year}-${month}-${day}`;
  }

  function formatDate(dateStr: string): string {
    const date = new Date(dateStr + "T12:00:00"); // Avoid UTC shift
    const today = new Date();
    const tomorrow = new Date(today);
    tomorrow.setDate(tomorrow.getDate() + 1);

    if (dateStr === getLocalDateStr(today)) {
      return 'Today';
    } else if (dateStr === getLocalDateStr(tomorrow)) {
      return 'Tomorrow';
    } else {
      return date.toLocaleDateString('en-US', { weekday: 'short', month: 'short', day: 'numeric' });
    }
  }

  function formatTime(timeStr: string): string {
    const [hours, minutes] = timeStr.split(':').map(Number);
    const period = hours >= 12 ? 'PM' : 'AM';
    const displayHours = hours % 12 || 12;
    return `${displayHours}:${String(minutes).padStart(2, '0')} ${period}`;
  }

  async function handleStartSession(session: StudySession) {
    addToast(`Starting session: ${session.deck_name || 'All Decks'}`, "info");
  }
</script>

<div class="upcoming-sessions-widget" style="height: 100%; display: flex; flex-direction: column;">
  {#if isLoading}
    <div class="flex items-center justify-center" style="flex: 1;">
      <div class="loading-spinner"></div>
    </div>
  {:else if upcoming.length === 0}
    <div class="flex flex-col items-center justify-center" style="flex: 1;">
      <svg class="w-12 h-12 mx-auto mb-3" style="color: var(--text-muted);" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
      </svg>
      <p style="font-family: var(--sans); font-size: 14px; color: var(--text-secondary);">No upcoming sessions</p>
      <p style="font-family: var(--sans); font-size: 12px; color: var(--text-muted); margin-top: 4px;">Schedule a study session to get started</p>
    </div>
  {:else}
    <div class="sessions-list">
      {#each upcoming as session (session.id)}
        <div class="session-card">
          <div class="session-left">
            <div class="session-date-badge">{formatDate(session.date)}</div>
            <div class="session-time-text">{formatTime(session.time)}</div>
          </div>
          <div class="session-right">
            <div class="session-deck">{session.deck_name || 'All decks'}</div>
            {#if session.card_goal}
              <div class="session-goal">{session.card_goal} cards · {session.duration_mins}min</div>
            {:else}
              <div class="session-goal">{session.duration_mins}min</div>
            {/if}
            {#if session.note}
              <div class="session-note">{session.note}</div>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .sessions-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex: 1;
    overflow-y: auto;
  }

  .session-card {
    display: flex;
    gap: 12px;
    padding: 10px 12px;
    background: var(--bg-subtle);
    border-radius: 8px;
    align-items: flex-start;
  }

  .session-left {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    min-width: 56px;
  }

  .session-date-badge {
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    font-family: var(--sans);
  }

  .session-time-text {
    font-size: 10px;
    color: var(--text-secondary);
    font-family: var(--sans);
  }

  .session-right {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .session-deck {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    font-family: var(--sans);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .session-goal {
    font-size: 11px;
    color: var(--text-secondary);
    font-family: var(--sans);
  }

  .session-note {
    font-size: 10px;
    color: var(--text-muted);
    font-family: var(--sans);
    font-style: italic;
  }

  .loading-spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
