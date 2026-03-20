import { invoke } from "@tauri-apps/api/core";
import { pluginEngine, setCurrentLoadingPlugin, clearCurrentLoadingPlugin } from "./pluginEngine";

/**
 * Plugin manifest type matching the Rust backend
 */
export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  entry_point: string;
  hooks?: string[];
  author?: string;
  homepage?: string;
  min_api_version?: string;
  enabled: boolean;
  path: string;
  has_css: boolean;
  load_error?: string;
}

/**
 * Load all enabled plugins from the plugins directory
 * This scans the directory, validates manifests, loads JS, and injects CSS
 */
export async function loadAllPlugins(): Promise<void> {
  let manifests: PluginManifest[];
  
  try {
    manifests = await invoke<PluginManifest[]>("get_installed_plugins");
  } catch (e) {
    console.error("Failed to scan plugins:", e);
    return;
  }

  console.log(`Found ${manifests.length} plugins`);

  for (const manifest of manifests) {
    // Skip disabled plugins
    if (!manifest.enabled) {
      console.log(`Plugin "${manifest.name}" is disabled, skipping`);
      continue;
    }

    // Skip plugins with load errors
    if (manifest.load_error) {
      console.warn(`Plugin "${manifest.id}" has load error: ${manifest.load_error}`);
      pluginEngine.recordError?.(manifest.id, manifest.load_error);
      continue;
    }

    try {
      // Load CSS first if present
      if (manifest.has_css) {
        try {
          const css = await invoke<string>("get_plugin_css", { pluginId: manifest.id });
          injectPluginCSS(manifest.id, css);
        } catch (cssError) {
          console.warn(`Failed to load CSS for plugin "${manifest.id}":`, cssError);
        }
      }

      // Get JS source from backend
      const source = await invoke<string>("get_plugin_source", { pluginId: manifest.id });

      // Register plugin in engine BEFORE evaluating script
      pluginEngine.registerPlugin(manifest);

      // Set the loading context so hooks auto-associate with this plugin
      setCurrentLoadingPlugin(manifest.id);

      // Evaluate the plugin JS in a Function constructor (NOT eval)
      // This gives us a clean scope but still access to window.__ankiPlugins
      const pluginFn = new Function(
        "__ankiPlugins", // parameter name the plugin receives
        source // plugin source code
      );
      pluginFn(window.__ankiPlugins);

      // Clear loading context
      clearCurrentLoadingPlugin();

      console.log(`Plugin loaded: ${manifest.name} v${manifest.version}`);

    } catch (e) {
      // Clear loading context on error
      clearCurrentLoadingPlugin();
      
      const errorMessage = e instanceof Error ? e.message : String(e);
      console.error(`Failed to load plugin "${manifest.id}":`, e);
      
      // Record the error in the plugin engine if the method exists
      if (pluginEngine.recordError) {
        pluginEngine.recordError(manifest.id, `Load failed: ${errorMessage}`);
      }
    }
  }
}

/**
 * Unload a specific plugin - removes hooks and CSS
 */
export async function unloadPlugin(pluginId: string): Promise<void> {
  // Remove all hooks registered by this plugin
  pluginEngine.unregisterAll(pluginId);

  // Remove injected CSS
  removePluginCSS(pluginId);

  console.log(`Plugin unloaded: ${pluginId}`);
}

/**
 * Enable a plugin (removes from disabled list)
 */
export async function enablePlugin(pluginId: string): Promise<void> {
  await invoke("enable_plugin", { pluginId });
  console.log(`Plugin enabled: ${pluginId}`);
}

/**
 * Disable a plugin (adds to disabled list and unloads)
 */
export async function disablePlugin(pluginId: string): Promise<void> {
  // First unload the plugin to remove hooks
  await unloadPlugin(pluginId);
  
  // Then disable it in the backend
  await invoke("disable_plugin", { pluginId });
  console.log(`Plugin disabled: ${pluginId}`);
}

/**
 * Reload a plugin - unloads and loads again
 */
export async function reloadPlugin(pluginId: string): Promise<void> {
  await unloadPlugin(pluginId);
  
  // Get fresh manifest
  const manifests = await invoke<PluginManifest[]>("get_installed_plugins");
  const manifest = manifests.find(m => m.id === pluginId);
  
  if (manifest && manifest.enabled && !manifest.load_error) {
    await loadSinglePlugin(manifest);
  }
}

/**
 * Load a single plugin (internal use)
 */
async function loadSinglePlugin(manifest: PluginManifest): Promise<void> {
  try {
    // Load CSS first if present
    if (manifest.has_css) {
      try {
        const css = await invoke<string>("get_plugin_css", { pluginId: manifest.id });
        injectPluginCSS(manifest.id, css);
      } catch (cssError) {
        console.warn(`Failed to load CSS for plugin "${manifest.id}":`, cssError);
      }
    }

    // Get JS source from backend
    const source = await invoke<string>("get_plugin_source", { pluginId: manifest.id });

    // Register plugin in engine
    pluginEngine.registerPlugin(manifest);

    // Set the loading context
    setCurrentLoadingPlugin(manifest.id);

    // Evaluate the plugin JS
    const pluginFn = new Function("__ankiPlugins", source);
    pluginFn(window.__ankiPlugins);

    // Clear loading context
    clearCurrentLoadingPlugin();

    console.log(`Plugin reloaded: ${manifest.name} v${manifest.version}`);

  } catch (e) {
    clearCurrentLoadingPlugin();
    const errorMessage = e instanceof Error ? e.message : String(e);
    console.error(`Failed to reload plugin "${manifest.id}":`, e);
    
    if (pluginEngine.recordError) {
      pluginEngine.recordError(manifest.id, `Reload failed: ${errorMessage}`);
    }
  }
}

/**
 * Inject plugin CSS into the document head
 */
function injectPluginCSS(pluginId: string, css: string): void {
  // Remove existing styles for this plugin first
  removePluginCSS(pluginId);

  const style = document.createElement("style");
  style.dataset.pluginId = pluginId;
  style.textContent = css;
  document.head.appendChild(style);
  
  console.log(`Plugin CSS injected: ${pluginId}`);
}

/**
 * Remove plugin CSS from the document head
 */
function removePluginCSS(pluginId: string): void {
  const styleEl = document.querySelector(`style[data-plugin-id="${pluginId}"]`);
  if (styleEl) {
    styleEl.remove();
  }
}
