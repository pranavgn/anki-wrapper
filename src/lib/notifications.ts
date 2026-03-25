// src/lib/notifications.ts
// OS-level notifications for study reminders, milestones, and scheduled sessions

import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "@tauri-apps/api/core";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

let reminderInterval: ReturnType<typeof setInterval> | null = null;
let scheduleCheckInterval: ReturnType<typeof setInterval> | null = null;
const firedSessionIds = new Set<string>(); // prevent duplicate notifications per app session

// ── Permission management ──

export async function initNotifications(): Promise<boolean> {
  if (!(await isTauri())) return false;
  let granted = await isPermissionGranted();
  if (!granted) {
    const permission = await requestPermission();
    granted = permission === "granted";
  }
  return granted;
}

// ── Simple notifications ──

export async function sendStudyReminder(dueCount: number) {
  if (!(await isTauri()) || dueCount <= 0) return;
  if (!(await isPermissionGranted())) return;

  sendNotification({
    title: "Cards due for review",
    body: `You have ${dueCount} card${dueCount === 1 ? "" : "s"} waiting. Keep your streak going!`,
  });
}

export async function sendStreakReminder(streakDays: number) {
  if (!(await isTauri())) return;
  if (!(await isPermissionGranted())) return;

  sendNotification({
    title: `${streakDays}-day streak at risk!`,
    body: "Don't break your streak — review a few cards today.",
  });
}

export async function sendMilestoneNotification(type: string, value: number) {
  if (!(await isTauri())) return;
  if (!(await isPermissionGranted())) return;

  const messages: Record<string, string> = {
    streak: `Amazing! You've maintained a ${value}-day study streak!`,
    reviews: `Milestone reached: ${value.toLocaleString()} total reviews!`,
    retention: `Your retention rate just hit ${value}%!`,
  };

  sendNotification({
    title: "Achievement unlocked!",
    body: messages[type] || `Milestone: ${value}`,
  });
}

// ── Scheduled session notifications ──

export interface StudySession {
  id: string;
  date: string;        // "YYYY-MM-DD"
  time: string;        // "HH:MM"
  duration_mins: number;
  deck_id: number | null;
  deck_name: string | null;
  card_goal: number | null;
  note: string;
  completed: boolean;
  notify: boolean;
}

async function checkScheduledSessions() {
  try {
    const sessions = await invoke<StudySession[]>("get_study_sessions");
    const now = new Date();
    const todayStr = now.toISOString().split("T")[0];
    const currentMinutes = now.getHours() * 60 + now.getMinutes();

    for (const session of sessions) {
      if (session.date !== todayStr) continue;
      if (session.completed) continue;
      if (!session.notify) continue;
      if (firedSessionIds.has(session.id)) continue;

      const [h, m] = session.time.split(":").map(Number);
      const sessionMinutes = h * 60 + m;

      // Fire notification if we're within 2 minutes of the scheduled time
      // (check runs every 60s, so a 2-min window ensures we don't miss it)
      if (currentMinutes >= sessionMinutes && currentMinutes <= sessionMinutes + 2) {
        firedSessionIds.add(session.id);

        const deckLabel = session.deck_name || "all decks";
        const goalLabel = session.card_goal ? ` — ${session.card_goal} cards` : "";
        const noteLabel = session.note ? `\n${session.note}` : "";

        sendNotification({
          title: `Study session: ${deckLabel}`,
          body: `Scheduled for ${session.time}, ${session.duration_mins}min${goalLabel}${noteLabel}`,
        });
      }
    }
  } catch (e) {
    console.error("Schedule check failed:", e);
  }
}

// ── Periodic check loops ──

export function startDailyReminderCheck(
  getDueCount: () => Promise<number>,
  intervalMs: number = 3600000 // 1 hour
) {
  stopDailyReminderCheck();
  reminderInterval = setInterval(async () => {
    try {
      const due = await getDueCount();
      if (due > 0) await sendStudyReminder(due);
    } catch (e) {
      console.error("Reminder check failed:", e);
    }
  }, intervalMs);
}

export function stopDailyReminderCheck() {
  if (reminderInterval) {
    clearInterval(reminderInterval);
    reminderInterval = null;
  }
}

export function startScheduleChecker() {
  stopScheduleChecker();
  // Check every 60 seconds
  checkScheduledSessions(); // run immediately
  scheduleCheckInterval = setInterval(checkScheduledSessions, 60000);
}

export function stopScheduleChecker() {
  if (scheduleCheckInterval) {
    clearInterval(scheduleCheckInterval);
    scheduleCheckInterval = null;
  }
}

export function stopAll() {
  stopDailyReminderCheck();
  stopScheduleChecker();
}
