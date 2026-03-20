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
        'bg-card': 'var(--bg-card)',
        'bg-subtle': 'var(--bg-subtle)',
        'text-primary': 'var(--text-primary)',
        'text-secondary': 'var(--text-secondary)',
        'accent': 'var(--accent)',
        'accent-soft': 'var(--accent-soft)',
        'success': 'var(--success)',
        'danger': 'var(--danger)',
        'warning': 'var(--warning)',
        'border': 'var(--border)',
      },
      fontFamily: {
        'sans': ['system-ui', 'Segoe UI', 'Roboto', 'sans-serif'],
        'serif': ['Georgia', 'serif'],
      },
      boxShadow: {
        'warm': '0 4px 24px rgba(0,0,0,0.06)',
      },
      borderRadius: {
        'DEFAULT': '1rem', // 2xl equivalent
      }
    },
  },
  plugins: [],
}
