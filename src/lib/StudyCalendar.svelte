<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { addToast } from "./toast";
  import NeuSelect from "./ui/NeuSelect.svelte";
  import {
    loadSessions,
    saveSession,
    deleteSession,
    markCompleted,
    createSession,
    sessionsForDate,
    type StudySession,
  } from "./studySchedule";

  interface Props {
    heatmapData: Record<string, number>;
    decks: Array<{ id: number; name: string }>;
  }

  let { heatmapData, decks }: Props = $props();

  // Calendar state
  let calMonth = $state(new Date().getMonth());
  let calYear = $state(new Date().getFullYear());
  let selectedDate = $state<string | null>(null);
  let sessions = $state<StudySession[]>([]);
  let showAddForm = $state(false);

  // Form state for new session
  let formTime = $state("09:00");
  let formDuration = $state(30);
  let formDeckId = $state<number | null>(null);
  let formCardGoal = $state<number | null>(null);
  let formNote = $state("");
  let formNotify = $state(true);
  let isSaving = $state(false);

  const todayStr = new Date().toISOString().split("T")[0];
  const monthTitle = $derived(
    new Date(calYear, calMonth).toLocaleDateString("en-US", { month: "long", year: "numeric" })
  );

  // Fixed 42-cell grid — no relayout on month change
  let calendarCells = $derived.by(() => {
    const firstDay = new Date(calYear, calMonth, 1).getDay();
    const daysInMonth = new Date(calYear, calMonth + 1, 0).getDate();
    const cells: (number | null)[] = [];
    for (let i = 0; i < firstDay; i++) cells.push(null);
    for (let d = 1; d <= daysInMonth; d++) cells.push(d);
    while (cells.length < 42) cells.push(null);
    return cells;
  });

  // Sessions for the selected date
  let selectedSessions = $derived(
    selectedDate ? sessionsForDate(sessions, selectedDate) : []
  );

  // Sessions grouped by date for dot rendering
  let sessionDates = $derived(new Set(sessions.map((s) => s.date)));

  // Load sessions on mount
  $effect(() => {
    loadSessions()
      .then((s) => (sessions = s))
      .catch((e) => console.error("Failed to load sessions:", e));
  });

  function changeMonth(dir: number) {
    let m = calMonth + dir;
    if (m < 0) { m = 11; calYear--; }
    else if (m > 11) { m = 0; calYear++; }
    calMonth = m;
  }

  function selectDate(day: number) {
    const dateStr = `${calYear}-${String(calMonth + 1).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
    selectedDate = selectedDate === dateStr ? null : dateStr;
    showAddForm = false;
  }

  function openAddForm() {
    formTime = "09:00";
    formDuration = 30;
    formDeckId = null;
    formCardGoal = null;
    formNote = "";
    formNotify = true;
    showAddForm = true;
  }

  async function handleSaveSession() {
    if (!selectedDate) return;
    isSaving = true;
    try {
      const deckName = formDeckId
        ? decks.find((d) => d.id === formDeckId)?.name || null
        : null;

      const session = createSession({
        date: selectedDate,
        time: formTime,
        duration_mins: formDuration,
        deck_id: formDeckId,
        deck_name: deckName,
        card_goal: formCardGoal,
        note: formNote,
        notify: formNotify,
      });

      await saveSession(session);
      sessions = await loadSessions();
      showAddForm = false;
      addToast("Session scheduled", "success");
    } catch (e) {
      addToast(`Failed to save: ${e}`, "error");
    } finally {
      isSaving = false;
    }
  }

  async function handleDeleteSession(id: string) {
    try {
      await deleteSession(id);
      sessions = await loadSessions();
      addToast("Session removed", "success");
    } catch (e) {
      addToast(`Failed to delete: ${e}`, "error");
    }
  }

  async function handleToggleCompleted(id: string, completed: boolean) {
    try {
      await markCompleted(id, !completed);
      sessions = await loadSessions();
    } catch (e) {
      addToast(`Failed to update: ${e}`, "error");
    }
  }

  function formatDuration(mins: number): string {
    if (mins < 60) return `${mins}m`;
    const h = Math.floor(mins / 60);
    const m = mins % 60;
    return m > 0 ? `${h}h ${m}m` : `${h}h`;
  }
</script>

<div class="study-calendar">
  <!-- Calendar header -->
  <div class="cal-header">
    <button class="cal-nav neu-subtle" onclick={() => changeMonth(-1)}>‹</button>
    <span class="cal-title">{monthTitle}</span>
    <button class="cal-nav neu-subtle" onclick={() => changeMonth(1)}>›</button>
  </div>

  <!-- Day labels -->
  <div class="cal-grid">
    {#each ["S", "M", "T", "W", "T", "F", "S"] as dayLabel}
      <span class="cal-day-label">{dayLabel}</span>
    {/each}

    <!-- Cells -->
    {#each calendarCells as day, i}
      {#if day === null}
        <div class="cal-cell cal-empty"></div>
      {:else}
        {@const dateStr = `${calYear}-${String(calMonth + 1).padStart(2, "0")}-${String(day).padStart(2, "0")}`}
        {@const reviewCount = heatmapData[dateStr] || 0}
        {@const hasSession = sessionDates.has(dateStr)}
        {@const isToday = dateStr === todayStr}
        {@const isSelected = dateStr === selectedDate}
        <button
          class="cal-cell"
          class:cal-today={isToday}
          class:cal-selected={isSelected}
          class:cal-has-reviews={reviewCount > 0 && !isToday}
          onclick={() => selectDate(day)}
          title={reviewCount > 0 ? `${reviewCount} reviews` : hasSession ? "Has scheduled session" : ""}
        >
          <span class="cal-day-num">{day}</span>
          <div class="cal-indicators">
            {#if reviewCount > 0 && !isToday}
              <div class="cal-dot cal-dot-review" style="opacity: {Math.min(reviewCount / 25, 1) * 0.55 + 0.3}"></div>
            {/if}
            {#if hasSession}
              <div class="cal-dot cal-dot-session"></div>
            {/if}
          </div>
        </button>
      {/if}
    {/each}
  </div>

  <!-- Selected date panel -->
  {#if selectedDate}
    <div class="cal-detail-panel">
      <div class="cal-detail-header">
        <span class="cal-detail-date">
          {new Date(selectedDate + "T12:00:00").toLocaleDateString("en-US", {
            weekday: "short", month: "short", day: "numeric",
          })}
        </span>
        <div class="cal-detail-badges">
          {#if heatmapData[selectedDate]}
            <span class="cal-badge cal-badge-reviews">{heatmapData[selectedDate]} reviews</span>
          {/if}
          <button class="cal-add-btn" onclick={openAddForm} title="Schedule a session">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
              <path d="M12 5v14M5 12h14" />
            </svg>
          </button>
        </div>
      </div>

      <!-- Existing sessions for this date -->
      {#each selectedSessions as session}
        <div class="cal-session" class:cal-session-done={session.completed}>
          <button
            class="cal-check"
            class:cal-check-done={session.completed}
            onclick={() => handleToggleCompleted(session.id, session.completed)}
            title={session.completed ? "Mark incomplete" : "Mark complete"}
          >
            {#if session.completed}
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round"><path d="M5 13l4 4L19 7" /></svg>
            {/if}
          </button>
          <div class="cal-session-info">
            <span class="cal-session-time">{session.time}</span>
            <span class="cal-session-label">
              {session.deck_name || "All decks"}
              {#if session.card_goal} · {session.card_goal} cards{/if}
              · {formatDuration(session.duration_mins)}
            </span>
            {#if session.note}
              <span class="cal-session-note">{session.note}</span>
            {/if}
          </div>
          <button class="cal-delete-btn" onclick={() => handleDeleteSession(session.id)} title="Delete">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M18 6L6 18M6 6l12 12" /></svg>
          </button>
        </div>
      {/each}

      <!-- Add session form -->
      {#if showAddForm}
        <div class="cal-add-form">
           <div class="cal-form-row">
             <label for="form-time" class="cal-form-label">Time</label>
             <input id="form-time" type="time" class="cal-form-input neu-pressed" bind:value={formTime} />
           </div>

           <div class="cal-form-row">
             <label for="form-duration" class="cal-form-label">Duration</label>
             <NeuSelect
               id="form-duration"
               options={[
                 { value: 15, label: '15 min' },
                 { value: 30, label: '30 min' },
                 { value: 45, label: '45 min' },
                 { value: 60, label: '1 hour' },
                 { value: 90, label: '1.5 hours' },
                 { value: 120, label: '2 hours' }
               ]}
               bind:value={formDuration}
               size="sm"
             />
           </div>

          <div class="cal-form-row">
            <span class="cal-form-label">Deck</span>
            <NeuSelect
              options={[
                { value: null, label: 'All decks' },
                ...decks.map(d => ({ value: d.id, label: d.name }))
              ]}
              bind:value={formDeckId}
              size="sm"
              searchable={true}
            />
          </div>

          <div class="cal-form-row">
            <label for="form-card-goal" class="cal-form-label">Card goal</label>
            <input
              id="form-card-goal"
              type="number"
              class="cal-form-input neu-pressed"
              placeholder="Optional"
              min="1"
              max="999"
              bind:value={formCardGoal}
            />
          </div>

          <div class="cal-form-row">
            <label for="form-note" class="cal-form-label">Note</label>
            <input
              id="form-note"
              type="text"
              class="cal-form-input neu-pressed"
              placeholder="e.g. Focus on kanji"
              bind:value={formNote}
            />
          </div>

          <div class="cal-form-row">
            <label for="form-notify" class="cal-form-label">Notify me</label>
            <input id="form-notify" type="checkbox" bind:checked={formNotify} />
          </div>

          <div class="cal-form-actions">
            <button class="neu-subtle neu-btn cal-form-cancel" onclick={() => showAddForm = false}>Cancel</button>
            <button class="cal-form-save" onclick={handleSaveSession} disabled={isSaving}>
              {isSaving ? "Saving…" : "Schedule"}
            </button>
          </div>
        </div>
      {/if}

      {#if selectedSessions.length === 0 && !showAddForm}
        <p class="cal-empty-msg">No sessions scheduled. Click + to add one.</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .study-calendar {
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .cal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }

  .cal-nav {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 15px;
    color: var(--text-secondary);
    padding: 1px 7px;
    border-radius: 5px;
    font-family: var(--sans);
    line-height: 1;
  }

  .cal-nav:hover {
    background: var(--bg-subtle);
  }

  .cal-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    font-family: var(--sans);
  }

  .cal-grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 1px;
    background: var(--bg-subtle);
    border-radius: 8px;
    padding: 4px;
  }

  .cal-day-label {
    font-size: 10px;
    color: var(--text-secondary);
    text-align: center;
    padding: 2px 0;
    font-family: var(--sans);
  }

  .cal-cell {
    height: 32px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    border: none;
    background: transparent;
    cursor: pointer;
    padding: 0;
    font-family: var(--sans);
    transition: background 0.1s;
    position: relative;
  }

  .cal-cell:hover:not(.cal-empty) {
    background: var(--accent-soft, rgba(196, 113, 79, 0.06));
  }

  .cal-empty {
    cursor: default;
  }

  .cal-day-num {
    font-size: 11px;
    font-weight: 400;
    color: var(--text-secondary);
    line-height: 1;
    font-variant-numeric: tabular-nums;
  }

  .cal-has-reviews .cal-day-num {
    color: var(--text-primary);
  }

  .cal-today {
    background: var(--accent) !important;
    border-radius: 6px;
  }

  .cal-today .cal-day-num {
    color: #fff !important;
    font-weight: 700;
  }

  .cal-selected {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }

  .cal-indicators {
    display: flex;
    gap: 2px;
    margin-top: 1px;
    height: 4px;
  }

  .cal-dot {
    width: 4px;
    height: 4px;
    border-radius: 50%;
  }

  .cal-dot-review {
    background: var(--accent);
  }

  .cal-dot-session {
    background: var(--accent-secondary, #3B82F6);
  }

  .cal-today .cal-dot-review,
  .cal-today .cal-dot-session {
    background: rgba(255, 255, 255, 0.7);
  }

  /* ── Detail panel ── */

  .cal-detail-panel {
    margin-top: 10px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .cal-detail-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .cal-detail-date {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
    font-family: var(--sans);
  }

  .cal-detail-badges {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .cal-badge {
    font-size: 10px;
    font-weight: 600;
    padding: 2px 7px;
    border-radius: 4px;
    font-family: var(--sans);
  }

  .cal-badge-reviews {
    background: var(--accent-soft, rgba(196, 113, 79, 0.08));
    color: var(--accent);
  }

  .cal-add-btn {
    width: 24px;
    height: 24px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg-card);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    transition: all 0.1s;
  }

  .cal-add-btn:hover {
    background: var(--accent-soft, rgba(196, 113, 79, 0.08));
    color: var(--accent);
    border-color: var(--accent);
  }

  /* ── Session cards ── */

  .cal-session {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 8px;
    background: var(--bg-base);
    border: 1px solid var(--border);
  }

  .cal-session-done {
    opacity: 0.55;
  }

  .cal-check {
    width: 18px;
    height: 18px;
    border-radius: 4px;
    border: 1.5px solid var(--border);
    background: var(--bg-card);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    margin-top: 1px;
    transition: all 0.1s;
    padding: 0;
  }

  .cal-check:hover {
    border-color: var(--accent);
  }

  .cal-check-done {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  .cal-session-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .cal-session-time {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
    font-family: var(--sans);
  }

  .cal-session-label {
    font-size: 10px;
    color: var(--text-secondary);
    font-family: var(--sans);
  }

  .cal-session-note {
    font-size: 10px;
    color: var(--text-secondary);
    font-style: italic;
    font-family: var(--sans);
  }

  .cal-delete-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-muted);
    padding: 2px;
    border-radius: 4px;
    opacity: 0;
    transition: opacity 0.1s, color 0.1s;
  }

  .cal-session:hover .cal-delete-btn {
    opacity: 1;
  }

  .cal-delete-btn:hover {
    color: var(--danger, #EF4444);
  }

  /* ── Add form ── */

  .cal-add-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    border-radius: 8px;
    background: var(--bg-base);
    border: 1px solid var(--border);
  }

  .cal-form-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }

  .cal-form-label {
    font-size: 11px;
    color: var(--text-secondary);
    font-family: var(--sans);
    white-space: nowrap;
  }

  .cal-form-input {
    flex: 1;
    max-width: 140px;
    padding: 4px 8px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg-card);
    color: var(--text-primary);
    font-size: 12px;
    font-family: var(--sans);
  }

  .cal-form-input:focus {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }

  .cal-form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    margin-top: 4px;
  }

  .cal-form-cancel {
    padding: 5px 12px;
    border-radius: 6px;
    font-size: 11px;
    cursor: pointer;
  }

  .cal-form-save {
    padding: 5px 14px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    background: var(--accent);
    color: #fff;
    border: none;
  }

  .cal-form-save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .cal-empty-msg {
    font-size: 11px;
    color: var(--text-secondary);
    text-align: center;
    padding: 8px 0;
    margin: 0;
    font-family: var(--sans);
  }
</style>
