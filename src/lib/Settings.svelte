<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { fly, fade } from "svelte/transition";
  import { addToast } from "./toast";
  import { prefs } from "./prefs";

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  let isLoading = $state(false);
  let appVersion = $state("1.0.0");

  // Load app version on mount
  $effect(() => {
    loadVersion();
  });

  async function loadVersion() {
    try {
      appVersion = await invoke<string>("get_app_version");
    } catch (e) {
      console.error("Error loading version:", e);
    }
  }

  async function handleSave() {
    isLoading = true;
    try {
      await prefs.save();
      prefs.applyTheme();
      prefs.applyFontSize();
      addToast("Settings saved successfully", "success");
      onClose();
    } catch (e) {
      addToast(`Failed to save: ${e}`, "error");
    } finally {
      isLoading = false;
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }

  function handleThemeChange(newTheme: 'light' | 'dark' | 'system') {
    prefs.theme = newTheme;
    prefs.applyTheme();
  }

  // Backup state
  let backups = $state<Array<{name: string; path: string; created: string; size_bytes: number}>>([]);
  let isLoadingBackups = $state(false);
  let showRestoreConfirm = $state(false);
  let backupToRestore = $state('');

  $effect(() => {
    if (prefs.auto_backup) {
      checkAutoBackup();
    }
  });

  async function checkAutoBackup() {
    try {
      const lastBackup = await invoke<string | null>("get_last_backup_date");
      if (!lastBackup) {
        // No backups exist, create one
        await handleCreateBackup();
      }
    } catch (e) {
      console.error("Auto backup check failed:", e);
    }
  }

  async function loadBackups() {
    isLoadingBackups = true;
    try {
      backups = await invoke<Array<{name: string; path: string; created: string; size_bytes: number}>>("list_backups");
    } catch (e) {
      addToast(`Failed to load backups: ${e}`, "error");
    } finally {
      isLoadingBackups = false;
    }
  }

  async function handleCreateBackup() {
    try {
      const backupPath = await invoke<string>("create_backup");
      addToast("Backup created successfully", "success");
      await loadBackups();
    } catch (e) {
      addToast(`Failed to create backup: ${e}`, "error");
    }
  }

  function confirmRestore(backupName: string) {
    backupToRestore = backupName;
    showRestoreConfirm = true;
  }

  async function handleRestore() {
    if (!backupToRestore) return;
    try {
      await invoke("restore_backup", { backupName: backupToRestore });
      addToast("Backup restored. Reloading...", "success");
      showRestoreConfirm = false;
      // Reload the page after a short delay
      setTimeout(() => {
        window.location.reload();
      }, 1500);
    } catch (e) {
      addToast(`Failed to restore backup: ${e}`, "error");
    }
  }

  async function handleDeleteBackup(backupName: string) {
    if (!confirm(`Delete backup "${backupName}"?`)) return;
    try {
      await invoke("delete_backup", { backupName });
      addToast("Backup deleted", "success");
      await loadBackups();
    } catch (e) {
      addToast(`Failed to delete backup: ${e}`, "error");
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div 
  class="fixed inset-0 bg-black/15 z-50 flex justify-end"
  onclick={handleBackdropClick}
  transition:fade={{ duration: 150 }}
>
  <div 
    class="w-[480px] bg-white dark:bg-[#292524] h-full border-l border-border flex flex-col"
    transition:fly={{ x: 480, duration: 240 }}
  >
    <!-- Header -->
    <div class="px-6 py-4 border-b border-border flex items-center justify-between">
      <div>
        <h2 class="text-lg font-semibold text-text-primary">Settings</h2>
      </div>
      <button
        onclick={onClose}
        class="p-2 hover:bg-bg-subtle rounded-lg transition-colors"
      >
        <svg class="h-5 w-5 text-text-secondary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto px-6 py-4">
      <!-- Appearance Section -->
      <details class="group mb-6" open>
        <summary class="flex items-center justify-between cursor-pointer list-none py-2 font-medium text-text-primary">
          <span>Appearance</span>
          <svg class="h-4 w-4 text-text-secondary transition-transform group-open:rotate-180" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
          </svg>
        </summary>
        <div class="space-y-4 pt-2">
          <!-- Theme -->
          <div>
            <label class="block text-sm text-text-secondary mb-2">Theme</label>
            <div class="flex gap-2">
              <button
                onclick={() => handleThemeChange('light')}
                class="flex-1 px-4 py-2 rounded-lg border transition-colors {prefs.theme === 'light' ? 'border-accent bg-accent-soft text-text-primary' : 'border-border hover:bg-bg-subtle text-text-secondary'}"
              >
                <svg class="w-5 h-5 mx-auto mb-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
                </svg>
                <span class="text-xs">Light</span>
              </button>
              <button
                onclick={() => handleThemeChange('dark')}
                class="flex-1 px-4 py-2 rounded-lg border transition-colors {prefs.theme === 'dark' ? 'border-accent bg-accent-soft text-text-primary' : 'border-border hover:bg-bg-subtle text-text-secondary'}"
              >
                <svg class="w-5 h-5 mx-auto mb-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
                </svg>
                <span class="text-xs">Dark</span>
              </button>
              <button
                onclick={() => handleThemeChange('system')}
                class="flex-1 px-4 py-2 rounded-lg border transition-colors {prefs.theme === 'system' ? 'border-accent bg-accent-soft text-text-primary' : 'border-border hover:bg-bg-subtle text-text-secondary'}"
              >
                <svg class="w-5 h-5 mx-auto mb-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
                </svg>
                <span class="text-xs">System</span>
              </button>
            </div>
          </div>

          <!-- Font Size -->
          <div>
            <label class="block text-sm text-text-secondary mb-1">Font Size: {prefs.font_size}px</label>
            <input
              type="range"
              bind:value={prefs.font_size}
              min="12"
              max="20"
              step="2"
              class="w-full"
              onchange={() => prefs.applyFontSize()}
            />
            <div class="flex justify-between text-xs text-text-secondary mt-1">
              <span>12px</span>
              <span>20px</span>
            </div>
          </div>

          <!-- Animations -->
          <div class="flex items-center justify-between">
            <label class="text-sm text-text-secondary">Animations</label>
            <button
              onclick={() => prefs.animations_enabled = !prefs.animations_enabled}
              class="relative w-11 h-6 rounded-full transition-colors {prefs.animations_enabled ? 'bg-accent' : 'bg-bg-subtle'}"
            >
              <span class="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full transition-transform {prefs.animations_enabled ? 'translate-x-5' : ''}"></span>
            </button>
          </div>

          <!-- Reduce Motion -->
          <div class="flex items-center justify-between">
            <label class="text-sm text-text-secondary">Reduce Motion</label>
            <button
              onclick={() => prefs.reduce_motion = !prefs.reduce_motion}
              class="relative w-11 h-6 rounded-full transition-colors {prefs.reduce_motion ? 'bg-accent' : 'bg-bg-subtle'}"
            >
              <span class="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full transition-transform {prefs.reduce_motion ? 'translate-x-5' : ''}"></span>
            </button>
          </div>
        </div>
      </details>

      <!-- Study Section -->
      <details class="group mb-6">
        <summary class="flex items-center justify-between cursor-pointer list-none py-2 font-medium text-text-primary">
          <span>Study</span>
          <svg class="h-4 w-4 text-text-secondary transition-transform group-open:rotate-180" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
          </svg>
        </summary>
        <div class="space-y-4 pt-2">
          <!-- Daily Cutoff Hour -->
          <div>
            <label class="block text-sm text-text-secondary mb-1">Daily Cutoff Hour</label>
            <select
              bind:value={prefs.daily_cutoff_hour}
              class="w-full px-3 py-2 bg-bg-subtle border border-border rounded-lg text-text-primary"
            >
              {#each Array(24) as _, i}
                <option value={i}>{i}:00</option>
              {/each}
            </select>
            <p class="text-xs text-text-secondary mt-1">Hour when "today's" reviews reset</p>
          </div>

          <!-- Show Remaining Count -->
          <div class="flex items-center justify-between">
            <label class="text-sm text-text-secondary">Show Remaining Count</label>
            <button
              onclick={() => prefs.show_remaining_count = !prefs.show_remaining_count}
              class="relative w-11 h-6 rounded-full transition-colors {prefs.show_remaining_count ? 'bg-accent' : 'bg-bg-subtle'}"
            >
              <span class="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full transition-transform {prefs.show_remaining_count ? 'translate-x-5' : ''}"></span>
            </button>
          </div>

          <!-- Show Elapsed Time -->
          <div class="flex items-center justify-between">
            <label class="text-sm text-text-secondary">Show Elapsed Time</label>
            <button
              onclick={() => prefs.show_elapsed_time = !prefs.show_elapsed_time}
              class="relative w-11 h-6 rounded-full transition-colors {prefs.show_elapsed_time ? 'bg-accent' : 'bg-bg-subtle'}"
            >
              <span class="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full transition-transform {prefs.show_elapsed_time ? 'translate-x-5' : ''}"></span>
            </button>
          </div>

          <!-- Autoplay Audio -->
          <div class="flex items-center justify-between">
            <label class="text-sm text-text-secondary">Autoplay Audio</label>
            <button
              onclick={() => prefs.autoplay_audio = !prefs.autoplay_audio}
              class="relative w-11 h-6 rounded-full transition-colors {prefs.autoplay_audio ? 'bg-accent' : 'bg-bg-subtle'}"
            >
              <span class="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full transition-transform {prefs.autoplay_audio ? 'translate-x-5' : ''}"></span>
            </button>
          </div>
        </div>
      </details>

      <!-- Review Section -->
      <details class="group mb-6">
        <summary class="flex items-center justify-between cursor-pointer list-none py-2 font-medium text-text-primary">
          <span>Review</span>
          <svg class="h-4 w-4 text-text-secondary transition-transform group-open:rotate-180" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
          </svg>
        </summary>
        <div class="space-y-4 pt-2">
          <!-- Show Intervals on Buttons -->
          <div class="flex items-center justify-between">
            <label class="text-sm text-text-secondary">Show Intervals on Buttons</label>
            <button
              onclick={() => prefs.show_intervals_on_buttons = !prefs.show_intervals_on_buttons}
              class="relative w-11 h-6 rounded-full transition-colors {prefs.show_intervals_on_buttons ? 'bg-accent' : 'bg-bg-subtle'}"
            >
              <span class="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full transition-transform {prefs.show_intervals_on_buttons ? 'translate-x-5' : ''}"></span>
            </button>
          </div>

          <!-- Confirm Delete -->
          <div class="flex items-center justify-between">
            <label class="text-sm text-text-secondary">Confirm Before Delete</label>
            <button
              onclick={() => prefs.confirm_delete = !prefs.confirm_delete}
              class="relative w-11 h-6 rounded-full transition-colors {prefs.confirm_delete ? 'bg-accent' : 'bg-bg-subtle'}"
            >
              <span class="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full transition-transform {prefs.confirm_delete ? 'translate-x-5' : ''}"></span>
            </button>
          </div>
        </div>
      </details>

      <!-- Data Section -->
      <details class="group mb-6" open={isLoadingBackups}>
        <summary class="flex items-center justify-between cursor-pointer list-none py-2 font-medium text-text-primary">
          <span>Data</span>
          <svg class="h-4 w-4 text-text-secondary transition-transform group-open:rotate-180" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
          </svg>
        </summary>
        <div class="space-y-4 pt-2">
          <!-- Auto Backup -->
          <div class="flex items-center justify-between">
            <label class="text-sm text-text-secondary">Auto Backup</label>
            <button
              onclick={() => prefs.auto_backup = !prefs.auto_backup}
              class="relative w-11 h-6 rounded-full transition-colors {prefs.auto_backup ? 'bg-accent' : 'bg-bg-subtle'}"
            >
              <span class="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full transition-transform {prefs.auto_backup ? 'translate-x-5' : ''}"></span>
            </button>
          </div>

          <!-- Backup Count -->
          <div>
            <label class="block text-sm text-text-secondary mb-1">Backup Count: {prefs.backup_count}</label>
            <input
              type="range"
              bind:value={prefs.backup_count}
              min="1"
              max="20"
              step="1"
              class="w-full"
              disabled={!prefs.auto_backup}
            />
            <div class="flex justify-between text-xs text-text-secondary mt-1">
              <span>1</span>
              <span>20</span>
            </div>
          </div>

          <!-- Manual Backup Button -->
          <button
            onclick={handleCreateBackup}
            class="w-full px-4 py-2 bg-accent text-white rounded-lg hover:bg-accent/90 transition-colors"
          >
            Create Backup Now
          </button>

          <!-- Backup List -->
          <div>
            <button
              onclick={loadBackups}
              class="text-sm text-accent hover:underline mb-2"
            >
              {isLoadingBackups ? 'Loading...' : 'Show Backups'}
            </button>
            {#if backups.length > 0}
              <div class="space-y-2 max-h-48 overflow-y-auto">
                {#each backups as backup}
                  <div class="flex items-center justify-between p-2 bg-bg-subtle rounded-lg text-sm">
                    <div class="flex-1 min-w-0">
                      <p class="text-text-primary truncate">{backup.name.replace('.anki2', '')}</p>
                      <p class="text-xs text-text-secondary">{backup.created} • {formatBytes(backup.size_bytes)}</p>
                    </div>
                    <div class="flex gap-1 ml-2">
                      <button
                        onclick={() => confirmRestore(backup.name)}
                        class="px-2 py-1 text-xs text-accent hover:bg-accent/10 rounded"
                      >
                        Restore
                      </button>
                      <button
                        onclick={() => handleDeleteBackup(backup.name)}
                        class="px-2 py-1 text-xs text-danger hover:bg-danger/10 rounded"
                      >
                        Delete
                      </button>
                    </div>
                  </div>
                {/each}
              </div>
            {:else if !isLoadingBackups}
              <p class="text-sm text-text-secondary">No backups found</p>
            {/if}
          </div>
        </div>
      </details>

      <!-- Restore Confirmation Modal -->
      {#if showRestoreConfirm}
        <div 
          class="fixed inset-0 bg-black/50 z-50 flex items-center justify-center"
          onclick={() => showRestoreConfirm = false}
          transition:fade={{ duration: 150 }}
        >
          <div 
            class="bg-bg-card border border-border rounded-2xl p-6 max-w-sm mx-4 shadow-xl"
            onclick={(e) => e.stopPropagation()}
          >
            <h3 class="text-lg font-semibold text-text-primary mb-2">Restore Backup?</h3>
            <p class="text-sm text-text-secondary mb-4">
              This will replace your current collection with the backup. This action cannot be undone.
            </p>
            <div class="flex gap-3">
              <button
                onclick={() => showRestoreConfirm = false}
                class="flex-1 px-4 py-2 border border-border rounded-lg text-text-primary hover:bg-bg-subtle"
              >
                Cancel
              </button>
              <button
                onclick={handleRestore}
                class="flex-1 px-4 py-2 bg-danger text-white rounded-lg hover:bg-danger/90"
              >
                Restore
              </button>
            </div>
          </div>
        </div>
      {/if}

      <!-- About Section -->
      <details class="group mb-6">
        <summary class="flex items-center justify-between cursor-pointer list-none py-2 font-medium text-text-primary">
          <span>About</span>
          <svg class="h-4 w-4 text-text-secondary transition-transform group-open:rotate-180" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
          </svg>
        </summary>
        <div class="space-y-4 pt-2">
          <div class="text-sm text-text-secondary">
            <p><strong>Anki Wrapper</strong></p>
            <p>Version: {appVersion}</p>
            <p class="mt-2">A custom desktop UI for Anki</p>
          </div>
        </div>
      </details>
    </div>

    <!-- Footer -->
    <div class="sticky bottom-0 px-6 py-4 bg-white dark:bg-[#292524] border-t border-border flex gap-3">
      <button
        onclick={onClose}
        class="flex-1 px-4 py-2 border border-border rounded-lg text-text-primary hover:bg-bg-subtle transition-colors"
      >
        Cancel
      </button>
      <button
        onclick={handleSave}
        disabled={isLoading}
        class="flex-1 px-4 py-2 bg-accent text-white rounded-lg hover:bg-accent/90 transition-colors disabled:opacity-50"
      >
        {isLoading ? 'Saving...' : 'Save Settings'}
      </button>
    </div>
  </div>
</div>
