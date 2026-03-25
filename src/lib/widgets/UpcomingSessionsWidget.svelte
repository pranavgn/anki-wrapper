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

  function formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    const today = new Date();
    const tomorrow = new Date(today);
    tomorrow.setDate(tomorrow.getDate() + 1);

    if (dateStr === today.toISOString().split('T')[0]) {
      return 'Today';
    } else if (dateStr === tomorrow.toISOString().split('T')[0]) {
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
    // This would need to be connected to the actual study flow
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
        <div class="session-item p-3 rounded-xl" style="background: var(--bg-subtle);">
          <div class="flex items-center justify-between mb-2">
            <div class="flex items-center gap-2">
              <div class="w-2 h-2 rounded-full" style="background: var(--accent);"></div>
              <span style="font-family: var(--sans); font-size: 13px; font-weight: 600; color: var(--text-primary);">
                {formatDate(session.date)}
              </span>
            </div>
            <span style="font-family: var(--sans); font-size: 12px; color: var(--text-secondary);">
              {formatTime(session.time)}
            </span>
          </div>
          
          <div class="flex items-center justify-between">
            <div>
              <p style="font-family: var(--sans); font-size: 14px; color: var(--text-primary); font-weight: 500;">
                {session.deck_name || 'All Decks'}
              </p>
              {#if session.card_goal}
                <p style="font-family: var(--sans); font-size: 12px; color: var(--text-muted);">
                  Goal: {session.card_goal} cards
                </p>
              {/if}
              {#if session.note}
                <p style="font-family: var(--sans); font-size: 12px; color: var(--text-secondary); margin-top: 4px;">
                  {session.note}
                </p>
              {/if}
            </div>
            
            <button
              onclick={() => handleStartSession(session)}
              class="neu-btn px-3 py-1.5 rounded-lg cursor-pointer"
              style="background: var(--accent); color: white; font-family: var(--sans); font-size: 12px; font-weight: 500; border: none;"
            >
              Start
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .upcoming-sessions-widget {
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .sessions-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
    scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
  }

  .sessions-list::-webkit-scrollbar {
    width: 3px;
  }
  .sessions-list::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 2px;
  }

  .loading-spinner {
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

  .session-item {
    transition: background 0.2s ease;
  }

  .session-item:hover {
    background: var(--bg-card);
  }
</style>
