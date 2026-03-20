import { invoke } from '@tauri-apps/api/core';

export interface AppPreferences {
  // Display
  animations_enabled: boolean;
  reduce_motion: boolean;
  theme: 'light' | 'dark' | 'system';
  font_size: number;
  
  // Study
  daily_cutoff_hour: number;
  show_remaining_count: boolean;
  show_elapsed_time: boolean;
  autoplay_audio: boolean;
  
  // Review
  show_intervals_on_buttons: boolean;
  confirm_delete: boolean;
  
  // Backup
  auto_backup: boolean;
  backup_count: number;
}

// Global state using Svelte 5 runes
class PreferencesState {
  animations_enabled = $state(true);
  reduce_motion = $state(false);
  theme = $state<'light' | 'dark' | 'system'>('system');
  font_size = $state(16);
  
  daily_cutoff_hour = $state(4);
  show_remaining_count = $state(true);
  show_elapsed_time = $state(true);
  autoplay_audio = $state(true);
  
  show_intervals_on_buttons = $state(true);
  confirm_delete = $state(true);
  
  auto_backup = $state(false);
  backup_count = $state(5);
  
  async load() {
    try {
      const p = await invoke<AppPreferences>('get_preferences');
      this.animations_enabled = p.animations_enabled;
      this.reduce_motion = p.reduce_motion;
      this.theme = p.theme as 'light' | 'dark' | 'system';
      this.font_size = p.font_size;
      this.daily_cutoff_hour = p.daily_cutoff_hour;
      this.show_remaining_count = p.show_remaining_count;
      this.show_elapsed_time = p.show_elapsed_time;
      this.autoplay_audio = p.autoplay_audio;
      this.show_intervals_on_buttons = p.show_intervals_on_buttons;
      this.confirm_delete = p.confirm_delete;
      this.auto_backup = p.auto_backup;
      this.backup_count = p.backup_count;
      
      // Apply theme immediately
      this.applyTheme();
      this.applyFontSize();
    } catch (e) {
      console.error('Failed to load preferences:', e);
    }
  }
  
  async save() {
    try {
      await invoke('save_preferences', { 
        prefs: {
          animations_enabled: this.animations_enabled,
          reduce_motion: this.reduce_motion,
          theme: this.theme,
          font_size: this.font_size,
          daily_cutoff_hour: this.daily_cutoff_hour,
          show_remaining_count: this.show_remaining_count,
          show_elapsed_time: this.show_elapsed_time,
          autoplay_audio: this.autoplay_audio,
          show_intervals_on_buttons: this.show_intervals_on_buttons,
          confirm_delete: this.confirm_delete,
          auto_backup: this.auto_backup,
          backup_count: this.backup_count,
        }
      });
    } catch (e) {
      console.error('Failed to save preferences:', e);
    }
  }
  
  applyTheme() {
    const root = document.documentElement;
    
    if (this.theme === 'system') {
      const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      root.classList.toggle('dark', prefersDark);
    } else {
      root.classList.toggle('dark', this.theme === 'dark');
    }
  }
  
  applyFontSize() {
    document.documentElement.style.fontSize = `${this.font_size}px`;
  }
  
  // Listen for system theme changes
  initThemeListener() {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    mediaQuery.addEventListener('change', () => {
      if (this.theme === 'system') {
        this.applyTheme();
      }
    });
  }
}

export const prefs = new PreferencesState();
