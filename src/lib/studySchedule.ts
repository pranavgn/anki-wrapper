// src/lib/studySchedule.ts
// CRUD helpers for study sessions — used by StatsView calendar

import { invoke } from "@tauri-apps/api/core";
import type { StudySession } from "./notifications";

export type { StudySession };

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
    date: now.toISOString().split("T")[0],
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
  const today = new Date().toISOString().split("T")[0];
  return sessions
    .filter((s) => s.date >= today && !s.completed)
    .sort((a, b) => {
      if (a.date !== b.date) return a.date.localeCompare(b.date);
      return a.time.localeCompare(b.time);
    });
}
