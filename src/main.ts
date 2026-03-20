import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import { prefs } from './lib/prefs'

// Load preferences on startup
prefs.load();

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
