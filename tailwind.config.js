/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{svelte,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        'bg-base': 'var(--bg-base)',
        'bg-deep': 'var(--bg-deep)',
        'bg-card': 'var(--bg-card)',
        'bg-card-raised': 'var(--bg-card-raised)',
        'bg-subtle': 'var(--bg-subtle)',
        'text-primary': 'var(--text-primary)',
        'text-secondary': 'var(--text-secondary)',
        'text-muted': 'var(--text-muted)',
        'accent': 'var(--accent)',
        'accent-soft': 'var(--accent-soft)',
        'success': 'var(--success)',
        'danger': 'var(--danger)',
        'warning': 'var(--warning)',
        'border': 'var(--border)',
      },
      fontFamily: {
        'sans': ['DM Sans', 'system-ui', '-apple-system', 'sans-serif'],
        'serif': ['Source Serif 4', 'Georgia', 'Palatino Linotype', 'serif'],
      },
      boxShadow: {
        'warm': '0 4px 24px rgba(0,0,0,0.06)',
        'neu-up': 'var(--neu-up)',
        'neu-down': 'var(--neu-down)',
        'neu-subtle': 'var(--neu-subtle)',
      },
      borderRadius: {
        'DEFAULT': '1rem', // 2xl equivalent
      }
    },
  },
  plugins: [],
}
