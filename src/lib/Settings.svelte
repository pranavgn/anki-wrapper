<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { addToast } from "./toast";
  import { prefs } from "./prefs.svelte";
  import NeuDialog from "./ui/NeuDialog.svelte";
  import NeuToggle from "./ui/NeuToggle.svelte";
  import { DESIGN_PRESETS, ACCENT_PRESETS } from './themes';
  import { save, open } from "@tauri-apps/plugin-dialog";

  interface Props {
    isOpen: boolean;
    onClose: () => void;
  }

  let { isOpen, onClose }: Props = $props();

  let isLoading = $state(false);
  let appVersion = $state("1.0.0");
  let isExporting = $state(false);
  let isImporting = $state(false);
  let showResetConfirm = $state(false);

  // Load app version on mount
  $effect(() => {
    if (isOpen) {
      loadVersion();
    }
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

  function handleThemeChange(newTheme: 'light' | 'dark' | 'system') {
    prefs.theme = newTheme;
    prefs.applyTheme();
  }

  function handleDesignPresetChange(presetId: string) {
    prefs.design_preset = presetId;
    prefs.applyTheme();
    prefs.save();
  }

  function handleAccentColorChange(color: string) {
    prefs.accent_color = color;
    prefs.applyTheme();
    prefs.save();
  }

  // Auto-save on toggle change
  async function handleToggleChange() {
    try {
      await prefs.save();
    } catch (e) {
      console.error("Failed to auto-save preferences:", e);
    }
  }

  // Statistics functions
  async function handleExportStats() {
    isExporting = true;
    try {
      const file = await save({ filters: [{ name: "CSV", extensions: ["csv"] }], defaultPath: "review-history.csv" });
      if (!file) { isExporting = false; return; }
      await invoke("export_review_stats_csv", { outPath: file });
      addToast("Review history exported", "success");
    } catch (e) {
      addToast(`Export failed: ${e}`, "error");
    } finally { isExporting = false; }
  }

  async function handleImportStats() {
    isImporting = true;
    try {
      const file = await open({ filters: [{ name: "CSV", extensions: ["csv"] }], multiple: false });
      if (!file) { isImporting = false; return; }
      const count = await invoke<number>("import_review_stats_csv", { path: file });
      addToast(`Imported ${count} review entries`, "success");
    } catch (e) {
      addToast(`Import failed: ${e}`, "error");
    } finally { isImporting = false; }
  }

  async function handleResetStats() {
    try {
      const deleted = await invoke<number>("reset_review_stats");
      addToast(`Deleted ${deleted} review entries`, "success");
      showResetConfirm = false;
    } catch (e) {
      addToast(`Reset failed: ${e}`, "error");
    }
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

<NeuDialog {isOpen} {onClose} title="Settings" size="lg">
  <div class="settings-content">
    <!-- Appearance Section -->
    <div class="settings-section">
      <h3 class="section-header">Appearance</h3>
      
      <!-- Theme -->
      <div class="setting-row">
        <span class="setting-label">Theme</span>
        <div class="theme-buttons">
          <button
            onclick={() => handleThemeChange('light')}
            class="theme-btn neu-subtle neu-btn {prefs.theme === 'light' ? 'active' : ''}"
            title="Light"
          >
            <svg class="theme-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
            </svg>
          </button>
          <button
            onclick={() => handleThemeChange('dark')}
            class="theme-btn neu-subtle neu-btn {prefs.theme === 'dark' ? 'active' : ''}"
            title="Dark"
          >
            <svg class="theme-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
            </svg>
          </button>
          <button
            onclick={() => handleThemeChange('system')}
            class="theme-btn neu-subtle neu-btn {prefs.theme === 'system' ? 'active' : ''}"
            title="System"
          >
            <svg class="theme-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
            </svg>
          </button>
        </div>
      </div>

      <!-- Font Size -->
      <div class="setting-row">
        <span class="setting-label">Font Size: {prefs.font_size}px</span>
        <div class="slider-container">
          <input
            type="range"
            bind:value={prefs.font_size}
            min="12"
            max="20"
            step="2"
            class="range-slider"
            onchange={() => prefs.applyFontSize()}
          />
          <div class="slider-labels">
            <span>12px</span>
            <span>20px</span>
          </div>
        </div>
      </div>

      <!-- Animations -->
      <div class="setting-row">
        <span class="setting-label">Animations</span>
        <NeuToggle
          bind:checked={prefs.animations_enabled}
          onchange={handleToggleChange}
        />
      </div>

      <!-- Reduce Motion -->
      <div class="setting-row">
        <span class="setting-label">Reduce Motion</span>
        <NeuToggle
          bind:checked={prefs.reduce_motion}
          onchange={handleToggleChange}
        />
      </div>

      <!-- Design Style -->
      <div class="setting-row" style="flex-direction: column; align-items: stretch; gap: 8px;">
        <span class="setting-label">Design Style</span>
        <div style="display: flex; gap: 8px;">
          {#each DESIGN_PRESETS as preset}
            <button
              onclick={() => handleDesignPresetChange(preset.id)}
              class="neu-subtle neu-btn"
              style="
                flex: 1;
                padding: 10px 8px;
                text-align: center;
                border: none;
                cursor: pointer;
                {prefs.design_preset === preset.id ? 'outline: 2px solid var(--accent); outline-offset: 2px;' : ''}
              "
            >
              <span style="font-family: var(--sans); font-size: 12px; font-weight: 600; color: var(--text-primary); display: block;">
                {preset.name}
              </span>
              <span style="font-family: var(--sans); font-size: 10px; color: var(--text-muted); display: block; margin-top: 2px;">
                {preset.description}
              </span>
            </button>
          {/each}
        </div>
      </div>

      <!-- Accent Color -->
      <div class="setting-row">
        <span class="setting-label">Accent Color</span>
        <div style="display: flex; gap: 8px;">
          {#each ACCENT_PRESETS as preset}
            <button
              onclick={() => handleAccentColorChange(preset.color)}
              class="neu-btn"
              title={preset.name}
              style="
                width: 28px;
                height: 28px;
                border-radius: 50%;
                background: {preset.color};
                border: 2px solid {prefs.accent_color === preset.color ? 'var(--text-primary)' : 'transparent'};
                cursor: pointer;
                box-shadow: {prefs.accent_color === preset.color ? '0 0 0 2px var(--bg-card), 0 0 0 4px ' + preset.color : 'none'};
                transition: transform 0.1s ease, box-shadow 0.1s ease;
              "
            ></button>
          {/each}
        </div>
      </div>
    </div>

    <!-- Study Section -->
    <div class="settings-section">
      <h3 class="section-header">Study</h3>
      
      <!-- Show Remaining Count -->
      <div class="setting-row">
        <span class="setting-label">Show Remaining Count</span>
        <NeuToggle
          bind:checked={prefs.show_remaining_count}
          onchange={handleToggleChange}
        />
      </div>

      <!-- Show Elapsed Time -->
      <div class="setting-row">
        <span class="setting-label">Show Elapsed Time</span>
        <NeuToggle
          bind:checked={prefs.show_elapsed_time}
          onchange={handleToggleChange}
        />
      </div>

      <!-- Show Intervals on Buttons -->
      <div class="setting-row">
        <span class="setting-label">Show Intervals on Buttons</span>
        <NeuToggle
          bind:checked={prefs.show_intervals_on_buttons}
          onchange={handleToggleChange}
        />
      </div>

      <!-- Autoplay Audio -->
      <div class="setting-row">
        <span class="setting-label">Autoplay Audio</span>
        <NeuToggle
          bind:checked={prefs.autoplay_audio}
          onchange={handleToggleChange}
        />
      </div>
    </div>

    <!-- Backup Section -->
    <div class="settings-section">
      <h3 class="section-header">Backup</h3>
      
      <!-- Auto Backup -->
      <div class="setting-row">
        <span class="setting-label">Auto Backup</span>
        <NeuToggle
          bind:checked={prefs.auto_backup}
          onchange={handleToggleChange}
        />
      </div>

      <!-- Backups to Keep -->
      <div class="setting-row">
        <span class="setting-label">Backups to Keep</span>
        <span class="setting-value">{prefs.backup_count}</span>
      </div>

      <!-- Manual Backup Button -->
      <button
        onclick={handleCreateBackup}
        class="backup-btn neu-subtle neu-btn"
      >
        Create Backup Now
      </button>

      <!-- Backup List -->
      <div class="backup-list">
        <button
          onclick={loadBackups}
          class="show-backups-btn"
        >
          {isLoadingBackups ? 'Loading...' : 'Show Backups'}
        </button>
        {#if backups.length > 0}
          <div class="backups-container">
            {#each backups as backup}
              <div class="backup-item">
                <div class="backup-info">
                  <p class="backup-name">{backup.name.replace('.anki2', '')}</p>
                  <p class="backup-meta">{backup.created} • {formatBytes(backup.size_bytes)}</p>
                </div>
                <div class="backup-actions">
                  <button
                    onclick={() => confirmRestore(backup.name)}
                    class="backup-action-btn restore-btn"
                  >
                    Restore
                  </button>
                  <button
                    onclick={() => handleDeleteBackup(backup.name)}
                    class="backup-action-btn delete-btn"
                  >
                    Delete
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {:else if !isLoadingBackups}
          <p class="no-backups">No backups found</p>
        {/if}
      </div>
    </div>

    <!-- Statistics Section -->
    <div class="settings-section">
      <h3 class="section-header">Statistics</h3>

      <div class="setting-row">
        <div>
          <span class="setting-label">Export Review History</span>
          <p class="setting-desc">Download all review data as CSV</p>
        </div>
        <button onclick={handleExportStats} class="neu-subtle neu-btn setting-action-btn" disabled={isExporting}>
          {isExporting ? 'Exporting...' : 'Export CSV'}
        </button>
      </div>

      <div class="setting-row">
        <div>
          <span class="setting-label">Import Review History</span>
          <p class="setting-desc">Merge CSV review data into your log</p>
        </div>
        <button onclick={handleImportStats} class="neu-subtle neu-btn setting-action-btn" disabled={isImporting}>
          {isImporting ? 'Importing...' : 'Import CSV'}
        </button>
      </div>

      <div class="setting-row">
        <div>
          <span class="setting-label">Notifications</span>
          <p class="setting-desc">Daily study reminders when cards are due</p>
        </div>
        <NeuToggle
          bind:checked={prefs.notifications_enabled}
          onchange={handleToggleChange}
        />
      </div>

      {#if !showResetConfirm}
        <div class="setting-row">
          <div>
            <span class="setting-label setting-danger-label">Reset All Statistics</span>
            <p class="setting-desc">Permanently delete your entire review history</p>
          </div>
          <button onclick={() => showResetConfirm = true} class="setting-danger-btn">
            Reset
          </button>
        </div>
      {:else}
        <div class="danger-confirm-box">
          <p class="danger-confirm-text">
            This will permanently delete all review history. This cannot be undone.
          </p>
          <div class="danger-confirm-actions">
            <button onclick={() => showResetConfirm = false} class="neu-subtle neu-btn setting-action-btn">Cancel</button>
            <button onclick={handleResetStats} class="setting-danger-btn">Yes, delete everything</button>
          </div>
        </div>
      {/if}
    </div>

    <!-- About Section -->
    <div class="settings-section">
      <h3 class="section-header">About</h3>
      <div class="about-content">
        <p class="about-title">Anki Wrapper</p>
        <p class="about-version">Version: {appVersion}</p>
        <p class="about-description">A custom desktop UI for Anki</p>
      </div>
    </div>
  </div>

  <!-- Footer -->
  <div class="settings-footer">
    <button
      onclick={onClose}
      class="cancel-btn neu-subtle neu-btn"
    >
      Cancel
    </button>
    <button
      onclick={handleSave}
      disabled={isLoading}
      class="save-btn"
    >
      {isLoading ? 'Saving...' : 'Save Settings'}
    </button>
  </div>
</NeuDialog>

<!-- Restore Confirmation Modal -->
<NeuDialog
  isOpen={showRestoreConfirm}
  onClose={() => showRestoreConfirm = false}
  title="Restore Backup?"
  size="sm"
>
  <div class="restore-dialog-content">
    <p class="restore-message">
      This will replace your current collection with the backup. This action cannot be undone.
    </p>
    <div class="restore-actions">
      <button
        onclick={() => showRestoreConfirm = false}
        class="cancel-btn neu-subtle neu-btn"
      >
        Cancel
      </button>
      <button
        onclick={handleRestore}
        class="restore-confirm-btn"
      >
        Restore
      </button>
    </div>
  </div>
</NeuDialog>

<style>
  .settings-content {
    display: flex;
    flex-direction: column;
    gap: 24px;
    max-height: 60vh;
    overflow-y: auto;
    padding-right: 8px;
  }

  .settings-section {
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .section-header {
    font-family: var(--sans);
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-muted);
    margin: 0 0 8px 0;
  }

  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 0;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 30%, transparent);
  }

  .setting-row:last-child {
    border-bottom: none;
  }

  .setting-label {
    font-family: var(--sans);
    font-size: 14px;
    color: var(--text-primary);
  }

  .setting-value {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-secondary);
  }

  .theme-buttons {
    display: flex;
    gap: 8px;
  }

  .theme-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 36px;
    border: none;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .theme-btn.active {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .theme-icon {
    width: 18px;
    height: 18px;
    color: var(--text-secondary);
  }

  .theme-btn.active .theme-icon {
    color: var(--accent);
  }

  .slider-container {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 140px;
  }

  .range-slider {
    width: 100%;
    height: 6px;
    border-radius: 3px;
    background: var(--bg-deep);
    appearance: none;
    cursor: pointer;
  }

  .range-slider::-webkit-slider-thumb {
    appearance: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--accent);
    cursor: pointer;
    box-shadow: 0 2px 6px rgba(0,0,0,0.15);
  }

  .range-slider::-moz-range-thumb {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--accent);
    cursor: pointer;
    border: none;
    box-shadow: 0 2px 6px rgba(0,0,0,0.15);
  }

  .slider-labels {
    display: flex;
    justify-content: space-between;
    font-family: var(--sans);
    font-size: 11px;
    color: var(--text-secondary);
  }

  .backup-btn {
    width: 100%;
    padding: 12px 16px;
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 500;
    color: var(--accent);
    border: none;
    cursor: pointer;
    margin-top: 8px;
  }

  .backup-list {
    margin-top: 12px;
  }

  .show-backups-btn {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--accent);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    text-decoration: underline;
  }

  .show-backups-btn:hover {
    opacity: 0.8;
  }

  .backups-container {
    margin-top: 12px;
    max-height: 192px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .backup-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    background: var(--bg-subtle);
    border-radius: var(--radius-sm);
  }

  .backup-info {
    flex: 1;
    min-width: 0;
  }

  .backup-name {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-primary);
    margin: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .backup-meta {
    font-family: var(--sans);
    font-size: 11px;
    color: var(--text-secondary);
    margin: 2px 0 0 0;
  }

  .backup-actions {
    display: flex;
    gap: 4px;
    margin-left: 8px;
  }

  .backup-action-btn {
    padding: 4px 8px;
    font-family: var(--sans);
    font-size: 11px;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .restore-btn {
    color: var(--accent);
    background: transparent;
  }

  .restore-btn:hover {
    background: var(--accent-soft);
  }

  .delete-btn {
    color: var(--danger);
    background: transparent;
  }

  .delete-btn:hover {
    background: color-mix(in srgb, var(--danger) 10%, transparent);
  }

  .no-backups {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-secondary);
    margin: 8px 0 0 0;
  }

  .about-content {
    padding: 8px 0;
  }

  .about-title {
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 4px 0;
  }

  .about-version {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0 0 8px 0;
  }

  .about-description {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0;
  }

  .settings-footer {
    display: flex;
    gap: 12px;
    margin-top: 24px;
    padding-top: 16px;
    border-top: 1px solid color-mix(in srgb, var(--border) 30%, transparent);
  }

  .cancel-btn {
    flex: 1;
    padding: 12px 16px;
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 500;
    color: var(--text-secondary);
    border: none;
    cursor: pointer;
  }

  .save-btn {
    flex: 1;
    padding: 12px 16px;
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 500;
    color: white;
    background: var(--accent);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .save-btn:hover:not(:disabled) {
    opacity: 0.9;
  }

  .save-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .restore-dialog-content {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .restore-message {
    font-family: var(--sans);
    font-size: 14px;
    line-height: 1.6;
    color: var(--text-secondary);
    margin: 0;
  }

  .restore-actions {
    display: flex;
    gap: 12px;
  }

  .restore-confirm-btn {
    flex: 1;
    padding: 12px 16px;
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 500;
    color: white;
    background: var(--danger);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .restore-confirm-btn:hover {
    opacity: 0.9;
  }

  .setting-desc {
    font-size: 11px;
    color: var(--text-muted);
    margin: 2px 0 0;
  }

  .setting-action-btn {
    padding: 6px 14px;
    border-radius: 8px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }

  .setting-danger-label { color: var(--danger, #EF4444); }

  .setting-danger-btn {
    padding: 6px 14px;
    border-radius: 8px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    border: 1px solid rgba(239,68,68,0.3);
    background: rgba(239,68,68,0.08);
    color: var(--danger, #EF4444);
    white-space: nowrap;
  }

  .danger-confirm-box {
    background: rgba(239,68,68,0.05);
    border-radius: 10px;
    padding: 14px;
    border: 1px solid rgba(239,68,68,0.15);
    margin: 8px 0;
  }

  .danger-confirm-text {
    font-size: 13px;
    color: var(--danger, #EF4444);
    margin: 0 0 10px;
    font-weight: 500;
  }

  .danger-confirm-actions {
    display: flex;
    gap: 8px;
  }
</style>
