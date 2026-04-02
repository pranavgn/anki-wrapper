// src/lib/studySchedule.ts
// CRUD helpers for study sessions — used by StatsView calendar & StudyScheduleWidget

import { invoke } from "@tauri-apps/api/core";
import type { StudySession } from "./notifications";

export type { StudySession };

export async function loadSessions(): Promise<StudySession[]> {
  return invoke<StudySession[]>("get_study_sessions");
}

export async function saveSession(session: StudySession): Promise<void> {
  await invoke("save_study_session", { session });
}

/**
 * Update an existing session by saving over it with the same ID.
 * If it's a recurring base session, optionally regenerate the series.
 */
export async function updateSession(
  session: StudySession,
  regenerateRecurring = false
): Promise<void> {
  // Delete old recurring children first if we're regenerating
  if (regenerateRecurring && session.recurrence && session.recurrence !== "none") {
    await deleteRecurringSeries(session.id);
  }

  // Save the (possibly-modified) session
  await saveSession(session);

  // Regenerate recurring children if requested
  if (regenerateRecurring && session.recurrence && session.recurrence !== "none") {
    await generateRecurringSessions(session);
  }
}

export async function deleteSession(sessionId: string): Promise<void> {
  await invoke("delete_study_session", { sessionId });
}

/**
 * Delete all recurring children of a base session (but not the base itself).
 */
export async function deleteRecurringSeries(baseSessionId: string): Promise<void> {
  const sessions = await loadSessions();
  const children = sessions.filter(
    (s) => s.base_session_id === baseSessionId && s.id !== baseSessionId
  );
  for (const child of children) {
    await deleteSession(child.id);
  }
}

/**
 * Delete a session and, if it's a base recurring session,
 * also delete all its future instances.
 */
export async function deleteSessionWithRecurring(
  sessionId: string,
  allFuture: boolean
): Promise<void> {
  if (allFuture) {
    const sessions = await loadSessions();
    const target = sessions.find((s) => s.id === sessionId);
    if (target) {
      const baseId = target.base_session_id ?? target.id;
      const toDelete = sessions.filter(
        (s) =>
          (s.id === baseId || s.base_session_id === baseId) &&
          s.date >= target.date
      );
      for (const s of toDelete) {
        await deleteSession(s.id);
      }
      return;
    }
  }
  await deleteSession(sessionId);
}

export async function markCompleted(
  sessionId: string,
  completed: boolean
): Promise<void> {
  await invoke("mark_session_completed", { sessionId, completed });
}

export function createSession(
  overrides: Partial<StudySession> = {}
): StudySession {
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
export function sessionsByDate(
  sessions: StudySession[]
): Map<string, StudySession[]> {
  const map = new Map<string, StudySession[]>();
  for (const s of sessions) {
    const arr = map.get(s.date) || [];
    arr.push(s);
    map.set(s.date, arr);
  }
  return map;
}

/** Get sessions for a specific date */
export function sessionsForDate(
  sessions: StudySession[],
  date: string
): StudySession[] {
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

/** Generate recurring sessions based on a base session */
export async function generateRecurringSessions(
  baseSession: StudySession,
  weeksToGenerate: number = 4
): Promise<void> {
  if (!baseSession.recurrence || baseSession.recurrence === "none") return;

  const baseDate = new Date(baseSession.date);

  for (let week = 1; week <= weeksToGenerate; week++) {
    if (baseSession.recurrence === "daily") {
      for (let day = 1; day <= 7; day++) {
        const newDate = new Date(baseDate);
        newDate.setDate(newDate.getDate() + week * 7 + day - 1);

        const recurringSession = createSession({
          ...baseSession,
          id: `ss_${Date.now()}_${week}_${day}`,
          date: newDate.toISOString().split("T")[0],
          base_session_id: baseSession.id,
        });

        await saveSession(recurringSession);
      }
    } else if (
      baseSession.recurrence === "weekly" &&
      baseSession.recur_days &&
      baseSession.recur_days.length > 0
    ) {
      for (const dayOfWeek of baseSession.recur_days) {
        const newDate = new Date(baseDate);
        const daysUntilNext = (dayOfWeek - newDate.getDay() + 7) % 7;
        newDate.setDate(newDate.getDate() + week * 7 + daysUntilNext);

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
