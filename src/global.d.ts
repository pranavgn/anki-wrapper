/**
 * Global type declarations for the Anki wrapper application
 */

declare global {
  interface Window {
    /**
     * Plugin API exposed to plugins for hook registration
     * 
     * This is the public API that plugins use to register their hooks.
     * The plugin engine automatically captures the current loading plugin ID
     * so plugins don't need to pass it themselves.
     */
    __ankiPlugins: {
      /**
       * Register a filter hook - transforms data through a pipeline
       * Each callback's output becomes the next callback's input
       * 
       * @param hookName - The name of the hook to register for
       * @param callback - Function that transforms the data
       * @param priority - Lower numbers run first (default: 10)
       */
      registerFilter: (
        hookName: string,
        callback: (data: any) => any,
        priority?: number
      ) => void;

      /**
       * Register an action hook - fire-and-forget side effects
       * All callbacks run in parallel, errors are isolated per-callback
       * 
       * @param hookName - The name of the hook to register for
       * @param callback - Function that performs side effects
       * @param priority - Lower numbers run first (default: 10)
       */
      registerAction: (
        hookName: string,
        callback: (data: any) => void,
        priority?: number
      ) => void;

      /**
       * Plugin API version for compatibility checks
       */
      version: string;
    };
  }
}

export {};
