<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { addToast } from "../toast";
  import NeuSelect from "../ui/NeuSelect.svelte";
  import {
    loadSessions,
    saveSession,
    deleteSession,
    markCompleted,
    createSession,
    type StudySession,
  } from "../studySchedule";

  interface Props {
    deckId?: number | null;
    deckName?: string | null;
  }

  let { deckId = null, deckName = null }: Props = $props();

  // Calendar state
  let calMonth = $state(new Date().getMonth());
  let calYear = $state(new Date().getFullYear());
  let selectedDate = $state<string | null>(null);
  let sessions = $state<StudySession[]>([]);
  let showAddForm = $state(false);
  let decks = $state<Array<{id: number; name: string}>>([]);

  // Form state for new session
  let formTime = $state("09:00");
  let formDuration = $state(30);
  let formDeckId = $state<number | null>(null);
  let formCardGoal = $state<number | null>(null);
  let formNote = $state("");
  let formNotify = $state(true);
  let isSaving = $state(false);

  let scheduleType = $state<'exact' | 'hour' | 'day'>('exact');
  let formHour = $state(9);
  let recurrence = $state<'none' | 'weekly' | 'daily'>('none');
  let recurDays = $state<number[]>([]);

  const todayStr = new Date().toISOString().split("T")[0];
  const monthTitle = $derived(
    new Date(calYear, calMonth).toLocaleDateString("en-US", { month: "long", year: "numeric" })
  );

  // Fixed 42-cell grid
  let calendarCells = $derived.by(() => {
    const firstDay = new Date(calYear, calMonth, 1).getDay();
    const daysInMonth = new Date(calYear, calMonth + 1, 0).getDate();
    const cells: (number | null)[] = [];
    for (let i = 0; i < firstDay; i++) cells.push(null);
    for (let d = 1; d <= daysInMonth; d++) cells.push(d);
    while (cells.length < 42) cells.push(null);
    return cells;
  });

  // Sessions grouped by date for dot rendering
  let sessionDates = $derived(new Set(sessions.map((s) => s.date)));

  // Sessions for the selected date
  let selectedSessions = $derived(
    selectedDate ? sessions.filter(s => s.date === selectedDate) : []
  );

  // Load sessions on mount (handled in onMount below)

  import { onMount } from "svelte";
  onMount(async () => {
    try {
      sessions = await loadSessions();
    } catch (e) {
      console.error("Failed to load sessions:", e);
    }
    try {
      const result = await invoke<Array<{id: number; name: string; short_name: string; level: number; new_count: number; learn_count: number; review_count: number; card_count: number; is_filtered: boolean}>>("get_all_decks");
      decks = result.map(d => ({ id: d.id, name: d.name }));
    } catch (e) {
      console.error("Failed to load decks for scheduler:", e);
    }
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
    formDeckId = deckId;
    formCardGoal = null;
    formNote = "";
    formNotify = true;
    scheduleType = 'exact';
    formHour = 9;
    recurrence = 'none';
    recurDays = [];
    showAddForm = true;
  }

  function toggleRecurDay(dayIndex: number) {
    if (recurDays.includes(dayIndex)) {
      recurDays = recurDays.filter(d => d !== dayIndex);
    } else {
      recurDays = [...recurDays, dayIndex];
    }
  }

  async function handleSaveSession() {
    if (!selectedDate) return;
    isSaving = true;
    try {
      const resolvedDeckName = formDeckId
        ? decks.find((d) => d.id === formDeckId)?.name || null
        : null;

      let sessionTime = formTime;
      if (scheduleType === 'hour') {
        sessionTime = `${String(formHour).padStart(2, "0")}:00`;
      } else if (scheduleType === 'day') {
        sessionTime = "00:00";
      }

      const session = createSession({
        date: selectedDate,
        time: sessionTime,
        duration_mins: formDuration,
        deck_id: formDeckId,
        deck_name: resolvedDeckName,
        card_goal: formCardGoal,
        note: formNote,
        notify: formNotify,
        schedule_type: scheduleType,
        recurrence: recurrence,
        recur_days: recurrence === 'weekly' ? recurDays : undefined,
      });

      await saveSession(session);

      if (recurrence !== 'none' && (recurDays.length > 0 || recurrence === 'daily')) {
        await generateRecurringSessions(session);
      }

      sessions = await loadSessions();
      showAddForm = false;
      addToast("Session scheduled", "success");
    } catch (e) {
      addToast(`Failed to save: ${e}`, "error");
    } finally {
      isSaving = false;
    }
  }

  async function generateRecurringSessions(baseSession: StudySession) {
    const weeksToGenerate = 4;
    const baseDate = new Date(baseSession.date);
    for (let week = 1; week <= weeksToGenerate; week++) {
      if (recurrence === 'daily') {
        for (let day = 1; day <= 7; day++) {
          const newDate = new Date(baseDate);
          newDate.setDate(newDate.getDate() + (week * 7) + day - 1);
          await saveSession(createSession({
            ...baseSession,
            id: `ss_${Date.now()}_${week}_${day}`,
            date: newDate.toISOString().split("T")[0],
            base_session_id: baseSession.id,
          }));
        }
      } else if (recurrence === 'weekly' && recurDays.length > 0) {
        for (const dayOfWeek of recurDays) {
          const newDate = new Date(baseDate);
          const daysUntilNext = (dayOfWeek - newDate.getDay() + 7) % 7;
          newDate.setDate(newDate.getDate() + (week * 7) + daysUntilNext);
          await saveSession(createSession({
            ...baseSession,
            id: `ss_${Date.now()}_${week}_${dayOfWeek}`,
            date: newDate.toISOString().split("T")[0],
            base_session_id: baseSession.id,
          }));
        }
      }
    }
  }

  async function handleDeleteSession(id: string) {
    try {
      const sessionToDelete = sessions.find(s => s.id === id);
      if (sessionToDelete?.base_session_id) {
        const futureSessions = sessions.filter(s =>
          s.base_session_id === sessionToDelete.base_session_id &&
          s.date >= sessionToDelete.date
        );
        for (const s of futureSessions) await deleteSession(s.id);
      } else {
        await deleteSession(id);
      }
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

  function formatSelectedDate(dateStr: string): string {
    const date = new Date(dateStr + "T12:00:00");
    const today = new Date();
    const tomorrow = new Date(today);
    tomorrow.setDate(tomorrow.getDate() + 1);
    if (dateStr === today.toISOString().split("T")[0]) return "Today";
    if (dateStr === tomorrow.toISOString().split("T")[0]) return "Tomorrow";
    return date.toLocaleDateString("en-US", { weekday: "short", month: "short", day: "numeric" });
  }

  function formatTime(timeStr: string): string {
    const [hours, minutes] = timeStr.split(':').map(Number);
    const period = hours >= 12 ? 'PM' : 'AM';
    const displayHours = hours % 12 || 12;
    return `${displayHours}:${String(minutes).padStart(2, '0')} ${period}`;
  }
</script>

<div class="schedule-layout">
  <!-- Left: Calendar -->
  <div class="cal-column">
    <div class="cal-header">
      <button class="cal-nav" onclick={() => changeMonth(-1)}>‹</button>
      <span class="cal-title">{monthTitle}</span>
      <button class="cal-nav" onclick={() => changeMonth(1)}>›</button>
    </div>

    <div class="cal-grid">
      {#each ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"] as dayLabel}
        <span class="cal-day-label">{dayLabel}</span>
      {/each}

      {#each calendarCells as day, i}
        {#if day === null}
          <div class="cal-cell cal-empty"></div>
        {:else}
          {@const dateStr = `${calYear}-${String(calMonth + 1).padStart(2, "0")}-${String(day).padStart(2, "0")}`}
          {@const hasSession = sessionDates.has(dateStr)}
          {@const isToday = dateStr === todayStr}
          {@const isSelected = dateStr === selectedDate}
          <button
            class="cal-cell"
            class:cal-today={isToday}
            class:cal-selected={isSelected}
            onclick={() => selectDate(day)}
            title={hasSession ? "Has scheduled session" : ""}
          >
            <span class="cal-day-num">{day}</span>
            {#if hasSession}
              <div class="cal-dot"></div>
            {/if}
          </button>
        {/if}
      {/each}
    </div>
  </div>

  <!-- Right: Scheduling panel -->
  <div class="schedule-column">
    {#if !selectedDate}
      <div class="schedule-placeholder">
        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" style="color: var(--text-muted); margin-bottom: 8px;">
          <rect x="3" y="4" width="18" height="18" rx="2" ry="2"/>
          <line x1="16" y1="2" x2="16" y2="6"/>
          <line x1="8" y1="2" x2="8" y2="6"/>
          <line x1="3" y1="10" x2="21" y2="10"/>
        </svg>
        <p class="placeholder-text">Select a date to schedule or view sessions</p>
      </div>
    {:else}
      <!-- Date header -->
      <div class="panel-header">
        <span class="panel-date">{formatSelectedDate(selectedDate)}</span>
        {#if !showAddForm}
          <button class="schedule-btn" onclick={openAddForm}>
            <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
              <path d="M12 5v14M5 12h14" />
            </svg>
            Schedule
          </button>
        {/if}
      </div>

      <!-- Sessions for this date -->
      <div class="panel-body">
        {#if selectedSessions.length > 0}
          <div class="session-list">
            {#each selectedSessions as session (session.id)}
              <div class="session-item" class:session-done={session.completed}>
                <button
                  class="session-check"
                  class:session-check-done={session.completed}
                  onclick={() => handleToggleCompleted(session.id, session.completed)}
                  title={session.completed ? "Mark incomplete" : "Mark complete"}
                >
                  {#if session.completed}
                    <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3.5" stroke-linecap="round"><path d="M5 13l4 4L19 7" /></svg>
                  {/if}
                </button>
                <div class="session-info">
                  <span class="session-time">{formatTime(session.time)}</span>
                  <span class="session-meta">
                    {session.deck_name || deckName || "All decks"}
                    {#if session.card_goal} · {session.card_goal} cards{/if}
                    · {formatDuration(session.duration_mins)}
                  </span>
                  {#if session.note}<span class="session-note">{session.note}</span>{/if}
                </div>
                <button class="session-delete" onclick={() => handleDeleteSession(session.id)} title="Delete">
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M18 6L6 18M6 6l12 12" /></svg>
                </button>
              </div>
            {/each}
          </div>
        {:else if !showAddForm}
          <p class="empty-msg">No sessions. Click Schedule to add one.</p>
        {/if}

        <!-- Add Session Form -->
        {#if showAddForm}
          <div class="add-form">
            <div class="form-row">
              <label class="form-label">Type</label>
              <NeuSelect
                options={[
                  { value: 'exact', label: 'Exact time' },
                  { value: 'hour', label: 'Hour block' },
                  { value: 'day', label: 'Full day' }
                ]}
                bind:value={scheduleType}
                size="sm"
              />
            </div>

            {#if scheduleType === 'exact'}
              <div class="form-row">
                <label class="form-label">Time</label>
                <input type="time" class="form-input" bind:value={formTime} />
              </div>
            {:else if scheduleType === 'hour'}
              <div class="form-row">
                <label class="form-label">Hour</label>
                <NeuSelect
                  options={Array.from({length: 24}, (_, i) => ({
                    value: i,
                    label: i === 0 ? '12 AM' : i < 12 ? i + ' AM' : i === 12 ? '12 PM' : (i-12) + ' PM'
                  }))}
                  bind:value={formHour}
                  size="sm"
                />
              </div>
            {/if}

            <div class="form-row">
              <label class="form-label">Duration</label>
              <NeuSelect
                options={[
                  { value: 15, label: '15 min' },
                  { value: 30, label: '30 min' },
                  { value: 45, label: '45 min' },
                  { value: 60, label: '1 hour' },
                  { value: 90, label: '1.5 hr' },
                  { value: 120, label: '2 hr' }
                ]}
                bind:value={formDuration}
                size="sm"
              />
            </div>

            <div class="form-row">
              <label class="form-label">Deck</label>
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

            <div class="form-row">
              <label class="form-label">Goal</label>
              <input
                type="number"
                class="form-input"
                placeholder="Cards (opt.)"
                min="1"
                max="999"
                bind:value={formCardGoal}
              />
            </div>

            <div class="form-row">
              <label class="form-label">Note</label>
              <input
                type="text"
                class="form-input"
                placeholder="Optional"
                bind:value={formNote}
              />
            </div>

            <div class="form-row">
              <label class="form-label">Repeat</label>
              <NeuSelect
                options={[
                  { value: 'none', label: 'Once' },
                  { value: 'weekly', label: 'Weekly' },
                  { value: 'daily', label: 'Daily' }
                ]}
                bind:value={recurrence}
                size="sm"
              />
            </div>

            {#if recurrence === 'weekly'}
              <div class="form-row">
                <span class="form-label">Days</span>
                <div style="display: flex; gap: 3px;">
                  {#each ['S','M','T','W','T','F','S'] as day, i}
                    <button
                      type="button"
                      class="day-pill"
                      class:day-pill-active={recurDays.includes(i)}
                      onclick={() => toggleRecurDay(i)}
                    >{day}</button>
                  {/each}
                </div>
              </div>
            {/if}

            <div class="form-actions">
              <button class="btn-cancel" onclick={() => showAddForm = false}>Cancel</button>
              <button class="btn-save" onclick={handleSaveSession} disabled={isSaving}>
                {isSaving ? "Saving…" : "Schedule"}
              </button>
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .schedule-layout {
    display: flex;
    gap: 16px;
    height: 100%;
    min-height: 0;
  }

  /* ── Left: Calendar ── */

  .cal-column {
    flex: 0 0 auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .cal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .cal-nav {
    background: var(--bg-subtle);
    border: 1px solid var(--border);
    cursor: pointer;
    font-size: 16px;
    color: var(--text-secondary);
    padding: 3px 12px;
    border-radius: 6px;
    font-family: var(--sans);
    line-height: 1.4;
    transition: background 0.1s, color 0.1s;
  }

  .cal-nav:hover {
    background: var(--bg-card);
    color: var(--text-primary);
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
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
  }

  .cal-day-label {
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    color: var(--text-secondary);
    text-align: center;
    padding: 5px 0 4px;
    font-family: var(--sans);
    background: var(--bg-subtle);
    border-bottom: 1px solid var(--border);
  }

  .cal-cell {
    height: 30px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    border: none;
    border-right: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    background: var(--bg-card);
    cursor: pointer;
    padding: 0;
    font-family: var(--sans);
    transition: background 0.1s;
    position: relative;
    gap: 1px;
  }

  /* Remove right border on last column */
  .cal-cell:nth-child(7n) {
    border-right: none;
  }

  /* Remove bottom border on last row items */
  .cal-cell:nth-last-child(-n+7) {
    border-bottom: none;
  }

  .cal-cell:hover:not(.cal-empty) {
    background: var(--accent-soft, rgba(196, 113, 79, 0.06));
  }

  .cal-empty {
    cursor: default;
    background: var(--bg-subtle);
  }

  .cal-day-num {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-primary);
    line-height: 1;
    font-variant-numeric: tabular-nums;
  }

  .cal-today {
    background: var(--accent) !important;
  }

  .cal-today .cal-day-num {
    color: #fff !important;
    font-weight: 700;
  }

  .cal-selected:not(.cal-today) {
    background: var(--accent-soft, rgba(196, 113, 79, 0.1));
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .cal-dot {
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--accent-secondary, #3B82F6);
  }

  .cal-today .cal-dot {
    background: rgba(255, 255, 255, 0.8);
  }

  /* ── Right: Schedule panel ── */

  .schedule-column {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .schedule-placeholder {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: 0;
  }

  .placeholder-text {
    font-family: var(--sans);
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.4;
    max-width: 140px;
    margin: 0;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 10px;
    flex-shrink: 0;
  }

  .panel-date {
    font-family: var(--sans);
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .schedule-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    font-size: 11px;
    font-weight: 600;
    font-family: var(--sans);
    color: white;
    background: var(--accent);
    border: none;
    border-radius: 6px;
    cursor: pointer;
    transition: opacity 0.1s;
  }

  .schedule-btn:hover {
    opacity: 0.88;
  }

  .panel-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
    scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
  }

  .session-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .session-item {
    display: flex;
    align-items: flex-start;
    gap: 7px;
    padding: 7px 9px;
    background: var(--bg-subtle);
    border-radius: 7px;
    border: 1px solid var(--border);
  }

  .session-done {
    opacity: 0.55;
  }

  .session-check {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    border: 1.5px solid var(--border);
    background: var(--bg-card);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    margin-top: 1px;
    padding: 0;
    transition: border-color 0.1s;
  }

  .session-check:hover { border-color: var(--accent); }

  .session-check-done {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  .session-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .session-time {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-primary);
    font-family: var(--sans);
  }

  .session-meta {
    font-size: 10px;
    color: var(--text-secondary);
    font-family: var(--sans);
  }

  .session-note {
    font-size: 10px;
    color: var(--text-muted);
    font-family: var(--sans);
    font-style: italic;
  }

  .session-delete {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-muted);
    padding: 2px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    opacity: 0;
    transition: opacity 0.1s, color 0.1s;
  }

  .session-item:hover .session-delete { opacity: 1; }
  .session-delete:hover { color: var(--danger, #EF4444); }

  .empty-msg {
    font-size: 11px;
    color: var(--text-muted);
    font-family: var(--sans);
    text-align: center;
    padding: 16px 0;
    margin: 0;
  }

  /* ── Add Form ── */

  .add-form {
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding: 10px;
    background: var(--bg-subtle);
    border: 1px solid var(--border);
    border-radius: 8px;
  }

  .form-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }

  .form-label {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary);
    font-family: var(--sans);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .form-input {
    flex: 1;
    padding: 4px 8px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg-card);
    color: var(--text-primary);
    font-size: 12px;
    font-family: var(--sans);
  }

  .form-input:focus {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }

  .day-pill {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: none;
    font-size: 10px;
    font-family: var(--sans);
    cursor: pointer;
    background: var(--bg-card);
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }

  .day-pill-active {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .form-actions {
    display: flex;
    gap: 6px;
    justify-content: flex-end;
    margin-top: 2px;
  }

  .btn-cancel {
    padding: 5px 11px;
    font-size: 11px;
    font-family: var(--sans);
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg-card);
    color: var(--text-secondary);
    cursor: pointer;
  }

  .btn-save {
    padding: 5px 13px;
    font-size: 11px;
    font-weight: 600;
    font-family: var(--sans);
    border-radius: 6px;
    border: none;
    background: var(--accent);
    color: white;
    cursor: pointer;
  }

  .btn-save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
