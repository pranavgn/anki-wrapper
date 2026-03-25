<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { addToast } from "../toast";
  import {
    loadSessions,
    saveSession,
    deleteSession,
    markCompleted,
    createSession,
    upcomingSessions,
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

  // Form state for new session
  let formTime = $state("09:00");
  let formDuration = $state(30);
  let formDeckId = $state<number | null>(null);
  let formCardGoal = $state<number | null>(null);
  let formNote = $state("");
  let formNotify = $state(true);
  let isSaving = $state(false);

  // New granularity and recurrence fields
  let scheduleType = $state<'exact' | 'hour' | 'day'>('exact');
  let formHour = $state(9);
  let recurrence = $state<'none' | 'weekly' | 'daily'>('none');
  let recurDays = $state<number[]>([]);

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

  // Sessions grouped by date for dot rendering
  let sessionDates = $derived(new Set(sessions.map((s) => s.date)));

  // Upcoming sessions for next 7 days
  let upcomingSessionsList = $derived.by(() => {
    const upcoming = upcomingSessions(sessions);
    const today = new Date();
    const nextWeek = new Date(today);
    nextWeek.setDate(nextWeek.getDate() + 7);
    
    return upcoming.filter(s => {
      const sessionDate = new Date(s.date);
      return sessionDate >= today && sessionDate <= nextWeek;
    }).slice(0, 5);
  });

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
      const deckName = formDeckId
        ? decks.find((d) => d.id === formDeckId)?.name || null
        : null;

      // Determine time based on scheduleType
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
        deck_name: deckName,
        card_goal: formCardGoal,
        note: formNote,
        notify: formNotify,
        schedule_type: scheduleType,
        recurrence: recurrence,
        recur_days: recurrence === 'weekly' ? recurDays : undefined,
      });

      await saveSession(session);

      // Handle recurring sessions
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
        // Generate for each day
        for (let day = 1; day <= 7; day++) {
          const newDate = new Date(baseDate);
          newDate.setDate(newDate.getDate() + (week * 7) + day - 1);
          
          const recurringSession = createSession({
            ...baseSession,
            id: `ss_${Date.now()}_${week}_${day}`,
            date: newDate.toISOString().split("T")[0],
            base_session_id: baseSession.id,
          });
          
          await saveSession(recurringSession);
        }
      } else if (recurrence === 'weekly' && recurDays.length > 0) {
        // Generate for specific days of the week
        for (const dayOfWeek of recurDays) {
          const newDate = new Date(baseDate);
          const daysUntilNext = (dayOfWeek - newDate.getDay() + 7) % 7;
          newDate.setDate(newDate.getDate() + (week * 7) + daysUntilNext);
          
          const recurringSession = createSession({
            ...baseSession,
            id: `ss_${Date.now()}_${week}_${dayOfWeek}`,
            date: newDate.toISOString().split("T")[0],
            base_session_id: baseSession.id,
          });
          
          await saveSession(recurringSession);
        }
      }
    }
  }

  async function handleDeleteSession(id: string) {
    try {
      // Delete recurring sessions if this is a base session
      const sessionToDelete = sessions.find(s => s.id === id);
      if (sessionToDelete?.base_session_id) {
        // Delete all future instances of this recurring session
        const futureSessions = sessions.filter(s => 
          s.base_session_id === sessionToDelete.base_session_id && 
          s.date >= sessionToDelete.date
        );
        for (const s of futureSessions) {
          await deleteSession(s.id);
        }
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

  function formatDate(dateStr: string): string {
    const date = new Date(dateStr + "T12:00:00");
    const today = new Date();
    const tomorrow = new Date(today);
    tomorrow.setDate(tomorrow.getDate() + 1);

    if (dateStr === today.toISOString().split("T")[0]) {
      return 'Today';
    } else if (dateStr === tomorrow.toISOString().split("T")[0]) {
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
</script>

<div class="study-schedule-widget">
  <!-- Mini Calendar -->
  <div class="mini-calendar">
    <div class="cal-header">
      <button class="cal-nav neu-subtle" onclick={() => changeMonth(-1)}>‹</button>
      <span class="cal-title">{monthTitle}</span>
      <button class="cal-nav neu-subtle" onclick={() => changeMonth(1)}>›</button>
    </div>

    <div class="cal-grid">
      {#each ["S", "M", "T", "W", "T", "F", "S"] as dayLabel}
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
            <div class="cal-indicators">
              {#if hasSession}
                <div class="cal-dot cal-dot-session"></div>
              {/if}
            </div>
          </button>
        {/if}
      {/each}
    </div>
  </div>

  <!-- Upcoming Sessions -->
  <div class="upcoming-section">
    <div class="section-header">
      <span class="section-title">Upcoming</span>
      <button class="add-btn neu-subtle" onclick={openAddForm} title="Schedule a session">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
          <path d="M12 5v14M5 12h14" />
        </svg>
        <span>Schedule</span>
      </button>
    </div>

    {#if upcomingSessionsList.length === 0}
      <p class="empty-msg">No upcoming sessions</p>
    {:else}
      <div class="session-list">
        {#each upcomingSessionsList as session (session.id)}
          <div class="session-item" class:session-done={session.completed}>
            <button
              class="session-check"
              class:session-check-done={session.completed}
              onclick={() => handleToggleCompleted(session.id, session.completed)}
              title={session.completed ? "Mark incomplete" : "Mark complete"}
            >
              {#if session.completed}
                <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round"><path d="M5 13l4 4L19 7" /></svg>
              {/if}
            </button>
            <div class="session-info">
              <div class="session-header">
                <span class="session-date">{formatDate(session.date)}</span>
                <span class="session-time">{formatTime(session.time)}</span>
              </div>
              <span class="session-label">
                {session.deck_name || deckName || "All decks"}
                {#if session.card_goal} · {session.card_goal} cards{/if}
                · {formatDuration(session.duration_mins)}
              </span>
              {#if session.note}
                <span class="session-note">{session.note}</span>
              {/if}
            </div>
            <button class="session-delete" onclick={() => handleDeleteSession(session.id)} title="Delete">
              <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M18 6L6 18M6 6l12 12" /></svg>
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Add Session Form -->
  {#if showAddForm}
    <div class="add-form">
      <div class="cal-form-row">
        <label class="cal-form-label">Schedule type</label>
        <select class="cal-form-input neu-pressed" bind:value={scheduleType}>
          <option value="exact">Exact time (e.g. 2:30 PM)</option>
          <option value="hour">Hour block (e.g. 2 PM - 3 PM)</option>
          <option value="day">Full day</option>
        </select>
      </div>

      {#if scheduleType === 'exact'}
        <div class="cal-form-row">
          <label class="cal-form-label">Time</label>
          <input type="time" class="cal-form-input neu-pressed" bind:value={formTime} />
        </div>
      {:else if scheduleType === 'hour'}
        <div class="cal-form-row">
          <label class="cal-form-label">Hour</label>
          <select class="cal-form-input neu-pressed" bind:value={formHour}>
            {#each Array.from({length: 24}, (_, i) => i) as h}
              <option value={h}>{h === 0 ? '12 AM' : h < 12 ? h + ' AM' : h === 12 ? '12 PM' : (h-12) + ' PM'}</option>
            {/each}
          </select>
        </div>
      {/if}

      <div class="cal-form-row">
        <label class="cal-form-label">Duration</label>
        <select class="cal-form-input neu-pressed" bind:value={formDuration}>
          <option value={15}>15 min</option>
          <option value={30}>30 min</option>
          <option value={45}>45 min</option>
          <option value={60}>1 hour</option>
          <option value={90}>1.5 hours</option>
          <option value={120}>2 hours</option>
        </select>
      </div>

      <div class="cal-form-row">
        <label class="cal-form-label">Deck</label>
        <select class="cal-form-input neu-pressed" bind:value={formDeckId}>
          <option value={null}>All decks</option>
          {#each decks as deck}
            <option value={deck.id}>{deck.name}</option>
          {/each}
        </select>
      </div>

      <div class="cal-form-row">
        <label class="cal-form-label">Card goal</label>
        <input
          type="number"
          class="cal-form-input neu-pressed"
          placeholder="Optional"
          min="1"
          max="999"
          bind:value={formCardGoal}
        />
      </div>

      <div class="cal-form-row">
        <label class="cal-form-label">Note</label>
        <input
          type="text"
          class="cal-form-input neu-pressed"
          placeholder="e.g. Focus on kanji"
          bind:value={formNote}
        />
      </div>

      <div class="cal-form-row">
        <label class="cal-form-label">Repeat</label>
        <select class="cal-form-input neu-pressed" bind:value={recurrence}>
          <option value="none">One-time</option>
          <option value="weekly">Weekly</option>
          <option value="daily">Daily</option>
        </select>
      </div>

      {#if recurrence === 'weekly'}
        <div class="cal-form-row">
          <label class="cal-form-label">On days</label>
          <div style="display: flex; gap: 4px;">
            {#each ['S','M','T','W','T','F','S'] as day, i}
              <button
                type="button"
                class="neu-subtle"
                style="width: 28px; height: 28px; border-radius: 50%; font-size: 11px; cursor: pointer;
                  {recurDays.includes(i) ? 'background: var(--accent); color: white;' : 'color: var(--text-secondary);'}"
                onclick={() => toggleRecurDay(i)}
              >{day}</button>
            {/each}
          </div>
        </div>
      {/if}

      <div class="cal-form-row">
        <label class="cal-form-label">Notify me</label>
        <input type="checkbox" bind:checked={formNotify} />
      </div>

      <div class="cal-form-actions">
        <button class="neu-subtle neu-btn cal-form-cancel" onclick={() => showAddForm = false}>Cancel</button>
        <button class="cal-form-save" onclick={handleSaveSession} disabled={isSaving}>
          {isSaving ? "Saving…" : "Schedule"}
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .study-schedule-widget {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  /* Mini Calendar */
  .mini-calendar {
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
  }

  .cal-day-label {
    font-size: 9px;
    color: var(--text-muted);
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
    color: var(--text-muted);
    line-height: 1;
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
    width: 3px;
    height: 3px;
    border-radius: 50%;
  }

  .cal-dot-session {
    background: var(--accent-secondary, #3B82F6);
  }

  .cal-today .cal-dot-session {
    background: rgba(255, 255, 255, 0.7);
  }

  /* Upcoming Section */
  .upcoming-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .section-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
    font-family: var(--sans);
  }

  .add-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    font-size: 11px;
    font-family: var(--sans);
    color: var(--accent);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.1s;
  }

  .add-btn:hover {
    background: var(--accent-soft, rgba(196, 113, 79, 0.08));
  }

  .empty-msg {
    font-size: 12px;
    color: var(--text-muted);
    text-align: center;
    padding: 12px 0;
    font-family: var(--sans);
  }

  .session-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .session-item {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px;
    background: var(--bg-subtle);
    border-radius: 8px;
    transition: background 0.1s;
  }

  .session-item:hover {
    background: var(--bg-card);
  }

  .session-done {
    opacity: 0.6;
  }

  .session-check {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    border: 1px solid var(--border);
    background: var(--bg-card);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    margin-top: 2px;
  }

  .session-check-done {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .session-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .session-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .session-date {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-primary);
    font-family: var(--sans);
  }

  .session-time {
    font-size: 10px;
    color: var(--text-secondary);
    font-family: var(--sans);
  }

  .session-label {
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

  .session-delete {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    border: none;
    background: transparent;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    flex-shrink: 0;
    margin-top: 2px;
  }

  .session-delete:hover {
    background: var(--danger-soft, rgba(192, 68, 74, 0.1));
    color: var(--danger);
  }

  /* Add Form */
  .add-form {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    background: var(--bg-subtle);
    border-radius: 8px;
    margin-top: 8px;
  }

  .cal-form-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .cal-form-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-primary);
    font-family: var(--sans);
  }

  .cal-form-input {
    padding: 6px 10px;
    border-radius: 6px;
    border: none;
    font-size: 12px;
    font-family: var(--sans);
    color: var(--text-primary);
    background: var(--bg-card);
  }

  .cal-form-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    margin-top: 4px;
  }

  .cal-form-cancel {
    padding: 6px 12px;
    font-size: 12px;
  }

  .cal-form-save {
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 600;
    color: white;
    background: var(--accent);
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-family: var(--sans);
  }

  .cal-form-save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
