<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { fly, fade } from "svelte/transition";
  import { addToast } from "./toast";
  import { pluginEngine } from "./pluginEngine";
  import { unloadPlugin, type PluginManifest } from "./pluginLoader";

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  let plugins = $state<PluginManifest[]>([]);
  let loading = $state(true);
  let showHelp = $state(false);
  let expandedErrors = $state<Set<string>>(new Set());

  // Load plugins on mount
  $effect(() => {
    loadPlugins();
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

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }

  function getPluginStatus(plugin: PluginManifest): 'enabled' | 'disabled' | 'error' {
    if (plugin.load_error) return 'error';
    if (plugin.enabled) return 'enabled';
    return 'disabled';
  }

  function getStatusDot(status: 'enabled' | 'disabled' | 'error'): string {
    switch (status) {
      case 'enabled': return 'bg-green-500';
      case 'disabled': return 'bg-gray-400';
      case 'error': return 'bg-red-500';
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div 
  class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
  onclick={handleBackdropClick}
  transition:fade={{ duration: 150 }}
>
  <div 
    class="bg-bg-card border border-border rounded-2xl shadow-xl w-full max-w-2xl max-h-[80vh] flex flex-col"
    transition:fly={{ y: 20, duration: 200 }}
  >
    <!-- Header -->
    <div class="flex items-center justify-between px-6 py-4 border-b border-border">
      <div class="flex items-center gap-3">
        <h2 class="text-xl font-semibold text-text-primary">Plugins</h2>
        <button
          class="w-6 h-6 flex items-center justify-center rounded-full bg-bg-subtle text-text-secondary hover:text-text-primary hover:bg-border transition-colors"
          onclick={() => showHelp = !showHelp}
          title="Help"
        >
          ?
        </button>
      </div>
      <button
        class="w-8 h-8 flex items-center justify-center rounded-lg hover:bg-bg-subtle text-text-secondary hover:text-text-primary transition-colors"
        onclick={onClose}
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>

    <!-- Help Section -->
    {#if showHelp}
      <div class="px-6 py-4 bg-bg-subtle border-b border-border">
        <div class="space-y-3 text-sm text-text-secondary">
          <p>
            <strong class="text-text-primary">How to install plugins:</strong> Drop a plugin folder 
            into the plugins directory and restart the app.
          </p>
          <p>
            <strong class="text-text-primary">Plugin manifest (manifest.json):</strong>
          </p>
          <pre class="bg-bg-base p-3 rounded-lg overflow-x-auto text-xs font-mono">{`{
  "id": "my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "description": "What it does",
  "entry_point": "index.js",
  "hooks": ["card:render:front"]
}`}</pre>
          <p class="text-danger">
            <strong>Security warning:</strong> Plugins have full access to your card data and the app. 
            Only install plugins from sources you trust.
          </p>
        </div>
      </div>
    {/if}

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-6">
      <!-- Plugin Directory Info -->
      <div class="flex items-center justify-between mb-4 text-sm">
        <span class="text-text-secondary">
          Plugins are loaded from: 
          <code class="bg-bg-subtle px-2 py-0.5 rounded text-xs">~/.local/share/anki-wrapper/plugins/</code>
        </span>
        <button
          class="px-3 py-1.5 rounded-lg bg-accent text-white text-sm hover:bg-accent/90 transition-colors"
          onclick={openPluginsFolder}
        >
          Open Folder
        </button>
      </div>

      {#if loading}
        <div class="flex items-center justify-center py-12">
          <div class="animate-spin w-8 h-8 border-2 border-accent border-t-transparent rounded-full"></div>
        </div>
      {:else if plugins.length === 0}
        <!-- Empty State -->
        <div class="text-center py-12">
          <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-bg-subtle flex items-center justify-center">
            <svg class="w-8 h-8 text-text-secondary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
            </svg>
          </div>
          <h3 class="text-lg font-medium text-text-primary mb-2">No plugins installed</h3>
          <p class="text-text-secondary text-sm mb-4">
            Drop a plugin folder into the plugins directory to get started.
          </p>
          <button
            class="px-4 py-2 rounded-lg bg-accent text-white text-sm hover:bg-accent/90 transition-colors"
            onclick={openPluginsFolder}
          >
            Open Plugins Folder
          </button>
        </div>
      {:else}
        <!-- Plugin List -->
        <div class="space-y-3">
          {#each plugins as plugin (plugin.id)}
            {@const status = getPluginStatus(plugin)}
            {@const runtimeErrors = (plugin as any).runtimeErrors || []}
            {@const hasErrors = !!plugin.load_error || runtimeErrors.length > 0}
            
            <div class="bg-bg-base border border-border rounded-xl p-4 hover:border-accent/50 transition-colors">
              <div class="flex items-start justify-between gap-4">
                <!-- Status indicator and info -->
                <div class="flex items-start gap-3 flex-1 min-w-0">
                  <!-- Status dot -->
                  <div class="mt-1.5 w-2.5 h-2.5 rounded-full {getStatusDot(status)} flex-shrink-0"></div>
                  
                  <div class="flex-1 min-w-0">
                    <!-- Name and version -->
                    <div class="flex items-center gap-2 flex-wrap">
                      <span class="font-medium text-text-primary">{plugin.name}</span>
                      <span class="text-xs text-text-secondary">v{plugin.version}</span>
                      {#if status === 'error'}
                        <span class="text-xs text-danger font-medium">ERROR</span>
                      {/if}
                    </div>
                    
                    <!-- Description -->
                    <p class="text-sm text-text-secondary mt-0.5">
                      {#if plugin.load_error}
                        <span class="text-danger">{plugin.load_error}</span>
                      {:else}
                        {plugin.description}
                      {/if}
                    </p>
                    
                    <!-- Hooks (if no load error) -->
                    {#if !plugin.load_error && plugin.hooks && plugin.hooks.length > 0}
                      <p class="text-xs text-text-secondary mt-1">
                        Hooks: {plugin.hooks.join(', ')}
                      </p>
                    {/if}
                    
                    <!-- Author -->
                    {#if plugin.author}
                      <p class="text-xs text-text-secondary mt-1">
                        Author: {plugin.author}
                      </p>
                    {/if}
                    
                    <!-- Runtime Errors -->
                    {#if runtimeErrors.length > 0}
                      <div class="mt-2">
                        <button
                          class="text-xs text-danger hover:underline flex items-center gap-1"
                          onclick={() => toggleErrorExpand(plugin.id)}
                        >
                          <span>⚠ {runtimeErrors.length} runtime error(s)</span>
                          <svg 
                            class="w-3 h-3 transition-transform" 
                            class:rotate-180={expandedErrors.has(plugin.id)}
                            fill="none" 
                            stroke="currentColor" 
                            viewBox="0 0 24 24"
                          >
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                          </svg>
                        </button>
                        
                        {#if expandedErrors.has(plugin.id)}
                          <div class="mt-2 p-2 bg-danger/10 rounded-lg border border-danger/20">
                            {#each runtimeErrors as error}
                              <p class="text-xs font-mono text-danger break-all">{error}</p>
                            {/each}
                          </div>
                        {/if}
                      </div>
                    {/if}
                  </div>
                </div>
                
                <!-- Toggle switch -->
                <div class="flex-shrink-0">
                  {#if status === 'error'}
                    <span class="text-xs text-text-secondary px-2 py-1 bg-bg-subtle rounded">
                      Cannot enable
                    </span>
                  {:else}
                    <button
                      class="relative w-12 h-6 rounded-full transition-colors {plugin.enabled ? 'bg-green-500' : 'bg-gray-400'}"
                      onclick={() => togglePlugin(plugin.id, plugin.enabled)}
                      disabled={status === 'error'}
                    >
                      <span 
                        class="absolute top-1 w-4 h-4 bg-white rounded-full transition-transform {plugin.enabled ? 'left-7' : 'left-1'}"
                      ></span>
                    </button>
                  {/if}
                </div>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Footer -->
    <div class="px-6 py-4 border-t border-border flex items-center justify-between">
      <span class="text-sm text-text-secondary">
        Plugin API version: 1.0.0
      </span>
      <button
        class="px-4 py-2 rounded-lg bg-bg-subtle text-text-primary hover:bg-border transition-colors"
        onclick={onClose}
      >
        Close
      </button>
    </div>
  </div>
</div>
