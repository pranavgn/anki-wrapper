import type { StudySession } from "../notifications";

/** Session display type — drives color coding across all widgets */
export type SessionType = 'review' | 'new' | 'mixed';

/** Color config per session type */
export interface SessionTypeTheme {
  bg: string;
  text: string;
  label: string;
}

export const SESSION_TYPE_THEMES: Record<SessionType, SessionTypeTheme> = {
  review: { bg: 'rgba(83, 74, 183, 0.1)',  text: '#534AB7', label: 'Review' },
  new:    { bg: 'rgba(29, 158, 117, 0.1)', text: '#0F6E56', label: 'New cards' },
  mixed:  { bg: 'rgba(55, 138, 221, 0.1)', text: '#185FA5', label: 'Mixed' },
};

/** Calendar view mode */
export type CalendarView = 'month' | 'week';

/** Generic floating popup state — reusable across any widget */
export interface FloatingPopupState<T = unknown> {
  open: boolean;
  anchor: { x: number; y: number } | null;
  data: T | null;
}

/** Scheduler popup data payload */
export interface SchedulerPopupData {
  date: string;          // pre-filled YYYY-MM-DD
  time: string;          // default "09:00"
  duration: number;      // default 30
  deckId: number | null;
  deckName: string | null;
  sessionType: SessionType;
}

/** Helper to create a closed popup state */
export function closedPopup<T>(): FloatingPopupState<T> {
  return { open: false, anchor: null, data: null };
}

/** Helper to create default scheduler data */
export function defaultSchedulerData(dateStr?: string): SchedulerPopupData {
  // FIX: Use local date string, not UTC
  const today = new Date().toLocaleDateString('en-CA'); // YYYY-MM-DD format in local time
  return {
    date: dateStr || today,
    time: '09:00',
    duration: 30,
    deckId: null,
    deckName: null,
    sessionType: 'review',
  };
}

/** Duration options for the scheduler dropdown */
export const DURATION_OPTIONS = [
  { value: 15, label: '15 min' },
  { value: 20, label: '20 min' },
  { value: 30, label: '30 min' },
  { value: 45, label: '45 min' },
  { value: 60, label: '60 min' },
] as const;

/** Format time from 24h "HH:MM" to 12h "9:00 AM" */
export function formatTime12h(time: string): string {
  const [h, m] = time.split(':').map(Number);
  const period = h >= 12 ? 'PM' : 'AM';
  return `${h % 12 || 12}:${String(m).padStart(2, '0')} ${period}`;
}

/** Format duration to human string */
export function formatDuration(mins: number): string {
  if (mins < 60) return `${mins}m`;
  const h = Math.floor(mins / 60);
  const m = mins % 60;
  return m > 0 ? `${h}h ${m}m` : `${h}h`;
}

/** Format date to relative or short label */
export function formatDateLabel(dateStr: string): string {
  // FIX: Use local date string, not UTC
  const today = new Date().toLocaleDateString('en-CA'); // YYYY-MM-DD format in local time
  const tomorrow = new Date();
  tomorrow.setDate(tomorrow.getDate() + 1);
  const tomorrowStr = tomorrow.toLocaleDateString('en-CA');

  if (dateStr === today) return 'Today';
  if (dateStr === tomorrowStr) return 'Tomorrow';
  const d = new Date(dateStr + 'T12:00:00');
  return d.toLocaleDateString('en-US', { weekday: 'short', month: 'short', day: 'numeric' });
}
