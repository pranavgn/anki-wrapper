// src/lib/chartTheme.ts

function getCSSVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

function hexToRgba(hex: string, alpha: number): string {
  const h = hex.replace('#', '');
  const r = parseInt(h.substring(0, 2), 16);
  const g = parseInt(h.substring(2, 4), 16);
  const b = parseInt(h.substring(4, 6), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

export function getChartColors() {
  const accent = getCSSVar('--accent') || '#C4714F';
  const textPrimary = getCSSVar('--text-primary') || '#2C2825';
  const textSecondary = getCSSVar('--text-secondary') || '#8C8279';
  const isDark = document.documentElement.classList.contains('dark');

  return {
    accent,
    accentAlpha: (a: number) => hexToRgba(accent, a),
    newCards: hexToRgba(accent, 0.8),
    learning: isDark ? 'rgba(134,176,140,0.75)' : 'rgba(107,143,113,0.7)',
    young: isDark ? 'rgba(216,174,99,0.8)' : 'rgba(196,154,79,0.75)',
    mature: hexToRgba(accent, 0.45),
    again: isDark ? '#F87171' : '#EF4444',
    hard: isDark ? '#FBBF24' : '#F59E0B',
    good: isDark ? '#60A5FA' : '#3B82F6',
    easy: isDark ? '#34D399' : '#10B981',
    textPrimary,
    textSecondary,
    gridColor: isDark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.06)',
    tooltipBg: isDark ? '#1C1917' : '#2C2825',
    tooltipText: '#FFFFFF',
    cardBg: isDark ? '#292524' : '#EDE8E0',
  };
}

export type ChartColorPalette = ReturnType<typeof getChartColors>;

export function baseBarOptions(colors: ChartColorPalette) {
  return {
    responsive: true,
    maintainAspectRatio: false,
    scales: {
      x: {
        grid: { display: false },
        ticks: { color: colors.textSecondary, font: { size: 10, family: 'DM Sans, sans-serif' } },
      },
      y: {
        grid: { color: colors.gridColor },
        ticks: { color: colors.textSecondary, font: { family: 'DM Sans, sans-serif' } },
      },
    },
    plugins: {
      legend: { display: false },
      tooltip: {
        backgroundColor: colors.tooltipBg,
        titleColor: colors.tooltipText,
        bodyColor: colors.tooltipText,
        borderColor: 'transparent',
        borderWidth: 0,
        cornerRadius: 10,
        padding: 10,
        titleFont: { family: 'DM Sans, sans-serif' },
        bodyFont: { family: 'DM Sans, sans-serif' },
      },
    },
  };
}

export function baseDoughnutOptions(colors: ChartColorPalette) {
  return {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: { display: false },
      tooltip: {
        backgroundColor: colors.tooltipBg,
        titleColor: colors.tooltipText,
        bodyColor: colors.tooltipText,
        cornerRadius: 8,
        padding: 10,
      },
    },
  };
}

// Invalidate on theme change
let _cached: ChartColorPalette | null = null;
export function getColors(force = false): ChartColorPalette {
  if (!_cached || force) _cached = getChartColors();
  return _cached;
}

if (typeof window !== 'undefined') {
  new MutationObserver(() => { _cached = null; })
    .observe(document.documentElement, { attributes: true, attributeFilter: ['class', 'style'] });
}
