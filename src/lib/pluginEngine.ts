// Types for the plugin hook system

export interface FilterHookPayload {
  [key: string]: any;
}

export interface PluginRegistration {
  pluginId: string;
  hookName: string;
  callback: Function;
  priority: number; // lower runs first, default 10
}

export interface PluginManifest {
  id: string; // unique, e.g. "furigana-injector"
  name: string; // display name, e.g. "Furigana Injector"
  version: string; // semver, e.g. "1.0.0"
  description: string;
  entry_point: string; // JS filename relative to plugin dir, e.g. "index.js"
  hooks?: string[]; // optional: declared hooks this plugin uses (for UI display)
  author?: string;
  homepage?: string;
}

/**
 * PluginEngine - Core runtime for plugin hook system
 * 
 * Manages registration and execution of filter and action hooks.
 * Ensures complete error isolation - a plugin crash never breaks the app.
 */
class PluginEngine {
  private filterHooks: Map<string, PluginRegistration[]> = new Map();
  private actionHooks: Map<string, PluginRegistration[]> = new Map();
  private loadedPlugins: Map<string, PluginManifest> = new Map();
  private pluginErrors: Map<string, string[]> = new Map();

  /**
   * Register a filter hook - transforms data through a pipeline
   * Each callback's output becomes the next callback's input
   */
  registerFilter(
    pluginId: string,
    hookName: string,
    callback: (data: any) => any,
    priority: number = 10
  ): void {
    // Validate plugin is loaded
    if (!this.loadedPlugins.has(pluginId)) {
      throw new Error(`Plugin "${pluginId}" is not loaded. Cannot register filter hook.`);
    }

    const registrations = this.filterHooks.get(hookName) || [];
    
    const registration: PluginRegistration = {
      pluginId,
      hookName,
      callback,
      priority
    };

    registrations.push(registration);
    
    // Sort by priority (lower = runs first)
    registrations.sort((a, b) => a.priority - b.priority);
    
    this.filterHooks.set(hookName, registrations);
  }

  /**
   * Register an action hook - fire-and-forget side effects
   * All callbacks run in parallel, errors are isolated per-callback
   */
  registerAction(
    pluginId: string,
    hookName: string,
    callback: (data: any) => void,
    priority: number = 10
  ): void {
    // Validate plugin is loaded
    if (!this.loadedPlugins.has(pluginId)) {
      throw new Error(`Plugin "${pluginId}" is not loaded. Cannot register action hook.`);
    }

    const registrations = this.actionHooks.get(hookName) || [];
    
    const registration: PluginRegistration = {
      pluginId,
      hookName,
      callback,
      priority
    };

    registrations.push(registration);
    
    // Sort by priority (lower = runs first)
    registrations.sort((a, b) => a.priority - b.priority);
    
    this.actionHooks.set(hookName, registrations);
  }

  /**
   * Unregister all hooks for a specific plugin
   * Called when disabling a plugin
   */
  unregisterAll(pluginId: string): void {
    // Remove from filter hooks
    for (const [hookName, registrations] of this.filterHooks.entries()) {
      const filtered = registrations.filter(r => r.pluginId !== pluginId);
      if (filtered.length === 0) {
        this.filterHooks.delete(hookName);
      } else {
        this.filterHooks.set(hookName, filtered);
      }
    }

    // Remove from action hooks
    for (const [hookName, registrations] of this.actionHooks.entries()) {
      const filtered = registrations.filter(r => r.pluginId !== pluginId);
      if (filtered.length === 0) {
        this.actionHooks.delete(hookName);
      } else {
        this.actionHooks.set(hookName, filtered);
      }
    }

    // Clear errors for this plugin
    this.pluginErrors.delete(pluginId);
  }

  /**
   * Run a filter hook - data flows through callbacks in priority order
   * Each callback receives the output of the previous one
   * If a callback throws, log the error and continue with previous data
   */
  async runFilter(hookName: string, data: any): Promise<any> {
    const registrations = this.filterHooks.get(hookName) || [];
    
    let result = { ...data }; // Create a copy to avoid mutation

    for (const registration of registrations) {
      try {
        result = await registration.callback(result);
      } catch (error) {
        // Log error but continue with previous result
        const errorMessage = error instanceof Error ? error.message : String(error);
        this.logError(registration.pluginId, `Hook "${hookName}": ${errorMessage}`);
        
        // Continue with the result as it was before this callback
        // Don't update result - it stays as the previous valid state
      }
    }

    return result;
  }

  /**
   * Run an action hook - fire all callbacks, errors are isolated
   * All callbacks run, one failure doesn't stop others
   */
  async runAction(hookName: string, data: any): Promise<void> {
    const registrations = this.actionHooks.get(hookName) || [];

    // Run all callbacks in parallel
    const promises = registrations.map(async (registration) => {
      try {
        await registration.callback(data);
      } catch (error) {
        // Log error but don't let it propagate
        const errorMessage = error instanceof Error ? error.message : String(error);
        this.logError(registration.pluginId, `Hook "${hookName}": ${errorMessage}`);
      }
    });

    // Wait for all to complete (or fail gracefully)
    await Promise.all(promises);
  }

  /**
   * Register a plugin manifest - marks plugin as loaded
   */
  registerPlugin(manifest: PluginManifest): void {
    this.loadedPlugins.set(manifest.id, manifest);
  }

  /**
   * Get all loaded plugins
   */
  getLoadedPlugins(): PluginManifest[] {
    return Array.from(this.loadedPlugins.values());
  }

  /**
   * Get errors for a specific plugin
   */
  getPluginErrors(pluginId: string): string[] {
    return this.pluginErrors.get(pluginId) || [];
  }

  /**
   * Check if a plugin is loaded
   */
  isPluginLoaded(pluginId: string): boolean {
    return this.loadedPlugins.has(pluginId);
  }

  /**
   * Get plugin manifest by ID
   */
  getPlugin(pluginId: string): PluginManifest | undefined {
    return this.loadedPlugins.get(pluginId);
  }

  /**
   * Clear all errors for a plugin (e.g., after fixing)
   */
  clearErrors(pluginId: string): void {
    this.pluginErrors.delete(pluginId);
  }

  /**
   * Record an error for a plugin (called by plugin loader)
   */
  recordError(pluginId: string, message: string): void {
    this.logError(pluginId, message);
  }

  /**
   * Internal: Log an error for a plugin
   */
  private logError(pluginId: string, message: string): void {
    const errors = this.pluginErrors.get(pluginId) || [];
    errors.push(message);
    this.pluginErrors.set(pluginId, errors);
    
    // Also log to console for debugging
    console.error(`[Plugin "${pluginId}"]`, message);
  }
}

// Create singleton instance
const engine = new PluginEngine();

// Current loading plugin ID - set during plugin script evaluation
let currentLoadingPluginId: string | null = null;

/**
 * Set the current loading plugin ID
 * Called by plugin loader before evaluating plugin script
 */
export function setCurrentLoadingPlugin(pluginId: string): void {
  currentLoadingPluginId = pluginId;
}

/**
 * Get the current loading plugin ID
 */
export function getCurrentLoadingPlugin(): string | null {
  return currentLoadingPluginId;
}

/**
 * Clear the current loading plugin ID
 * Called after plugin script evaluation completes
 */
export function clearCurrentLoadingPlugin(): void {
  currentLoadingPluginId = null;
}

// Public API exposed to plugins via window.__ankiPlugins
// Plugins use this to register their hooks
const pluginPublicAPI = {
  /**
   * Register a filter hook for the currently loading plugin
   */
  registerFilter: (
    hookName: string, 
    callback: (data: any) => any, 
    priority?: number
  ): void => {
    if (!currentLoadingPluginId) {
      throw new Error('No plugin is currently loading. Cannot register filter hook.');
    }
    engine.registerFilter(currentLoadingPluginId, hookName, callback, priority);
  },

  /**
   * Register an action hook for the currently loading plugin
   */
  registerAction: (
    hookName: string, 
    callback: (data: any) => void, 
    priority?: number
  ): void => {
    if (!currentLoadingPluginId) {
      throw new Error('No plugin is currently loading. Cannot register action hook.');
    }
    engine.registerAction(currentLoadingPluginId, hookName, callback, priority);
  },

  /**
   * Plugin API version for compatibility checks
   */
  version: '1.0.0'
};

// Expose to window for plugins
if (typeof window !== 'undefined') {
  (window as any).__ankiPlugins = pluginPublicAPI;
}

// Internal API for app code to use
export const pluginEngine = engine;

// Typed helper functions for app code

/**
 * Filter card HTML - used in StudyView before rendering front/back
 */
export async function filterCardHtml(
  hookName: 'card:render:front' | 'card:render:back',
  html: string,
  cardId: number,
  noteId: number
): Promise<string> {
  const result = await pluginEngine.runFilter(hookName, { html, cardId, noteId });
  return result.html;
}

/**
 * Filter editor field - used in CardEditor when rendering field values
 */
export async function filterEditorField(
  html: string,
  fieldIndex: number,
  notetypeId: number
): Promise<string> {
  const result = await pluginEngine.runFilter('editor:field:render', { 
    html, 
    fieldIndex, 
    notetypeId 
  });
  return result.html;
}

/**
 * Filter search query - used before sending search to backend
 */
export async function filterSearchQuery(query: string): Promise<string> {
  const result = await pluginEngine.runFilter('search:query', { query });
  return result.query;
}

/**
 * Run review answer action hook - fired after answering a card
 */
export async function runReviewAnswerHook(
  cardId: number,
  noteId: number,
  rating: number,
  deckId: number
): Promise<void> {
  await pluginEngine.runAction('review:answer', { cardId, noteId, rating, deckId });
}

/**
 * Run review show action hook - fired when a new card is shown
 */
export async function runReviewShowHook(
  cardId: number,
  noteId: number,
  deckId: number
): Promise<void> {
  await pluginEngine.runAction('review:show', { cardId, noteId, deckId });
}

/**
 * Run app ready action hook - fired after collection is initialized
 */
export async function runAppReadyHook(): Promise<void> {
  await pluginEngine.runAction('app:ready', {});
}

/**
 * Run deck study start action hook - fired when entering study mode
 */
export async function runDeckStudyStartHook(
  deckId: number,
  deckName: string
): Promise<void> {
  await pluginEngine.runAction('deck:study:start', { deckId, deckName });
}

/**
 * Run deck study end action hook - fired when exiting study mode
 */
export async function runDeckStudyEndHook(
  deckId: number,
  reviewed: number
): Promise<void> {
  await pluginEngine.runAction('deck:study:end', { deckId, reviewed });
}

// Re-export types for external use
export type { PluginEngine as PluginEngineClass };
