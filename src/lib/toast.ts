import { writable } from "svelte/store";

export interface Toast {
  id: number;
  message: string;
  type: 'success' | 'error' | 'warning' | 'info';
  visible: boolean;
}

let toastIdCounter = 0;

export const toasts = writable<Toast[]>([]);

// Convenience wrapper object
export const toast = {
  success: (message: string) => addToast(message, 'success'),
  error: (message: string) => addToast(message, 'error'),
  warning: (message: string) => addToast(message, 'warning'),
  info: (message: string) => addToast(message, 'info'),
};

export function addToast(message: string, type: 'success' | 'error' | 'warning' | 'info') {
  const id = ++toastIdCounter;
  const toast: Toast = {
    id,
    message,
    type,
    visible: true
  };

  toasts.update(currentToasts => [...currentToasts, toast]);

  // Auto-remove after timeout
  const timeoutMs = type === 'success' ? 2000 : (type === 'warning' || type === 'info' ? 6000 : 4000);
  setTimeout(() => {
    // First make invisible for fade out animation
    toasts.update(currentToasts =>
      currentToasts.map(t => t.id === id ? { ...t, visible: false } : t)
    );

    // Then remove after animation
    setTimeout(() => {
      toasts.update(currentToasts => currentToasts.filter(t => t.id !== id));
    }, 300);
  }, timeoutMs);
}
