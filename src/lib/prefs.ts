import { invoke } from '@tauri-apps/api/core';

export interface AppPreferences {
  animations_enabled: boolean;
  reduce_motion: boolean;
}

// Global state using Svelte 5 runes
class PreferencesState {
  animations_enabled = $state(true);
  reduce_motion = $state(false);
  
  async load() {
    try {
      const p = await invoke<AppPreferences>('get_preferences');
      this.animations_enabled = p.animations_enabled;
      this.reduce_motion = p.reduce_motion;
    } catch (e) {
      console.error('Failed to load preferences:', e);
    }
  }
  
  async save() {
    try {
      await invoke('save_preferences', { 
        prefs: {
          animations_enabled: this.animations_enabled,
          reduce_motion: this.reduce_motion
        }
      });
    } catch (e) {
      console.error('Failed to save preferences:', e);
    }
  }
}

export const prefs = new PreferencesState();
