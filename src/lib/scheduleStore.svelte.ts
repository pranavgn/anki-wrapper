import { invoke } from "@tauri-apps/api/core";
import {
  loadSessions,
  saveSession,
  deleteSession,
  markCompleted,
  createSession,
  upcomingSessions as getUpcoming,
  sessionsByDate,
  generateRecurringSessions,
  type StudySession,
} from "./studySchedule";
import { addToast } from "./toast";
import type { SchedulerPopupData } from "./types/scheduler";

class ScheduleStore {
  sessions = $state<StudySession[]>([]);
  decks = $state<Array<{ id: number; name: string }>>([]);
  loading = $state(false);
  initialized = $state(false);

  /** Upcoming sessions (today+future, not completed, sorted) */
  upcoming = $derived.by(() => getUpcoming(this.sessions));

  /** Set of dates that have sessions — for calendar dot rendering */
  sessionDates = $derived(new Set(this.sessions.map(s => s.date)));

  /** Sessions grouped by date */
  byDate = $derived.by(() => sessionsByDate(this.sessions));

  async init() {
    if (this.initialized) return;
    this.loading = true;
    try {
      const [sessions, deckResult] = await Promise.all([
        loadSessions(),
        invoke<Array<{ id: number; name: string; short_name: string; level: number; new_count: number; learn_count: number; review_count: number; card_count: number; is_filtered: boolean }>>("get_all_decks")
          .catch(() => [] as Array<{ id: number; name: string }>),
      ]);
      this.sessions = sessions;
      this.decks = deckResult.map(d => ({ id: d.id, name: d.name }));
      this.initialized = true;
    } catch (e) {
      console.error("Failed to init schedule store:", e);
    } finally {
      this.loading = false;
    }
  }

  async refresh() {
    try {
      this.sessions = await loadSessions();
    } catch (e) {
      console.error("Failed to refresh sessions:", e);
    }
  }

  async addSession(data: SchedulerPopupData) {
    try {
      const session = createSession({
        date: data.date,
        time: data.time,
        duration_mins: data.duration,
        deck_id: data.deckId,
        deck_name: data.deckName,
      });
      await saveSession(session);
      await this.refresh();
      addToast("Session scheduled", "success");
    } catch (e) {
      addToast(`Failed to save: ${e}`, "error");
    }
  }

  async removeSession(id: string) {
    try {
      const target = this.sessions.find(s => s.id === id);
      if (target?.base_session_id) {
        const future = this.sessions.filter(s =>
          s.base_session_id === target.base_session_id && s.date >= target.date
        );
        for (const s of future) await deleteSession(s.id);
      } else {
        await deleteSession(id);
      }
      await this.refresh();
      addToast("Session removed", "success");
    } catch (e) {
      addToast(`Failed to delete: ${e}`, "error");
    }
  }

  async toggleCompleted(id: string, currentlyCompleted: boolean) {
    try {
      await markCompleted(id, !currentlyCompleted);
      await this.refresh();
    } catch (e) {
      addToast(`Failed to update: ${e}`, "error");
    }
  }

  /** Get sessions for a specific date */
  sessionsFor(date: string): StudySession[] {
    return this.sessions.filter(s => s.date === date);
  }
}

/** Singleton store instance — imported by all schedule widgets */
export const scheduleStore = new ScheduleStore();
