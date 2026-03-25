# Anki Wrapper Plugin Development Guide

## 1. Overview

The Anki Wrapper plugin system is a JavaScript-based hook/filter architecture that allows developers to extend the application's functionality. Plugins are self-contained folders stored in `~/.local/share/anki-wrapper/plugins/<plugin-id>/`, each containing a `manifest.json` and an entry point JavaScript file.

### What Plugins Can Do

- **Hook into app lifecycle events** (app:ready, study:start, etc.)
- **Modify card rendering** (add furigana, highlight text, inject badges)
- **Inject custom CSS** to change the app's appearance
- **Register custom widgets** for the dashboard or deck overview
- **Filter search queries** before they reach the backend
- **Respond to user actions** (answering cards, navigating pages)

### Architecture

Plugins run in the same JavaScript context as the app (no sandbox). They access the plugin API via `window.__ankiPlugins`, which provides methods to register action hooks, filter hooks, and widgets. The plugin engine ensures complete error isolation—a plugin crash never breaks the app.

```
┌─────────────────────────────────────────────────────────────┐
│                     Anki Wrapper App                        │
├─────────────────────────────────────────────────────────────┤
│  Plugin Engine (pluginEngine.ts)                            │
│  ├── Action Hooks: fire-and-forget events                   │
│  ├── Filter Hooks: data transformation pipelines            │
│  └── Widget Registry: custom UI components                  │
├─────────────────────────────────────────────────────────────┤
│  Plugin Loader (pluginLoader.ts)                            │
│  ├── Reads manifests from plugins directory                 │
│  ├── Validates security constraints                         │
│  ├── Injects CSS into <head>                                │
│  └── Evaluates JS via new Function()                        │
├─────────────────────────────────────────────────────────────┤
│  Tauri Backend (lib.rs)                                     │
│  ├── get_installed_plugins: scan and validate plugins       │
│  ├── get_plugin_source: read entry point JS                 │
│  ├── get_plugin_css: read styles.css                        │
│  ├── enable_plugin / disable_plugin: manage disabled list   │
│  └── open_plugins_folder: open in file manager              │
└─────────────────────────────────────────────────────────────┘
```

## 2. Getting Started

### 2.1 Plugin Directory Structure

Plugins are stored in `~/.local/share/anki-wrapper/plugins/`. Each plugin is a folder with this structure:

```
~/.local/share/anki-wrapper/plugins/
└── my-plugin/
    ├── manifest.json      (required)
    ├── index.js           (required, entry point)
    └── styles.css         (optional)
```

### 2.2 The Manifest File

Every plugin must have a `manifest.json` file with the following structure:

```json
{
  "id": "my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "description": "Does cool stuff",
  "entry_point": "index.js",
  "hooks": ["card:render:front", "app:ready"],
  "author": "Your Name",
  "homepage": "https://github.com/yourname/my-plugin",
  "min_api_version": "1.0.0"
}
```

#### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier. Must match the folder name exactly. |
| `name` | string | Human-readable display name shown in Plugin Manager. |
| `version` | string | Semantic version (e.g., "1.0.0"). |
| `description` | string | Short description of what the plugin does. |
| `entry_point` | string | Path to the main JS file (relative to plugin folder). |

#### Optional Fields

| Field | Type | Description |
|-------|------|-------------|
| `hooks` | string[] | List of hooks this plugin uses (for UI display only). |
| `author` | string | Plugin author's name. |
| `homepage` | string | URL to plugin's homepage or repository. |
| `min_api_version` | string | Minimum plugin API version required (currently "1.0.0"). |

#### Security Constraints

The Rust backend validates manifests for security:

- **`id`**: Must not contain `..`, `/`, or `\` (prevents path traversal)
- **`entry_point`**: Must not contain `..`, `/`, or `\` (no subdirectories allowed)
- **Plugin folder name**: Must not start with `.` (hidden folders are skipped)

Plugins with invalid manifests will show a load error in the Plugin Manager UI.

### 2.3 Your First Plugin

Here's a minimal plugin that logs a message when the app starts:

**manifest.json:**
```json
{
  "id": "hello-world",
  "name": "Hello World",
  "version": "1.0.0",
  "description": "A simple hello world plugin",
  "entry_point": "index.js"
}
```

**index.js:**
```javascript
(function(api) {
  // Register an action hook
  api.registerAction('app:ready', function() {
    console.log('Hello from my plugin!');
  });
})(window.__ankiPlugins);
```

#### How It Works

1. The plugin loader reads `manifest.json` and validates it
2. It fetches the JS source via `get_plugin_source` Tauri command
3. The JS is evaluated using `new Function("__ankiPlugins", source)`
4. Your plugin function is called with `window.__ankiPlugins` as the argument
5. Your plugin registers its hooks, which are stored in the plugin engine
6. When events fire, your hooks are called in priority order

**Important:** You don't need to pass your plugin ID to the API. The loader sets a loading context before evaluating your script, so the engine automatically associates your hooks with your plugin.

## 3. Plugin API Reference

The plugin API is exposed via `window.__ankiPlugins` with the following interface:

```typescript
interface PluginAPI {
  // Register a filter hook (transforms data)
  registerFilter(
    hookName: string,
    callback: (data: any) => any,
    priority?: number  // default: 10, lower runs first
  ): void;

  // Register an action hook (fire-and-forget)
  registerAction(
    hookName: string,
    callback: (data: any) => void,
    priority?: number  // default: 10, lower runs first
  ): void;

  // Register a custom widget
  registerWidget(config: WidgetConfig): void;

  // API version for compatibility checks
  version: string;  // "1.0.0"
}
```

### 3.1 Action Hooks

Action hooks are fire-and-forget events. They notify plugins that something happened, but plugins cannot modify the event data.

```javascript
api.registerAction('app:ready', function(data) {
  // data is an empty object for app:ready
  console.log('App is ready!');
}, 10);  // optional priority
```

#### Available Action Hooks

| Hook | Payload | When Fired |
|------|---------|------------|
| `app:ready` | `{}` | After collection opens and plugins load |
| `review:answer` | `{ cardId, noteId, rating, deckId }` | After user answers a card (rating: 1=Again, 2=Hard, 3=Good, 4=Easy) |
| `review:show` | `{ cardId, noteId, deckId }` | When a new card is shown in study mode |
| `deck:study:start` | `{ deckId, deckName }` | When user enters study mode |
| `deck:study:end` | `{ deckId, reviewed }` | When user exits study mode (reviewed: number of cards reviewed) |

**Note:** The plugin engine supports any action name via `runAction(name, data)`. The hooks listed above are the ones the app currently fires. You can listen for custom action names if needed.

#### Priority

Priority determines execution order. Lower numbers run first:

```javascript
// Runs before most other plugins
api.registerAction('app:ready', function() {
  console.log('I run first!');
}, 1);

// Runs after most other plugins
api.registerAction('app:ready', function() {
  console.log('I run last!');
}, 100);
```

### 3.2 Filter Hooks

Filter hooks transform data as it passes through the system. Each filter callback receives data and **must return the (possibly modified) data**.

```javascript
api.registerFilter('card:render:front', function(data) {
  // data contains: { html, cardId, noteId }
  // Modify the HTML
  data.html += '<div class="my-badge">Plugin</div>';
  // Return the modified data
  return data;
}, 10);
```

#### Available Filter Hooks

| Hook | Input | Expected Return | When Called |
|------|-------|----------------|------------|
| `card:render:front` | `{ html, cardId, noteId }` | `{ html, cardId, noteId }` | Before front of card renders in study view |
| `card:render:back` | `{ html, cardId, noteId }` | `{ html, cardId, noteId }` | Before back of card renders in study view |
| `editor:field:render` | `{ html, fieldIndex, notetypeId }` | `{ html, fieldIndex, notetypeId }` | Before field renders in card editor |
| `search:query` | `{ query }` | `{ query }` | Before search query is sent to backend |

#### Filter Pipeline

Filters run in priority order. Each filter's output becomes the next filter's input:

```javascript
// Filter 1 (priority: 1)
api.registerFilter('card:render:front', function(data) {
  data.html = data.html.replace('foo', 'bar');
  return data;
});

// Filter 2 (priority: 2)
api.registerFilter('card:render:front', function(data) {
  // Receives HTML with 'foo' already replaced by 'bar'
  data.html += '<div>Extra content</div>';
  return data;
});
```

#### Error Handling

If a filter throws an error, the engine logs it and continues with the previous valid result. Your plugin won't break other plugins or the app.

### 3.3 Widget Registration

Widgets allow you to add custom UI components to the dashboard or deck overview.

```javascript
api.registerWidget({
  id: 'my-widget',
  title: 'My Widget',
  render: function(container) {
    container.innerHTML = '<div>Hello from my widget!</div>';
    // Optional: return a cleanup function
    return function() {
      console.log('Widget cleanup');
    };
  },
  locations: ['dashboard', 'deckOverview'],
  defaultOrder: 50
});
```

#### WidgetConfig Interface

```typescript
interface WidgetConfig {
  id: string;                    // Unique widget ID (prefixed with plugin ID)
  title: string;                 // Display title
  render: (container: HTMLElement) => void | (() => void);
  locations?: ('dashboard' | 'deckOverview')[];  // default: ['dashboard']
  defaultOrder?: number;         // default: 100, lower = appears first
  gridHeight?: number;           // Height in grid units (1 unit = 180px). default: 1
}
```

#### Widget Grid Layout

Dashboard widgets are rendered in a fixed-height grid system. Each widget occupies a number of "grid units" where 1 unit = 180px tall. The `gridHeight` property controls how many units your widget spans:

- `gridHeight: 1` — 180px tall (default). Good for: stat summaries, quick-glance info, small lists.
- `gridHeight: 2` — 360px tall. Good for: calendars, charts, longer lists.
- `gridHeight: 3` — 540px tall. Use sparingly; good for: complex interactive widgets.

Your widget's `render` container will have a fixed height. If your content might overflow, add `overflow-y: auto` to your inner content area:

```javascript
api.registerWidget({
  id: 'long-list-widget',
  title: 'Recent Activity',
  gridHeight: 2,
  render: function(container) {
    container.style.height = '100%';
    container.style.display = 'flex';
    container.style.flexDirection = 'column';
    
    const header = document.createElement('div');
    header.textContent = '50 most recent actions';
    header.style.marginBottom = '8px';
    container.appendChild(header);
    
    const list = document.createElement('div');
    list.style.flex = '1';
    list.style.minHeight = '0';
    list.style.overflowY = 'auto';
    // ... populate list items
    container.appendChild(list);
  },
  locations: ['dashboard'],
  defaultOrder: 50
});
```

**Important:** The decks list is rendered in a dedicated left column on the dashboard, separate from the widget grid. Plugin widgets always appear in the right-side widget column. Widgets cannot be manually reordered by dragging — their display order is determined by `defaultOrder` and the user's `widget_order` preference array.

#### Render Function

The `render` function receives a container `HTMLElement` and should populate it with your widget's content. You can optionally return a cleanup function that's called when the widget is removed.

```javascript
api.registerWidget({
  id: 'timer-widget',
  title: 'Study Timer',
  render: function(container) {
    let seconds = 0;
    container.innerHTML = '<div id="timer">0:00</div>';
    
    const interval = setInterval(function() {
      seconds++;
      const mins = Math.floor(seconds / 60);
      const secs = seconds % 60;
      container.querySelector('#timer').textContent = 
        mins + ':' + (secs < 10 ? '0' : '') + secs;
    }, 1000);
    
    // Return cleanup function
    return function() {
      clearInterval(interval);
    };
  },
  locations: ['dashboard'],
  defaultOrder: 10
});
```

## 4. Theme Development

### 4.1 CSS Custom Properties

The app uses CSS custom properties for all colors. Themes override these in `:root` (light theme) and `:root.dark` (dark theme).

**Light Theme (default):**
```css
:root {
  --bg-base: #F0EBE3;           /* Page background */
  --bg-deep: #E8E2D8;           /* Deeper background */
  --bg-card: #F7F4EF;           /* Card/panel background */
  --bg-card-raised: #FAF8F4;    /* Raised card background */
  --bg-subtle: #EDE9E3;         /* Subtle background (inputs, hover) */
  
  --text-primary: #2C2825;      /* Main text color */
  --text-secondary: #8C8278;    /* Secondary/muted text */
  --text-muted: #B5ADA3;        /* Muted text */
  
  --accent: #C4714F;            /* Primary accent (buttons, links) */
  --accent-soft: #F0E6DF;       /* Light accent background */
  --success: #6B8F71;           /* Success states */
  --danger: #C0444A;            /* Error/danger states */
  --warning: #C49A4F;           /* Warning states */
  
  --border: #DDD6CC;            /* Border color */
  
  --neu-up: 5px 5px 14px rgba(0,0,0,0.12), -4px -4px 10px rgba(255,255,255,0.9);
  --neu-down: inset 2px 2px 6px rgba(0,0,0,0.12), inset -2px -2px 5px rgba(255,255,255,0.7);
  --neu-subtle: 3px 3px 7px rgba(0,0,0,0.08), -2px -2px 5px rgba(255,255,255,0.8);
  --shadow-warm: 0 4px 24px rgba(0,0,0,0.10);
  
  --radius-sm: 10px;
  --radius-md: 16px;
  --radius-lg: 22px;
}
```

**Dark Theme:**
```css
:root.dark {
  --bg-base: #1C1917;
  --bg-deep: #141210;
  --bg-card: #292524;
  --bg-card-raised: #2E2A27;
  --bg-subtle: #44403C;
  
  --text-primary: #FAFAF9;
  --text-secondary: #A8A29E;
  --text-muted: #78716C;
  
  --accent: #D4845F;
  --accent-soft: #3D2E26;
  --success: #7DAF83;
  --danger: #F87171;
  --warning: #FBBF24;
  
  --border: #44403C;
  
  --neu-up: 6px 6px 14px rgba(0,0,0,0.45), -3px -3px 8px rgba(255,255,255,0.06);
  --neu-down: inset 3px 3px 6px rgba(0,0,0,0.4), inset -2px -2px 5px rgba(255,255,255,0.04);
  --neu-subtle: 3px 3px 7px rgba(0,0,0,0.35), -2px -2px 5px rgba(255,255,255,0.04);
  --shadow-warm: 0 4px 24px rgba(0,0,0,0.3);
}
```

### 4.2 Creating a Theme Plugin

Theme plugins typically only need CSS, but can include JS for dynamic theming.

**manifest.json:**
```json
{
  "id": "theme-solarized",
  "name": "Solarized Theme",
  "version": "1.0.0",
  "description": "Solarized color scheme for Anki Wrapper",
  "entry_point": "index.js",
  "hooks": ["app:ready"]
}
```

**index.js:**
```javascript
(function(api) {
  // Themes typically only need CSS, but JS can be used for dynamic theming
  api.registerAction('app:ready', function() {
    console.log('Solarized theme loaded');
  });
})(window.__ankiPlugins);
```

**styles.css:**
```css
/* Light theme */
:root {
  --bg-base: #FDF6E3;
  --bg-card: #EEE8D5;
  --bg-subtle: #E0DAC7;
  --text-primary: #073642;
  --text-secondary: #586E75;
  --accent: #268BD2;
  --accent-soft: #D5E5F0;
  --success: #859900;
  --danger: #DC322F;
  --warning: #B58900;
  --border: #D3CBB8;
  --shadow-warm: 0 4px 24px rgba(0,0,0,0.08);
}

/* Dark theme */
:root.dark {
  --bg-base: #002B36;
  --bg-card: #073642;
  --bg-subtle: #0A4050;
  --text-primary: #FDF6E3;
  --text-secondary: #93A1A1;
  --accent: #268BD2;
  --accent-soft: #0A4050;
  --success: #859900;
  --danger: #DC322F;
  --warning: #B58900;
  --border: #0A4050;
  --shadow-warm: 0 4px 24px rgba(0,0,0,0.4);
}
```

### 4.3 Design System Classes

The app uses Tailwind v4 with custom theme tokens:

**Background Colors:**
- `bg-bg-base` - Page background
- `bg-bg-deep` - Deeper background
- `bg-bg-card` - Card/panel background
- `bg-bg-card-raised` - Raised card background
- `bg-bg-subtle` - Subtle background (inputs, hover)

**Text Colors:**
- `text-text-primary` - Main text
- `text-text-secondary` - Secondary/muted text
- `text-text-muted` - Muted text

**Accent Colors:**
- `text-accent`, `bg-accent` - Primary accent
- `bg-accent-soft` - Light accent background
- `text-success`, `text-danger`, `text-warning` - Status colors

**Other:**
- `border-border` - Border color
- `shadow-warm` - Card shadow
- `rounded-2xl` - Large border radius (maps to `--radius-md`)

### 4.4 Typography

- **Default font:** `system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif` (via `--sans`)
- **Card content font:** `'Source Serif 4', 'Iowan Old Style', 'Apple Garamond', Baskerville, 'Times New Roman', serif` (via `--serif`)
- **Base font size:** 16px (15px on screens ≤1024px)
- **Card content:** 1.2rem / 1.85 line height

## 5. Plugin Examples

### 5.1 Card Timer Plugin

Adds a visible timer to the study view:

```javascript
(function(api) {
  let timerEl = null;
  let startTime = null;
  let intervalId = null;

  api.registerAction('deck:study:start', function(data) {
    startTime = Date.now();
    
    // Create timer element
    timerEl = document.createElement('div');
    timerEl.id = 'plugin-timer';
    timerEl.style.cssText = `
      position: fixed;
      bottom: 16px;
      right: 16px;
      background: var(--bg-card);
      border: 1px solid var(--border);
      border-radius: 12px;
      padding: 8px 16px;
      font-size: 14px;
      color: var(--text-secondary);
      z-index: 100;
      box-shadow: var(--shadow-warm);
    `;
    document.body.appendChild(timerEl);

    // Update timer every second
    intervalId = setInterval(function() {
      var elapsed = Math.floor((Date.now() - startTime) / 1000);
      var mins = Math.floor(elapsed / 60);
      var secs = elapsed % 60;
      timerEl.textContent = mins + ':' + (secs < 10 ? '0' : '') + secs;
    }, 1000);
  });

  api.registerAction('deck:study:end', function() {
    if (intervalId) clearInterval(intervalId);
    if (timerEl) timerEl.remove();
    timerEl = null;
  });
})(window.__ankiPlugins);
```

### 5.2 Furigana Plugin

Adds furigana (ruby text) support for Japanese:

```javascript
(function(api) {
  // Register filter for card front
  api.registerFilter('card:render:front', function(data) {
    data.html = addFurigana(data.html);
    return data;
  });

  // Register filter for card back
  api.registerFilter('card:render:back', function(data) {
    data.html = addFurigana(data.html);
    return data;
  });

  function addFurigana(html) {
    // Convert [kanji|reading] syntax to ruby tags
    return html.replace(/\[([^\|]+)\|([^\]]+)\]/g,
      '<ruby>$1<rp>(</rp><rt>$2</rt><rp>)</rp></ruby>'
    );
  }
})(window.__ankiPlugins);
```

**Usage in cards:**
```
[漢字|かんじ] means "kanji"
```

**Renders as:**
```
漢字 (with かんじ above it)
```

### 5.3 Search Highlight Plugin

Highlights search terms in card content:

```javascript
(function(api) {
  let searchQuery = '';

  // Capture search queries
  api.registerFilter('search:query', function(data) {
    searchQuery = data.query;
    return data;
  });

  // Highlight in card front
  api.registerFilter('card:render:front', function(data) {
    if (searchQuery) {
      data.html = highlightTerms(data.html, searchQuery);
    }
    return data;
  });

  // Highlight in card back
  api.registerFilter('card:render:back', function(data) {
    if (searchQuery) {
      data.html = highlightTerms(data.html, searchQuery);
    }
    return data;
  });

  function highlightTerms(html, query) {
    // Extract search terms (simple implementation)
    const terms = query.split(/\s+/).filter(t => t.length > 2);
    let result = html;
    
    terms.forEach(function(term) {
      const regex = new RegExp('(' + term + ')', 'gi');
      result = result.replace(regex, 
        '<mark style="background: var(--accent-soft); padding: 0 2px; border-radius: 2px;">$1</mark>'
      );
    });
    
    return result;
  }
})(window.__ankiPlugins);
```

### 5.4 Stats Widget Plugin

Adds a custom stats widget to the dashboard:

```javascript
(function(api) {
  api.registerWidget({
    id: 'daily-goal',
    title: 'Daily Goal',
    render: function(container) {
      // Create widget content
      container.innerHTML = `
        <div style="padding: 16px;">
          <div style="font-size: 14px; color: var(--text-secondary); margin-bottom: 8px;">
            Today's Progress
          </div>
          <div style="font-size: 32px; font-weight: 600; color: var(--accent);">
            42
          </div>
          <div style="font-size: 12px; color: var(--text-muted);">
            cards reviewed
          </div>
        </div>
      `;
    },
    locations: ['dashboard'],
    defaultOrder: 5
  });
})(window.__ankiPlugins);
```

## 6. Tauri IPC from Plugins

Plugins run in the webview context and have access to `window.__TAURI_INTERNALS__`. They can call Tauri invoke commands if needed, but this is advanced usage.

### 6.1 Calling Tauri Commands

```javascript
(function(api) {
  api.registerAction('app:ready', async function() {
    try {
      // Access the Tauri invoke API
      const invoke = window.__TAURI_INTERNALS__.invoke;
      const version = await invoke('get_app_version');
      console.log('App version:', version);
    } catch(e) {
      console.error('Tauri invoke failed:', e);
    }
  });
})(window.__ankiPlugins);
```

### 6.2 Available Tauri Commands

The app exposes these commands via the `invoke_handler` in `lib.rs`:

**Deck Management:**
- `get_all_decks` - Get all decks
- `get_deck_stats` - Get deck statistics
- `create_deck` - Create a new deck
- `rename_deck` - Rename a deck
- `delete_deck` - Delete a deck

**Card Management:**
- `search_cards` - Search for cards
- `search_notes` - Search for notes
- `get_card` - Get a specific card
- `get_note` - Get a specific note
- `update_card` - Update a card
- `update_note` - Update a note

**Tag Management:**
- `get_all_tags` - Get all tags

**Preferences:**
- `get_preferences` - Get user preferences
- `save_preferences` - Save user preferences

**Plugin Management:**
- `get_installed_plugins` - Get all installed plugins
- `get_plugin_source` - Get plugin JS source
- `get_plugin_css` - Get plugin CSS
- `enable_plugin` - Enable a plugin
- `disable_plugin` - Disable a plugin
- `open_plugins_folder` - Open plugins folder in file manager

**Import/Export:**
- `import_anki_package` - Import .apkg file
- `export_collection_colpkg` - Export collection

**Statistics:**
- `get_review_stats` - Get review statistics
- `get_deck_overview` - Get deck overview data

**Note:** This list may not be exhaustive. Check the `invoke_handler` registration in `lib.rs` for the complete list.

## 7. Plugin Lifecycle

### 7.1 Loading Sequence

1. **App starts** → `init_standalone_collection` is called
2. **Collection ready** → `loadAllPlugins()` is called
3. **Plugin scanning** → Backend scans `~/.local/share/anki-wrapper/plugins/`
4. **Manifest validation** → Each plugin's `manifest.json` is validated
5. **CSS injection** → If `styles.css` exists, it's injected into `<head>`
6. **JS evaluation** → Entry point JS is evaluated via `new Function()`
7. **Hook registration** → Plugin registers its action/filter hooks
8. **App ready** → `app:ready` action is fired
9. **Normal operation** → Hooks are called during app usage

### 7.2 Unloading Sequence

When a plugin is disabled:

1. **Disable command** → `disable_plugin` Tauri command is called
2. **Hook unregistration** → All hooks registered by the plugin are removed
3. **CSS removal** → Plugin's `<style>` element is removed from `<head>`
4. **Cleanup** → Any cleanup functions returned by widgets are called

### 7.3 Error Isolation

The plugin engine ensures complete error isolation:

- **Filter errors:** If a filter throws, the error is logged and the previous valid result is used
- **Action errors:** If an action throws, the error is logged and other actions continue
- **Load errors:** If a plugin fails to load, it's marked with a `load_error` and skipped
- **Timeout:** Each plugin has a 3-second timeout during loading

## 8. Debugging Plugins

### 8.1 Opening DevTools

Open DevTools to see plugin logs and errors:
- **macOS:** `Cmd + Option + I`
- **Windows/Linux:** `Ctrl + Shift + I`

### 8.2 Console Logging

Use `console.log` in your plugin code—it works normally:

```javascript
(function(api) {
  api.registerAction('app:ready', function() {
    console.log('Plugin loaded!');
    console.log('API version:', api.version);
  });
})(window.__ankiPlugins);
```

### 8.3 Error Messages

Plugin errors are logged to the console with the plugin ID:

```
[Plugin "my-plugin"] Hook "app:ready": TypeError: Cannot read property 'x' of undefined
```

### 8.4 Plugin Manager UI

The Plugin Manager shows load errors for plugins with invalid manifests or missing files.

### 8.5 CSS Debugging

Plugin styles are injected as `<style data-plugin-id="your-plugin-id">` in `<head>`. You can inspect them in DevTools.

## 9. Security & Limitations

### 9.1 Security Model

- **No sandbox:** Plugins execute in the same context as the app
- **DOM access:** Plugins can access and modify the entire DOM
- **Storage access:** Plugins can use `localStorage`, `sessionStorage`, etc.
- **Network access:** Plugins can use `fetch`, `XMLHttpRequest`, etc.
- **Tauri access:** Plugins can call Tauri invoke commands (if they know the API)

### 9.2 Security Constraints

- **Path traversal:** Plugin IDs and entry points cannot contain `..`, `/`, or `\`
- **No subdirectories:** Entry points must be in the plugin root folder
- **No hidden folders:** Plugin folders starting with `.` are skipped
- **Manifest validation:** Invalid manifests are rejected with load errors

### 9.3 Limitations

- **No Rust code:** Plugins cannot modify the Rust backend
- **No native APIs:** Plugins cannot directly access system APIs (only via Tauri)
- **No persistent background:** Plugins run in the webview context, not as background services
- **No IPC between plugins:** Plugins cannot directly communicate with each other

### 9.4 Best Practices

- **Error handling:** Wrap your code in try-catch to avoid breaking the app
- **Cleanup:** Return cleanup functions from widgets to avoid memory leaks
- **Priority:** Use appropriate priorities to ensure correct execution order
- **Testing:** Test your plugin with the app to ensure it works correctly
- **Documentation:** Document your plugin's hooks and behavior

## 10. Publishing & Distribution

### 10.1 Manual Installation

Currently, plugins are manually installed by copying to the plugins directory:

1. Create a folder in `~/.local/share/anki-wrapper/plugins/`
2. Add your `manifest.json`, `index.js`, and optional `styles.css`
3. Restart the app or reload plugins via Plugin Manager

### 10.2 Accessing the Plugins Folder

Access the plugins folder via:
- **Plugin Manager UI:** Click "Open Plugins Folder" button
- **Manual navigation:** `~/.local/share/anki-wrapper/plugins/`
- **Terminal:** `open ~/.local/share/anki-wrapper/plugins/` (macOS)

### 10.3 Disabling Plugins

Plugins can be disabled via:
- **Plugin Manager UI:** Toggle the enable/disable switch
- **Manual editing:** Add the plugin ID to `~/.local/share/anki-wrapper/plugins/disabled.json`

### 10.4 Future Distribution

Future versions may include:
- Plugin marketplace
- Automatic updates
- Version compatibility checking
- Plugin dependencies

## Appendix A: Complete Plugin Example

Here's a complete plugin that demonstrates multiple features:

**manifest.json:**
```json
{
  "id": "study-enhancer",
  "name": "Study Enhancer",
  "version": "1.0.0",
  "description": "Enhances the study experience with timer, stats, and custom styling",
  "entry_point": "index.js",
  "hooks": ["app:ready", "deck:study:start", "deck:study:end", "card:render:front"],
  "author": "Your Name",
  "homepage": "https://github.com/yourname/study-enhancer",
  "min_api_version": "1.0.0"
}
```

**index.js:**
```javascript
(function(api) {
  let timerEl = null;
  let startTime = null;
  let intervalId = null;
  let cardsReviewed = 0;

  // Add timer when study starts
  api.registerAction('deck:study:start', function(data) {
    startTime = Date.now();
    cardsReviewed = 0;
    
    timerEl = document.createElement('div');
    timerEl.id = 'study-enhancer-timer';
    timerEl.style.cssText = `
      position: fixed;
      bottom: 16px;
      right: 16px;
      background: var(--bg-card);
      border: 1px solid var(--border);
      border-radius: 12px;
      padding: 12px 20px;
      font-size: 14px;
      color: var(--text-secondary);
      z-index: 100;
      box-shadow: var(--shadow-warm);
      display: flex;
      flex-direction: column;
      gap: 4px;
    `;
    document.body.appendChild(timerEl);

    intervalId = setInterval(function() {
      var elapsed = Math.floor((Date.now() - startTime) / 1000);
      var mins = Math.floor(elapsed / 60);
      var secs = elapsed % 60;
      timerEl.innerHTML = `
        <div>${mins}:${secs < 10 ? '0' : ''}${secs}</div>
        <div style="font-size: 12px; color: var(--text-muted);">${cardsReviewed} cards</div>
      `;
    }, 1000);
  });

  // Count cards when answered
  api.registerAction('review:answer', function(data) {
    cardsReviewed++;
  });

  // Remove timer when study ends
  api.registerAction('deck:study:end', function() {
    if (intervalId) clearInterval(intervalId);
    if (timerEl) timerEl.remove();
    timerEl = null;
  });

  // Add badge to card front
  api.registerFilter('card:render:front', function(data) {
    data.html += '<div style="position: absolute; top: 8px; right: 8px; background: var(--accent-soft); color: var(--accent); padding: 4px 8px; border-radius: 8px; font-size: 12px;">Enhanced</div>';
    return data;
  });
})(window.__ankiPlugins);
```

**styles.css:**
```css
/* Add subtle animation to cards */
.card-flip-inner {
  transition: transform 0.38s cubic-bezier(0.4, 0, 0.12, 1), box-shadow 0.2s ease;
}

.card-flip-inner:hover {
  box-shadow: 0 8px 32px rgba(0,0,0,0.15);
}
```

## Appendix B: API Version History

| Version | Changes |
|---------|---------|
| 1.0.0 | Initial release with action hooks, filter hooks, and widgets |

## Appendix C: Troubleshooting

### Plugin not loading

1. Check the Plugin Manager UI for load errors
2. Verify `manifest.json` has all required fields
3. Ensure `entry_point` file exists
4. Check console for error messages

### Hooks not firing

1. Verify you're using the correct hook name
2. Check that your plugin is enabled
3. Ensure you're registering hooks in the global scope
4. Check console for registration errors

### CSS not applying

1. Verify `styles.css` exists in plugin folder
2. Check that the plugin is enabled
3. Inspect `<head>` for your `<style>` element
4. Use DevTools to check CSS specificity

### Widget not appearing

1. Verify `locations` includes the target location
2. Check that `render` function is defined
3. Ensure widget ID is unique
4. Check console for registration errors

---

**Happy plugin development!** 🎉
