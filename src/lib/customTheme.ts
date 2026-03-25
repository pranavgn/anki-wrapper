import { invoke } from "@tauri-apps/api/core";

let styleEl: HTMLStyleElement | null = null;
let scriptEl: HTMLScriptElement | null = null;

export async function loadCustomTheme(): Promise<void> {
  // Load and inject CSS
  const css = await invoke<string | null>("load_custom_theme_css");
  if (css) {
    removeCustomCSS();
    styleEl = document.createElement("style");
    styleEl.id = "custom-theme-css";
    styleEl.textContent = css;
    document.head.appendChild(styleEl);
  }

  // Load and inject JS
  const js = await invoke<string | null>("load_custom_theme_js");
  if (js) {
    removeCustomJS();
    try {
      const fn = new Function("themeAPI", js);
      fn({
        setVariable: (name: string, value: string) => {
          document.documentElement.style.setProperty(name, value);
        },
        getVariable: (name: string) => {
          return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
        },
        isDark: () => document.documentElement.classList.contains("dark"),
        onThemeChange: (cb: (isDark: boolean) => void) => {
          const obs = new MutationObserver(() => cb(document.documentElement.classList.contains("dark")));
          obs.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
          return () => obs.disconnect();
        },
      });
    } catch (e) {
      console.error("Custom theme JS error:", e);
    }
  }
}

export function removeCustomCSS(): void {
  if (styleEl) {
    styleEl.remove();
    styleEl = null;
  }
}

export function removeCustomJS(): void {
  if (scriptEl) {
    scriptEl.remove();
    scriptEl = null;
  }
}

export async function removeCustomTheme(): Promise<void> {
  await invoke("remove_custom_theme");
  removeCustomCSS();
  removeCustomJS();
}
