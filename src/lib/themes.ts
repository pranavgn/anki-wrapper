export interface ThemePreset {
  id: string;
  name: string;
  description: string;
  variables: Record<string, string>;
  darkOverrides?: Record<string, string>;
}

export const ACCENT_PRESETS = [
  { id: 'terracotta', name: 'Terracotta', color: '#C4714F' },
  { id: 'ocean',      name: 'Ocean',      color: '#3B82F6' },
  { id: 'forest',     name: 'Forest',     color: '#22C55E' },
  { id: 'lavender',   name: 'Lavender',   color: '#8B5CF6' },
  { id: 'rose',       name: 'Rose',       color: '#EC4899' },
  { id: 'slate',      name: 'Slate',      color: '#64748B' },
] as const;

export type AccentPresetId = typeof ACCENT_PRESETS[number]['id'];

export const DESIGN_PRESETS: ThemePreset[] = [
  {
    id: 'neumorphic',
    name: 'Neumorphic',
    description: 'Soft, tactile shadows',
    variables: {},
    darkOverrides: {},
  },
  {
    id: 'clean',
    name: 'Clean',
    description: 'Flat, minimal borders',
    variables: {
      '--neu-up': '0 1px 3px rgba(0,0,0,0.08), 0 1px 2px rgba(0,0,0,0.06)',
      '--neu-down': 'inset 0 1px 2px rgba(0,0,0,0.06)',
      '--neu-subtle': '0 1px 2px rgba(0,0,0,0.05)',
      '--radius-sm': '8px',
      '--radius-md': '12px',
      '--radius-lg': '16px',
    },
    darkOverrides: {
      '--neu-up': '0 1px 3px rgba(0,0,0,0.2), 0 1px 2px rgba(0,0,0,0.15)',
      '--neu-down': 'inset 0 1px 2px rgba(0,0,0,0.2)',
      '--neu-subtle': '0 1px 2px rgba(0,0,0,0.15)',
    },
  },
  {
    id: 'glassmorphic',
    name: 'Glass',
    description: 'Frosted translucency',
    variables: {
      '--neu-up': '0 4px 16px rgba(0,0,0,0.08)',
      '--neu-down': 'inset 0 1px 3px rgba(0,0,0,0.1)',
      '--neu-subtle': '0 2px 8px rgba(0,0,0,0.06)',
      '--bg-card': 'rgba(247, 244, 239, 0.7)',
      '--bg-card-raised': 'rgba(250, 248, 244, 0.8)',
    },
    darkOverrides: {
      '--neu-up': '0 4px 16px rgba(0,0,0,0.25)',
      '--neu-down': 'inset 0 1px 3px rgba(0,0,0,0.25)',
      '--neu-subtle': '0 2px 8px rgba(0,0,0,0.2)',
      '--bg-card': 'rgba(41, 37, 36, 0.7)',
      '--bg-card-raised': 'rgba(46, 42, 39, 0.8)',
    },
  },
];

/** Compute a soft accent background tint from a hex color */
export function computeAccentSoft(hex: string): string {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return `rgba(${r}, ${g}, ${b}, 0.1)`;
}

/** CSS custom properties that design presets may override */
const OVERRIDABLE_PROPS = [
  '--neu-up', '--neu-down', '--neu-subtle',
  '--radius-sm', '--radius-md', '--radius-lg',
  '--bg-card', '--bg-card-raised',
];

/**
 * Apply a design preset + accent color to :root.
 * Call this after toggling .dark class on documentElement.
 */
export function applyDesignPreset(
  presetId: string,
  accentColor: string,
  isDark: boolean
): void {
  const root = document.documentElement;
  const preset = DESIGN_PRESETS.find(p => p.id === presetId);

  // 1. Clear previous overrides so CSS defaults reassert
  OVERRIDABLE_PROPS.forEach(p => root.style.removeProperty(p));

  // 2. Apply preset variables (merge dark overrides if dark mode)
  if (preset) {
    const vars = isDark && preset.darkOverrides
      ? { ...preset.variables, ...preset.darkOverrides }
      : preset.variables;
    Object.entries(vars).forEach(([key, val]) => {
      root.style.setProperty(key, val);
    });
  }

  // 3. Apply accent color
  root.style.setProperty('--accent', accentColor);
  root.style.setProperty('--accent-soft', computeAccentSoft(accentColor));
}
