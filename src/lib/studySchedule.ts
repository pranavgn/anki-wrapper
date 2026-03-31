// src/lib/studySchedule.ts
// CRUD helpers for study sessions — used by StatsView calendar

import { invoke } from "@tauri-apps/api/core";
import type { StudySession } from "./notifications";

export type { StudySession };

// FIX: Use local date string instead of UTC's toISOString()
function getLocalDateStr(d: Date = new Date()): string {
  const year = d.getFullYear();
  const month = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export async function loadSessions(): Promise<StudySession[]> {
  return invoke<StudySession[]>("get_study_sessions");
}

export async function saveSession(session: StudySession): Promise<void> {
  await invoke("save_study_session", { session });
}

export async function deleteSession(sessionId: string): Promise<void> {
  await invoke("delete_study_session", { sessionId });
}

export async function markCompleted(sessionId: string, completed: boolean): Promise<void> {
  await invoke("mark_session_completed", { sessionId, completed });
}

export function createSession(overrides: Partial<StudySession> = {}): StudySession {
  const now = new Date();
  return {
    id: `ss_${Date.now()}`,
    // FIX: Use local date, not UTC
    date: getLocalDateStr(now),
    time: `${String(now.getHours()).padStart(2, "0")}:${String(now.getMinutes()).padStart(2, "0")}`,
    duration_mins: 30,
    deck_id: null,
    deck_name: null,
    card_goal: null,
    note: "",
    completed: false,
    notify: true,
    ...overrides,
  };
}

/** Group sessions by date for calendar rendering */
export function sessionsByDate(sessions: StudySession[]): Map<string, StudySession[]> {
  const map = new Map<string, StudySession[]>();
  for (const s of sessions) {
    const arr = map.get(s.date) || [];
    arr.push(s);
    map.set(s.date, arr);
  }
  return map;
}

/** Get sessions for a specific date */
export function sessionsForDate(sessions: StudySession[], date: string): StudySession[] {
  return sessions.filter((s) => s.date === date);
}

/** Upcoming sessions (today or future, not completed) */
export function upcomingSessions(sessions: StudySession[]): StudySession[] {
  // FIX: Use local date for comparison
  const today = getLocalDateStr();
  return sessions
    .filter((s) => s.date >= today && !s.completed)
    .sort((a, b) => {
      if (a.date !== b.date) return a.date.localeCompare(b.date);
      return a.time.localeCompare(b.time);
    });
}

/** Generate recurring sessions based on a base session */
export async function generateRecurringSessions(
  baseSession: StudySession,
  weeksToGenerate: number = 4
): Promise<void> {
  if (!baseSession.recurrence || baseSession.recurrence === 'none') return;

  // FIX: Use T12:00:00 to avoid UTC date shift
  const baseDate = new Date(baseSession.date + "T12:00:00");
  
  for (let week = 1; week <= weeksToGenerate; week++) {
    if (baseSession.recurrence === 'daily') {
      // Generate for each day
      for (let day = 1; day <= 7; day++) {
        const newDate = new Date(baseDate);
        newDate.setDate(newDate.getDate() + (week * 7) + day - 1);
        
        const recurringSession = createSession({
          ...baseSession,
          id: `ss_${Date.now()}_${week}_${day}`,
          date: getLocalDateStr(newDate),
          base_session_id: baseSession.id,
        });
        
        await saveSession(recurringSession);
      }
    } else if (baseSession.recurrence === 'weekly' && baseSession.recur_days && baseSession.recur_days.length > 0) {
      // Generate for specific days of the week
      for (const dayOfWeek of baseSession.recur_days) {
        const newDate = new Date(baseDate);
        const daysUntilNext = (dayOfWeek - newDate.getDay() + 7) % 7;
        newDate.setDate(newDate.getDate() + (week * 7) + daysUntilNext);
        
        const recurringSession = createSession({
          ...baseSession,
          id: `ss_${Date.now()}_${week}_${dayOfWeek}`,
          date: getLocalDateStr(newDate),
          base_session_id: baseSession.id,
        });
        
        await saveSession(recurringSession);
      }
    }
  }
}
