<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { addToast } from "./toast";
  import { pluginEngine } from "./pluginEngine";
  import { unloadPlugin, type PluginManifest } from "./pluginLoader";
  import NeuDialog from "./ui/NeuDialog.svelte";
  import NeuToggle from "./ui/NeuToggle.svelte";

  interface Props {
    isOpen: boolean;
    onClose: () => void;
  }

  let { isOpen, onClose }: Props = $props();

  let plugins = $state<PluginManifest[]>([]);
  let loading = $state(true);
  let expandedErrors = $state<Set<string>>(new Set());

  // Load plugins on mount
  $effect(() => {
    if (isOpen) {
      loadPlugins();
    }
  });

  async function loadPlugins() {
    loading = true;
    try {
      plugins = await invoke<PluginManifest[]>("get_installed_plugins");
      
      // Merge runtime error info from pluginEngine
      for (const p of plugins) {
        const errors = pluginEngine.getPluginErrors(p.id);
        if (errors.length > 0) {
          (p as any).runtimeErrors = errors;
        }
      }
    } catch (e) {
      console.error("Failed to load plugins:", e);
      addToast(`Failed to load plugins: ${e}`, "error");
    } finally {
      loading = false;
    }
  }

  async function togglePlugin(pluginId: string, currentlyEnabled: boolean) {
    try {
      if (currentlyEnabled) {
        await invoke("disable_plugin", { pluginId });
        await unloadPlugin(pluginId);
        addToast(`${pluginId} disabled. Changes take effect immediately.`, "warning");
      } else {
        await invoke("enable_plugin", { pluginId });
        addToast(`${pluginId} enabled. Restart the app to load it.`, "success");
      }
      // Refresh the list
      await loadPlugins();
    } catch (e) {
      addToast(`Failed to toggle plugin: ${e}`, "error");
    }
  }

  async function uninstallPlugin(pluginId: string) {
    if (!confirm(`Uninstall plugin "${pluginId}"?`)) return;
    try {
      await invoke("uninstall_plugin", { pluginId });
      addToast(`${pluginId} uninstalled.`, "success");
      await loadPlugins();
    } catch (e) {
      addToast(`Failed to uninstall plugin: ${e}`, "error");
    }
  }

  async function openPluginsFolder() {
    try {
      await invoke("open_plugins_folder");
    } catch (e) {
      addToast(`Failed to open folder: ${e}`, "error");
    }
  }

  function toggleErrorExpand(pluginId: string) {
    const newSet = new Set(expandedErrors);
    if (newSet.has(pluginId)) {
      newSet.delete(pluginId);
    } else {
      newSet.add(pluginId);
    }
    expandedErrors = newSet;
  }

  function getPluginStatus(plugin: PluginManifest): 'enabled' | 'disabled' | 'error' {
    if (plugin.load_error) return 'error';
    if (plugin.enabled) return 'enabled';
    return 'disabled';
  }
</script>

<NeuDialog {isOpen} {onClose} title="Plugins" size="lg">
  <div class="plugin-manager-content">
    {#if loading}
      <div class="loading-state">
        <div class="spinner"></div>
      </div>
    {:else if plugins.length === 0}
      <!-- Empty State -->
      <div class="empty-state">
        <div class="empty-icon">
          <svg class="empty-icon-svg" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
          </svg>
        </div>
        <h3 class="empty-title">No plugins installed</h3>
        <p class="empty-description">
          Drop a plugin folder into the plugins directory to get started.
        </p>
        <button
          class="open-folder-btn"
          onclick={openPluginsFolder}
        >
          Open Plugins Folder
        </button>
      </div>
    {:else}
      <!-- Plugin List -->
      <div class="plugin-list">
        {#each plugins as plugin (plugin.id)}
          {@const status = getPluginStatus(plugin)}
          {@const runtimeErrors = (plugin as any).runtimeErrors || []}
          {@const hasErrors = !!plugin.load_error || runtimeErrors.length > 0}
          
          <div class="plugin-card neu-raised">
            <div class="plugin-header">
              <div class="plugin-info">
                <div class="plugin-name-row">
                  <span class="plugin-name">{plugin.name}</span>
                  <span class="plugin-version">v{plugin.version}</span>
                  {#if status === 'error'}
                    <span class="plugin-error-badge">ERROR</span>
                  {/if}
                </div>
                
                <p class="plugin-description">
                  {#if plugin.load_error}
                    <span class="error-text">{plugin.load_error}</span>
                  {:else}
                    {plugin.description}
                  {/if}
                </p>
                
                {#if plugin.author}
                  <p class="plugin-author">Author: {plugin.author}</p>
                {/if}
                
                <!-- Runtime Errors -->
                {#if runtimeErrors.length > 0}
                  <div class="runtime-errors">
                    <button
                      class="error-toggle-btn"
                      onclick={() => toggleErrorExpand(plugin.id)}
                      aria-expanded={expandedErrors.has(plugin.id)}
                      aria-label="Toggle error details"
                    >
                      <span>⚠ {runtimeErrors.length} runtime error(s)</span>
                      <svg 
                        class="error-toggle-icon" 
                        class:expanded={expandedErrors.has(plugin.id)}
                        fill="none" 
                        stroke="currentColor" 
                        viewBox="0 0 24 24"
                      >
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                      </svg>
                    </button>
                    
                    {#if expandedErrors.has(plugin.id)}
                      <div class="error-details">
                        {#each runtimeErrors as error}
                          <p class="error-message">{error}</p>
                        {/each}
                      </div>
                    {/if}
                  </div>
                {/if}
              </div>
              
              <div class="plugin-actions">
                {#if status === 'error'}
                  <span class="cannot-enable">Cannot enable</span>
                {:else}
                  <NeuToggle
                    checked={plugin.enabled}
                    onchange={() => togglePlugin(plugin.id, plugin.enabled)}
                  />
                {/if}
                
                <button
                  class="uninstall-btn neu-subtle neu-btn"
                  onclick={() => uninstallPlugin(plugin.id)}
                  title="Uninstall plugin"
                  aria-label="Uninstall plugin"
                >
                  <svg class="uninstall-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}

    <!-- Install Plugin Section -->
    <div class="install-section neu-pressed">
      <div class="install-content">
        <span class="install-label">Install Plugin</span>
        <p class="install-description">
          Drop a plugin folder into the plugins directory or use the button below.
        </p>
      </div>
      <button
        class="install-btn neu-subtle neu-btn"
        onclick={openPluginsFolder}
      >
        Open Plugins Folder
      </button>
    </div>
  </div>
</NeuDialog>

<style>
  .plugin-manager-content {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-height: 60vh;
    overflow-y: auto;
    padding-right: 8px;
  }

  .loading-state {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 48px 0;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 2px solid var(--accent);
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 48px 0;
    text-align: center;
  }

  .empty-icon {
    width: 64px;
    height: 64px;
    margin-bottom: 16px;
    border-radius: 50%;
    background: var(--bg-subtle);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .empty-icon-svg {
    width: 32px;
    height: 32px;
    color: var(--text-secondary);
  }

  .empty-title {
    font-family: var(--sans);
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 8px 0;
  }

  .empty-description {
    font-family: var(--sans);
    font-size: 14px;
    color: var(--text-muted);
    margin: 0 0 16px 0;
  }

  .open-folder-btn {
    padding: 10px 20px;
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

  .open-folder-btn:hover {
    opacity: 0.9;
  }

  .plugin-list {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .plugin-card {
    padding: 20px 24px;
  }

  .plugin-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }

  .plugin-info {
    flex: 1;
    min-width: 0;
  }

  .plugin-name-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 4px;
  }

  .plugin-name {
    font-family: var(--sans);
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .plugin-version {
    font-family: var(--sans);
    font-size: 11px;
    color: var(--text-muted);
  }

  .plugin-error-badge {
    font-family: var(--sans);
    font-size: 11px;
    font-weight: 600;
    color: var(--danger);
  }

  .plugin-description {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-secondary);
    margin: 4px 0 0 0;
    line-height: 1.5;
  }

  .error-text {
    color: var(--danger);
  }

  .plugin-author {
    font-family: var(--sans);
    font-size: 11px;
    color: var(--text-muted);
    margin: 4px 0 0 0;
  }

  .runtime-errors {
    margin-top: 8px;
  }

  .error-toggle-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    font-family: var(--sans);
    font-size: 12px;
    color: var(--danger);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
  }

  .error-toggle-btn:hover {
    text-decoration: underline;
  }

  .error-toggle-icon {
    width: 12px;
    height: 12px;
    transition: transform 0.2s ease;
  }

  .error-toggle-icon.expanded {
    transform: rotate(180deg);
  }

  .error-details {
    margin-top: 8px;
    padding: 8px;
    background: color-mix(in srgb, var(--danger) 10%, transparent);
    border-radius: var(--radius-sm);
    border: 1px solid color-mix(in srgb, var(--danger) 20%, transparent);
  }

  .error-message {
    font-family: monospace;
    font-size: 11px;
    color: var(--danger);
    margin: 0;
    word-break: break-all;
  }

  .plugin-actions {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-shrink: 0;
  }

  .cannot-enable {
    font-family: var(--sans);
    font-size: 12px;
    color: var(--text-secondary);
    padding: 4px 8px;
    background: var(--bg-subtle);
    border-radius: var(--radius-sm);
  }

  .uninstall-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    border: none;
    cursor: pointer;
    color: var(--danger);
  }

  .uninstall-btn:hover {
    background: color-mix(in srgb, var(--danger) 10%, transparent);
  }

  .uninstall-icon {
    width: 16px;
    height: 16px;
  }

  .install-section {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    gap: 16px;
  }

  .install-content {
    flex: 1;
  }

  .install-label {
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    display: block;
    margin-bottom: 4px;
  }

  .install-description {
    font-family: var(--sans);
    font-size: 12px;
    color: var(--text-secondary);
    margin: 0;
  }

  .install-btn {
    padding: 10px 16px;
    font-family: var(--sans);
    font-size: 13px;
    font-weight: 500;
    color: var(--accent);
    border: none;
    cursor: pointer;
    white-space: nowrap;
  }

  .install-btn:hover {
    background: var(--accent-soft);
  }
</style>
