<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { addToast } from "../toast";
  import NeuSelect from "../ui/NeuSelect.svelte";
  import {
    loadSessions,
    saveSession,
    updateSession,
    deleteSession,
    deleteSessionWithRecurring,
    markCompleted,
    createSession,
    generateRecurringSessions,
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
  let decks = $state<Array<{ id: number; name: string }>>([]);

  // Form state — shared for add AND edit
  let formTime = $state("09:00");
  let formDuration = $state(30);
  let formDeckId = $state<number | null>(null);
  let formCardGoal = $state<number | null>(null);
  let formNote = $state("");
  let formNotify = $state(true);
  let isSaving = $state(false);

  // Schedule type & recurrence
  let scheduleType = $state<"exact" | "hour" | "day">("exact");
  let formHour = $state(9);
  let recurrence = $state<"none" | "weekly" | "daily">("none");
  let recurDays = $state<number[]>([]);

  // Edit mode
  let editingSession = $state<StudySession | null>(null);
  let isEditing = $derived(editingSession !== null);

  // Delete-recurring confirmation
  let deleteConfirmId = $state<string | null>(null);
  let deleteConfirmIsRecurring = $state(false);

  const todayStr = new Date().toISOString().split("T")[0];
  const monthTitle = $derived(
    new Date(calYear, calMonth).toLocaleDateString("en-US", {
      month: "long",
      year: "numeric",
    })
  );

  // 42-cell calendar grid
  let calendarCells = $derived.by(() => {
    const firstDay = new Date(calYear, calMonth, 1).getDay();
    const daysInMonth = new Date(calYear, calMonth + 1, 0).getDate();
    const cells: (number | null)[] = [];
    for (let i = 0; i < firstDay; i++) cells.push(null);
    for (let d = 1; d <= daysInMonth; d++) cells.push(d);
    while (cells.length < 42) cells.push(null);
    return cells;
  });

  let sessionDates = $derived(new Set(sessions.map((s) => s.date)));

  // Upcoming sessions for next 7 days
  let upcomingSessionsList = $derived.by(() => {
    const upcoming = upcomingSessions(sessions);
    const today = new Date();
    const nextWeek = new Date(today);
    nextWeek.setDate(nextWeek.getDate() + 7);

    return upcoming
      .filter((s) => {
        const sessionDate = new Date(s.date);
        return sessionDate >= today && sessionDate <= nextWeek;
      })
      .slice(0, 5);
  });

  // Load sessions on mount
  $effect(() => {
    loadSessions()
      .then((s) => (sessions = s))
      .catch((e) => console.error("Failed to load sessions:", e));
  });

  // Load decks on mount
  import { onMount } from "svelte";
  onMount(async () => {
    try {
      const result = await invoke<
        Array<{
          id: number;
          name: string;
          short_name: string;
          level: number;
          new_count: number;
          learn_count: number;
          review_count: number;
          card_count: number;
          is_filtered: boolean;
        }>
      >("get_all_decks");
      decks = result.map((d) => ({ id: d.id, name: d.name }));
    } catch (e) {
      console.error("Failed to load decks for scheduler:", e);
    }
  });

  function changeMonth(dir: number) {
    let m = calMonth + dir;
    if (m < 0) {
      m = 11;
      calYear--;
    } else if (m > 11) {
      m = 0;
      calYear++;
    }
    calMonth = m;
  }

  function selectDate(day: number) {
    const dateStr = `${calYear}-${String(calMonth + 1).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
    selectedDate = selectedDate === dateStr ? null : dateStr;
    showAddForm = false;
    editingSession = null;
    deleteConfirmId = null;
  }

  // ──────────── ADD ────────────

  function openAddForm() {
    editingSession = null;
    formTime = "09:00";
    formDuration = 30;
    formDeckId = deckId;
    formCardGoal = null;
    formNote = "";
    formNotify = true;
    scheduleType = "exact";
    formHour = 9;
    recurrence = "none";
    recurDays = [];
    showAddForm = true;
  }

  // ──────────── EDIT ────────────

  function openEditForm(session: StudySession) {
    editingSession = session;
    showAddForm = true;

    // Pre-populate form from existing session
    formTime = session.time;
    formDuration = session.duration_mins;
    formDeckId = session.deck_id;
    formCardGoal = session.card_goal;
    formNote = session.note;
    formNotify = session.notify;
    scheduleType = session.schedule_type ?? "exact";
    recurrence = session.recurrence ?? "none";
    recurDays = session.recur_days ? [...session.recur_days] : [];

    // Derive formHour from time for "hour" schedule type
    const [h] = session.time.split(":").map(Number);
    formHour = h;
  }

  function cancelForm() {
    showAddForm = false;
    editingSession = null;
  }

  // ──────────── SAVE (add or edit) ────────────

  async function handleSaveSession() {
    if (!selectedDate && !isEditing) return;
    isSaving = true;
    try {
      const resolvedDeckName = formDeckId
        ? decks.find((d) => d.id === formDeckId)?.name || null
        : null;

      // Determine time
      let sessionTime = formTime;
      if (scheduleType === "hour") {
        sessionTime = `${String(formHour).padStart(2, "0")}:00`;
      } else if (scheduleType === "day") {
        sessionTime = "00:00";
      }

      if (isEditing && editingSession) {
        // ── Update existing session ──
        const updated: StudySession = {
          ...editingSession,
          time: sessionTime,
          duration_mins: formDuration,
          deck_id: formDeckId,
          deck_name: resolvedDeckName,
          card_goal: formCardGoal,
          note: formNote,
          notify: formNotify,
          schedule_type: scheduleType,
          recurrence: recurrence,
          recur_days: recurrence === "weekly" ? recurDays : undefined,
        };

        // If recurrence settings changed, regenerate the series
        const recurrenceChanged =
          editingSession.recurrence !== recurrence ||
          JSON.stringify(editingSession.recur_days ?? []) !==
            JSON.stringify(recurDays);

        await updateSession(updated, recurrenceChanged);
        addToast("Session updated", "success");
      } else {
        // ── Create new session ──
        const session = createSession({
          date: selectedDate!,
          time: sessionTime,
          duration_mins: formDuration,
          deck_id: formDeckId,
          deck_name: resolvedDeckName,
          card_goal: formCardGoal,
          note: formNote,
          notify: formNotify,
          schedule_type: scheduleType,
          recurrence: recurrence,
          recur_days: recurrence === "weekly" ? recurDays : undefined,
        });

        await saveSession(session);

        if (
          recurrence !== "none" &&
          (recurDays.length > 0 || recurrence === "daily")
        ) {
          await generateRecurringSessions(session);
        }
        addToast("Session scheduled", "success");
      }

      sessions = await loadSessions();
      showAddForm = false;
      editingSession = null;
    } catch (e) {
      addToast(`Failed to save: ${e}`, "error");
    } finally {
      isSaving = false;
    }
  }

  // ──────────── DELETE ────────────

  function promptDelete(session: StudySession) {
    const isRecurring =
      !!session.base_session_id ||
      (session.recurrence && session.recurrence !== "none");
    if (isRecurring) {
      deleteConfirmId = session.id;
      deleteConfirmIsRecurring = true;
    } else {
      handleDeleteSession(session.id, false);
    }
  }

  async function handleDeleteSession(id: string, allFuture: boolean) {
    deleteConfirmId = null;
    try {
      await deleteSessionWithRecurring(id, allFuture);
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

  function toggleRecurDay(dayIndex: number) {
    if (recurDays.includes(dayIndex)) {
      recurDays = recurDays.filter((d) => d !== dayIndex);
    } else {
      recurDays = [...recurDays, dayIndex];
    }
  }

  // ──────────── FORMATTERS ────────────

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
      return "Today";
    } else if (dateStr === tomorrow.toISOString().split("T")[0]) {
      return "Tomorrow";
    } else {
      return date.toLocaleDateString("en-US", {
        weekday: "short",
        month: "short",
        day: "numeric",
      });
    }
  }

  function formatTime(timeStr: string): string {
    const [hours, minutes] = timeStr.split(":").map(Number);
    const period = hours >= 12 ? "PM" : "AM";
    const displayHours = hours % 12 || 12;
    return `${displayHours}:${String(minutes).padStart(2, "0")} ${period}`;
  }

  function recurrenceLabel(session: StudySession): string {
    if (!session.recurrence || session.recurrence === "none") return "";
    if (session.recurrence === "daily") return "Daily";
    if (session.recurrence === "weekly") {
      const dayNames = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
      const days = (session.recur_days ?? []).map((d) => dayNames[d]).join(", ");
      return days ? `Weekly: ${days}` : "Weekly";
    }
    return "";
  }
</script>

<div
  class="study-schedule-widget"
  style="height: 100%; display: flex; flex-direction: column;"
>
  <!-- Mini Calendar -->
  <div class="mini-calendar">
    <div class="cal-header">
      <button class="cal-nav neu-subtle" onclick={() => changeMonth(-1)}>
        ‹
      </button>
      <span class="cal-title">{monthTitle}</span>
      <button class="cal-nav neu-subtle" onclick={() => changeMonth(1)}>
        ›
      </button>
    </div>

    <div class="cal-grid">
      {#each ["S", "M", "T", "W", "T", "F", "S"] as dayLabel}
        <span class="cal-day-label">{dayLabel}</span>
      {/each}

      {#each calendarCells as day}
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
      <button
        class="add-btn neu-subtle"
        onclick={openAddForm}
        title="Schedule a session"
      >
        <svg
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.5"
          stroke-linecap="round"
        >
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
            <!-- Complete toggle -->
            <button
              class="session-check"
              class:session-check-done={session.completed}
              onclick={() =>
                handleToggleCompleted(session.id, session.completed)}
              title={session.completed ? "Mark incomplete" : "Mark complete"}
            >
              {#if session.completed}
                <svg
                  width="10"
                  height="10"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="3"
                  stroke-linecap="round"
                  ><path d="M5 13l4 4L19 7" /></svg
                >
              {/if}
            </button>

            <!-- Session info — click to edit -->
            <button
              class="session-info"
              onclick={() => {
                selectedDate = session.date;
                openEditForm(session);
              }}
              title="Click to edit"
            >
              <div class="session-header">
                <span class="session-date">{formatDate(session.date)}</span>
                <span class="session-time">{formatTime(session.time)}</span>
              </div>
              <span class="session-label">
                {session.deck_name || deckName || "All decks"}
                {#if session.card_goal} · {session.card_goal} cards{/if}
                · {formatDuration(session.duration_mins)}
              </span>
              {#if recurrenceLabel(session)}
                <span class="session-recurrence"
                  >{recurrenceLabel(session)}</span
                >
              {/if}
              {#if session.note}
                <span class="session-note">{session.note}</span>
              {/if}
            </button>

            <!-- Delete -->
            {#if deleteConfirmId === session.id}
              <div class="delete-confirm">
                <button
                  class="delete-opt"
                  onclick={() => handleDeleteSession(session.id, false)}
                  >This one</button
                >
                <button
                  class="delete-opt delete-opt-all"
                  onclick={() => handleDeleteSession(session.id, true)}
                  >All future</button
                >
                <button
                  class="delete-opt"
                  onclick={() => (deleteConfirmId = null)}>Cancel</button
                >
              </div>
            {:else}
              <button
                class="session-delete"
                onclick={() => promptDelete(session)}
                title="Delete"
              >
                <svg
                  width="10"
                  height="10"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  ><path d="M18 6L6 18M6 6l12 12" /></svg
                >
              </button>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Add / Edit Session Form -->
  {#if showAddForm}
    <div class="add-form">
      <div class="form-title">
        {isEditing ? "Edit Session" : "New Session"}
      </div>

      <div class="cal-form-row">
        <label for="schedule-type" class="cal-form-label">Schedule type</label>
        <NeuSelect
          id="schedule-type"
          options={[
            { value: "exact", label: "Exact time (e.g. 2:30 PM)" },
            { value: "hour", label: "Hour block (e.g. 2 PM - 3 PM)" },
            { value: "day", label: "Full day" },
          ]}
          bind:value={scheduleType}
          size="sm"
        />
      </div>

      {#if scheduleType === "exact"}
        <div class="cal-form-row">
          <label for="form-time" class="cal-form-label">Time</label>
          <input
            id="form-time"
            type="time"
            class="cal-form-input neu-pressed"
            bind:value={formTime}
          />
        </div>
      {:else if scheduleType === "hour"}
        <div class="cal-form-row">
          <label for="form-hour" class="cal-form-label">Hour</label>
          <NeuSelect
            id="form-hour"
            options={Array.from({ length: 24 }, (_, i) => ({
              value: i,
              label:
                i === 0
                  ? "12 AM"
                  : i < 12
                    ? i + " AM"
                    : i === 12
                      ? "12 PM"
                      : i - 12 + " PM",
            }))}
            bind:value={formHour}
            size="sm"
          />
        </div>
      {/if}

      <div class="cal-form-row">
        <label for="form-duration" class="cal-form-label">Duration</label>
        <NeuSelect
          id="form-duration"
          options={[
            { value: 15, label: "15 min" },
            { value: 30, label: "30 min" },
            { value: 45, label: "45 min" },
            { value: 60, label: "1 hour" },
            { value: 90, label: "1.5 hours" },
            { value: 120, label: "2 hours" },
          ]}
          bind:value={formDuration}
          size="sm"
        />
      </div>

      <div class="cal-form-row">
        <label for="form-deck" class="cal-form-label">Deck</label>
        <NeuSelect
          id="form-deck"
          options={[
            { value: null, label: "All decks" },
            ...decks.map((d) => ({ value: d.id, label: d.name })),
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
        <label for="form-recurrence" class="cal-form-label">Repeat</label>
        <NeuSelect
          id="form-recurrence"
          options={[
            { value: "none", label: "One-time" },
            { value: "weekly", label: "Weekly" },
            { value: "daily", label: "Daily" },
          ]}
          bind:value={recurrence}
          size="sm"
        />
      </div>

      {#if recurrence === "weekly"}
        <div class="cal-form-row">
          <span id="recur-days-label" class="cal-form-label">On days</span>
          <div
            role="group"
            aria-labelledby="recur-days-label"
            style="display: flex; gap: 4px;"
          >
            {#each ["S", "M", "T", "W", "T", "F", "S"] as day, i}
              <button
                type="button"
                id={`recur-day-${i}`}
                class="neu-subtle"
                style="width: 28px; height: 28px; border-radius: 50%; font-size: 11px; cursor: pointer;
                  {recurDays.includes(i)
                  ? 'background: var(--accent); color: white;'
                  : 'color: var(--text-secondary);'}"
                onclick={() => toggleRecurDay(i)}>{day}</button
              >
            {/each}
          </div>
        </div>
      {/if}

      <div class="cal-form-row">
        <label for="form-notify" class="cal-form-label">Notify me</label>
        <input id="form-notify" type="checkbox" bind:checked={formNotify} />
      </div>

      <div class="cal-form-actions">
        <button class="neu-subtle neu-btn cal-form-cancel" onclick={cancelForm}
          >Cancel</button
        >
        <button
          class="cal-form-save"
          onclick={handleSaveSession}
          disabled={isSaving}
        >
          {isSaving ? "Saving…" : isEditing ? "Update" : "Schedule"}
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .study-schedule-widget {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  /* Mini Calendar */
  .mini-calendar {
    display: flex;
    flex-direction: column;
    gap: 0;
    flex-shrink: 0;
    height: 180px;
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
    border-radius: 50%;
  }

  .cal-today .cal-day-num {
    color: white;
    font-weight: 600;
  }

  .cal-selected {
    background: color-mix(in srgb, var(--accent) 18%, transparent) !important;
  }

  .cal-selected .cal-day-num {
    color: var(--accent);
    font-weight: 600;
  }

  .cal-indicators {
    display: flex;
    gap: 2px;
    position: absolute;
    bottom: 2px;
  }

  .cal-dot {
    width: 4px;
    height: 4px;
    border-radius: 50%;
  }

  .cal-dot-session {
    background: var(--accent);
  }

  /* Upcoming */
  .upcoming-section {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
    flex-shrink: 0;
  }

  .section-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    font-family: var(--sans);
  }

  .add-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    font-size: 11px;
    font-weight: 500;
    color: var(--accent);
    border: none;
    border-radius: 5px;
    cursor: pointer;
    font-family: var(--sans);
    background: transparent;
  }

  .add-btn:hover {
    background: var(--accent-soft, rgba(196, 113, 79, 0.06));
  }

  .empty-msg {
    font-size: 12px;
    color: var(--text-muted);
    font-family: var(--sans);
    text-align: center;
    padding: 12px 0;
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
    padding: 8px 10px;
    border-radius: 8px;
    background: var(--bg-subtle);
    transition: opacity 0.15s;
  }

  .session-done {
    opacity: 0.45;
  }

  .session-check {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    border: 1.5px solid var(--border);
    background: transparent;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    margin-top: 2px;
    transition: all 0.15s;
  }

  .session-check:hover {
    border-color: var(--accent);
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
    background: none;
    border: none;
    padding: 0;
    text-align: left;
    cursor: pointer;
    font-family: var(--sans);
  }

  .session-info:hover .session-date {
    color: var(--accent);
  }

  .session-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .session-date {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
    font-family: var(--sans);
    transition: color 0.1s;
  }

  .session-time {
    font-size: 11px;
    color: var(--text-secondary);
    font-family: var(--sans);
  }

  .session-label {
    font-size: 11px;
    color: var(--text-secondary);
    font-family: var(--sans);
  }

  .session-recurrence {
    font-size: 10px;
    color: var(--accent);
    font-family: var(--sans);
    font-weight: 500;
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

  /* Delete confirmation for recurring */
  .delete-confirm {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .delete-opt {
    font-size: 9px;
    font-weight: 500;
    padding: 2px 6px;
    border-radius: 4px;
    border: none;
    cursor: pointer;
    font-family: var(--sans);
    background: var(--bg-card);
    color: var(--text-secondary);
    white-space: nowrap;
  }

  .delete-opt:hover {
    background: var(--bg-deep);
  }

  .delete-opt-all {
    color: var(--danger);
  }

  .delete-opt-all:hover {
    background: var(--danger-soft, rgba(192, 68, 74, 0.1));
  }

  /* Add / Edit Form */
  .add-form {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    background: var(--bg-subtle);
    border-radius: 8px;
    margin-top: 8px;
  }

  .form-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
    font-family: var(--sans);
    margin-bottom: 2px;
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
