<script lang="ts">
  import { onMount } from "svelte";
  import { scheduleStore } from "../scheduleStore.svelte";
  import { SESSION_TYPE_THEMES, type CalendarView, type SessionType } from "../types/scheduler";
  import FloatingScheduler from "./FloatingScheduler.svelte";

  let view: CalendarView = $state('month');
  let currentDate = $state(new Date());
  let showViewDropdown = $state(false);
  let schedulerOpen = $state(false);
  let schedulerDate = $state<string | undefined>(undefined);

  const DAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
  const MONTHS = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];

  onMount(() => { scheduleStore.init(); });

  // FIX: Use local date string, not UTC (toISOString uses UTC which can be wrong timezone)
  function getLocalDateStr(d: Date = new Date()): string {
    const year = d.getFullYear();
    const month = String(d.getMonth() + 1).padStart(2, "0");
    const day = String(d.getDate()).padStart(2, "0");
    return `${year}-${month}-${day}`;
  }

  function todayStr(): string {
    return getLocalDateStr();
  }

  function isToday(dateStr: string): boolean {
    return dateStr === todayStr();
  }

  function getMonthDays(): Array<{ date: string; day: number; inMonth: boolean }> {
    const year = currentDate.getFullYear();
    const month = currentDate.getMonth();
    const firstDay = new Date(year, month, 1);
    const lastDay = new Date(year, month + 1, 0);
    const startDay = firstDay.getDay();
    const days: Array<{ date: string; day: number; inMonth: boolean }> = [];

    // Previous month days
    const prevLast = new Date(year, month, 0).getDate();
    for (let i = startDay - 1; i >= 0; i--) {
      const d = prevLast - i;
      const date = getLocalDateStr(new Date(year, month - 1, d));
      days.push({ date, day: d, inMonth: false });
    }

    // Current month days
    for (let d = 1; d <= lastDay.getDate(); d++) {
      const date = getLocalDateStr(new Date(year, month, d));
      days.push({ date, day: d, inMonth: true });
    }

    // Next month days
    const remaining = 42 - days.length;
    for (let d = 1; d <= remaining; d++) {
      const date = getLocalDateStr(new Date(year, month + 1, d));
      days.push({ date, day: d, inMonth: false });
    }

    return days;
  }

  function getWeekDays(): Array<{ date: string; day: number; dayName: string }> {
    const start = new Date(currentDate);
    start.setDate(start.getDate() - start.getDay());
    const days: Array<{ date: string; day: number; dayName: string }> = [];

    for (let i = 0; i < 7; i++) {
      const d = new Date(start);
      d.setDate(d.getDate() + i);
      days.push({
        date: getLocalDateStr(d),
        day: d.getDate(),
        dayName: DAYS[i].slice(0, 3),
      });
    }

    return days;
  }

  function getSessionTypes(dateStr: string): SessionType[] {
    const sessions = scheduleStore.sessionsFor(dateStr);
    const types = new Set<SessionType>();
    sessions.forEach(s => {
      if (s.session_type) types.add(s.session_type as SessionType);
    });
    return Array.from(types);
  }

  function getSessionsForDate(dateStr: string) {
    return scheduleStore.sessionsFor(dateStr);
  }

  function prevMonth() {
    currentDate = new Date(currentDate.getFullYear(), currentDate.getMonth() - 1, 1);
  }

  function nextMonth() {
    currentDate = new Date(currentDate.getFullYear(), currentDate.getMonth() + 1, 1);
  }

  function prevWeek() {
    const d = new Date(currentDate);
    d.setDate(d.getDate() - 7);
    currentDate = d;
  }

  function nextWeek() {
    const d = new Date(currentDate);
    d.setDate(d.getDate() + 7);
    currentDate = d;
  }

  function openScheduler(dateStr?: string) {
    schedulerDate = dateStr;
    schedulerOpen = true;
  }

  function formatTimeShort(time: string): string {
    const [h, m] = time.split(':').map(Number);
    const period = h >= 12 ? 'p' : 'a';
    return `${h % 12 || 12}:${String(m).padStart(2, '0')}${period}`;
  }
</script>

<div class="cal-widget" style="grid-row: span 2;">
  <div class="cal-header">
    <div class="cal-nav">
      <button class="cal-nav-btn" onclick={view === 'month' ? prevMonth : prevWeek}>‹</button>
      <span class="cal-title">
        {view === 'month' ? `${MONTHS[currentDate.getMonth()]} ${currentDate.getFullYear()}` : `Week of ${MONTHS[currentDate.getMonth()]} ${currentDate.getDate()}`}
      </span>
      <button class="cal-nav-btn" onclick={view === 'month' ? nextMonth : nextWeek}>›</button>
    </div>

    <div class="cal-actions">
      <div class="cal-view-toggle">
        <button class="cal-view-btn" onclick={() => showViewDropdown = !showViewDropdown}>
          {view === 'month' ? 'Month' : 'Week'} ▾
        </button>
        {#if showViewDropdown}
          <div class="cal-dropdown">
            <button class="cal-dropdown-item" onclick={() => { view = 'month'; showViewDropdown = false; }}>Month</button>
            <button class="cal-dropdown-item" onclick={() => { view = 'week'; showViewDropdown = false; }}>Week</button>
          </div>
        {/if}
      </div>

      <div class="cal-schedule-wrapper">
        <button class="cal-schedule-btn" onclick={() => openScheduler()}>
          + Schedule
        </button>
        <FloatingScheduler
          bind:open={schedulerOpen}
          initialDate={schedulerDate}
          onclose={() => { schedulerOpen = false; schedulerDate = undefined; }}
        />
      </div>
    </div>
  </div>

  {#if view === 'month'}
    <div class="cal-month">
      <div class="cal-weekdays">
        {#each DAYS as day}
          <div class="cal-weekday">{day}</div>
        {/each}
      </div>

      <div class="cal-grid">
        {#each getMonthDays() as { date, day, inMonth }}
          <button
            class="cal-day"
            class:cal-day-out={!inMonth}
            class:cal-day-today={isToday(date)}
            class:cal-day-has={scheduleStore.sessionDates.has(date)}
            onclick={() => openScheduler(date)}
          >
            <span class="cal-day-num">{day}</span>
            {#if scheduleStore.sessionDates.has(date)}
              <div class="cal-dots">
                {#each getSessionTypes(date) as type}
                  <span class="cal-dot" style="background: {SESSION_TYPE_THEMES[type].text};"></span>
                {/each}
              </div>
            {/if}
          </button>
        {/each}
      </div>
    </div>
  {:else}
    <div class="cal-week">
      {#each getWeekDays() as { date, day, dayName }}
        <div class="cal-week-col" class:cal-week-today={isToday(date)}>
          <div class="cal-week-header">
            <span class="cal-week-dayname">{dayName}</span>
            <span class="cal-week-daynum" class:cal-week-daynum-today={isToday(date)}>{day}</span>
          </div>
          <div class="cal-week-sessions">
            {#each getSessionsForDate(date) as session}
              <div
                class="cal-chip"
                style="background: {SESSION_TYPE_THEMES[(session.session_type as SessionType) || 'review'].bg}; color: {SESSION_TYPE_THEMES[(session.session_type as SessionType) || 'review'].text};"
              >
                {formatTimeShort(session.time)}
              </div>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .cal-widget {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 8px;
    position: relative;
    overflow: visible;
  }

  .cal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .cal-nav {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .cal-nav-btn {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--bg-subtle);
    border: none;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 16px;
    box-shadow: var(--neu-subtle);
  }
  .cal-nav-btn:hover { background: var(--bg-deep); }

  .cal-title {
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    min-width: 140px;
    text-align: center;
  }

  .cal-actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .cal-view-toggle {
    position: relative;
  }

  .cal-view-btn {
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 500;
    font-family: var(--sans);
    background: var(--bg-subtle);
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    cursor: pointer;
    box-shadow: var(--neu-subtle);
  }

  .cal-dropdown {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: 4px;
    background: var(--bg-card);
    border-radius: var(--radius-sm);
    box-shadow: var(--neu-up), 0 8px 24px rgba(0,0,0,0.12);
    z-index: 10;
    overflow: hidden;
  }

  .cal-dropdown-item {
    display: block;
    width: 100%;
    padding: 8px 16px;
    font-size: 12px;
    font-family: var(--sans);
    background: none;
    border: none;
    color: var(--text-primary);
    cursor: pointer;
    text-align: left;
  }
  .cal-dropdown-item:hover { background: var(--bg-subtle); }

  .cal-schedule-wrapper {
    position: relative;
  }

  .cal-schedule-btn {
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
  .cal-schedule-btn:hover { opacity: 0.92; }

  .cal-month {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .cal-weekdays {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 2px;
    background: var(--bg-deep);
    border-radius: 6px;
    padding: 0 2px;
    flex-shrink: 0;
  }

  .cal-weekday {
    padding: 4px 0;
    text-align: center;
    font-family: var(--sans);
    font-size: 10px;
    font-weight: 500;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .cal-grid {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 2px;
  }

  .cal-day {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    border-radius: var(--radius-sm);
    background: var(--bg-subtle);
    border: none;
    cursor: pointer;
    padding: 2px;
    position: relative;
  }
  .cal-day:hover { background: var(--bg-deep); }

  .cal-day-out {
    opacity: 0.3;
  }

  .cal-day-today {
    background: var(--accent) !important;
  }
  .cal-day-today .cal-day-num {
    color: #fff !important;
  }

  .cal-day-has {
    background: var(--accent-soft);
  }
  .cal-day-has .cal-day-num {
    color: var(--accent);
  }

  .cal-day-num {
    font-family: var(--sans);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: var(--text-primary);
  }

  .cal-dots {
    display: flex;
    gap: 2px;
    position: absolute;
    bottom: 2px;
  }

  .cal-dot {
    width: 3px;
    height: 3px;
    border-radius: 50%;
  }

  .cal-week {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 4px;
    flex: 1;
  }

  .cal-week-col {
    background: var(--bg-subtle);
    border-radius: var(--radius-sm);
    padding: 8px 4px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 200px;
  }

  .cal-week-today {
    background: var(--accent-soft);
  }

  .cal-week-header {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }

  .cal-week-dayname {
    font-family: var(--sans);
    font-size: 10px;
    font-weight: 500;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .cal-week-daynum {
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
  }

  .cal-week-daynum-today {
    background: var(--accent);
    color: #fff;
  }

  .cal-week-sessions {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .cal-chip {
    padding: 3px 6px;
    border-radius: 4px;
    font-family: var(--sans);
    font-size: 10px;
    font-weight: 500;
    text-align: center;
  }
</style>
