# Anki Wrapper Plugin Development Guide

This guide covers how to create plugins for the Anki Wrapper desktop application.

## Quick Start

### 1. Create a Plugin Folder

Plugins live in the plugins directory:
- **Linux**: `~/.local/share/anki-wrapper/plugins/`
- **macOS**: `~/Library/Application Support/anki-wrapper/plugins/`
- **Windows**: `%APPDATA%\anki-wrapper\plugins\`

Create a new folder for your plugin (e.g., `my-plugin/`).

### 2. Write manifest.json

Every plugin needs a `manifest.json` file:

```json
{
  "id": "my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "description": "What my plugin does",
  "entry_point": "index.js",
  "hooks": ["card:render:front"],
  "author": "Your Name"
}
```

### 3. Write index.js

Create your plugin logic in `index.js`:

```javascript
// My Plugin
(function() {
  'use strict';

  // Register filter hooks
  __ankiPlugins.registerFilter('card:render:front', function(data) {
    // Modify the HTML
    data.html = data.html + '<div>My modification</div>';
    return data;
  });

  // Register action hooks
  __ankiPlugins.registerAction('review:answer', function(data) {
    console.log('Card answered:', data.cardId);
  });
})();
```

### 4. Restart the App

Restart Anki Wrapper to load your plugin. Check the Plugins Manager (puzzle piece icon in the nav) to see if it loaded correctly.

---

## Manifest Schema

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | Unique identifier (lowercase, hyphens only) |
| `name` | string | Yes | Display name shown in Plugin Manager |
| `version` | string | Yes | Semver version (e.g., "1.0.0") |
| `description` | string | Yes | What the plugin does |
| `entry_point` | string | Yes | Main JS file (e.g., "index.js") |
| `hooks` | string[] | No | List of hooks this plugin uses |
| `author` | string | No | Plugin author name |
| `homepage` | string | No | URL to plugin documentation |
| `min_api_version` | string | No | Minimum plugin API version required |

---

## Available Hooks

### Filter Hooks

Filter hooks transform data through a pipeline. Each callback receives the output of the previous one.

| Hook Name | Type | Payload | When It Fires |
|-----------|------|---------|---------------|
| `card:render:front` | filter | `{ html: string, cardId: number, noteId: number }` | Before front HTML is rendered in StudyView |
| `card:render:back` | filter | `{ html: string, cardId: number, noteId: number }` | Before back HTML is rendered in StudyView |
| `editor:field:render` | filter | `{ html: string, fieldIndex: number, notetypeId: number }` | When CardEditor renders a field value |
| `search:query` | filter | `{ query: string }` | Before a search is sent to the backend |

**Filter Hook Example:**

```javascript
__ankiPlugins.registerFilter('card:render:front', function(data) {
  // Add a watermark to all cards
  return {
    ...data,
    html: data.html + '<div class="watermark">My Plugin</div>'
  };
});
```

### Action Hooks

Action hooks are fire-and-forget side effects. All registered callbacks run in parallel.

| Hook Name | Type | Payload | When It Fires |
|-----------|------|---------|---------------|
| `review:answer` | action | `{ cardId: number, noteId: number, rating: number, deckId: number }` | After a card is answered |
| `review:show` | action | `{ cardId: number, noteId: number, deckId: number }` | When a new card is shown |
| `app:ready` | action | `{}` | After the app finishes loading |
| `deck:study:start` | action | `{ deckId: number, deckName: string }` | When entering study mode |
| `deck:study:end` | action | `{ deckId: number, reviewed: number }` | When exiting study mode |

**Action Hook Example:**

```javascript
__ankiPlugins.registerAction('review:answer', function(data) {
  // Log card completion to analytics
  console.log('Card answered:', {
    cardId: data.cardId,
    rating: data.rating
  });
});
```

---

## Plugin API

The `__ankiPlugins` global object is available to all plugins:

### registerFilter(hookName, callback, priority?)

Register a filter hook. The callback receives data, modifies it, and returns the modified data.

```typescript
registerFilter(
  hookName: string,           // e.g., "card:render:front"
  callback: (data: any) => any, // Transform function
  priority?: number           // Lower runs first (default: 10)
): void
```

### registerAction(hookName, callback, priority?)

Register an action hook. The callback performs side effects and doesn't return anything.

```typescript
registerAction(
  hookName: string,              // e.g., "review:answer"
  callback: (data: any) => void, // Side effect function
  priority?: number              // Lower runs first (default: 10)
): void
```

### version

Current plugin API version string. Use this for compatibility checks.

```javascript
if (__ankiPlugins.version !== '1.0.0') {
  console.warn('Plugin API version mismatch');
}
```

---

## CSS in Plugins

Plugins can include a `styles.css` file for custom styling. This is automatically injected into the document head when the plugin loads.

**Example styles.css:**

```css
/* Prefix your selectors with the plugin ID for scoping */
#my-plugin .custom-class {
  color: red;
}

/* Or use data attributes */
[data-plugin="my-plugin"] ruby rt {
  font-size: 0.6em;
}
```

**Recommendation:** Prefix your CSS selectors to avoid conflicts with the app's styles. A common pattern is to wrap your plugin content in a container with a unique ID.

---

## Error Handling

If your plugin callback throws an exception:

1. **The error is caught** — It won't crash the app
2. **The error is logged** — Check the Plugin Manager for runtime errors
3. **The pipeline continues** — For filters, the previous valid data is used; for actions, other plugins still run

```javascript
// This is safe - errors won't break the app
__ankiPlugins.registerFilter('card:render:front', function(data) {
  // If this throws, your transformation is skipped
  // and the original data continues through the pipeline
  throw new Error('Oops!');
});
```

---

## Security Model

**⚠️ Important Security Notes:**

- Plugins have **full DOM access** — they can modify any part of the page
- Plugins can call **Tauri invoke** — they have access to all Rust backend commands
- Plugins can access **card data** — be careful what data you log or transmit
- **Only install plugins from trusted sources**

There is **no sandboxing** in V1. You are responsible for vetting the plugins you install.

---

## V1 Limitations

The current version has some limitations:

- ❌ **No hot-reload** — You must restart the app after installing/enabling a plugin
- ❌ **No direct Rust access** — Use `invoke()` to call backend commands
- ❌ **No custom UI panels** — Plugins can only modify existing content via hooks
- ❌ **No inter-plugin communication** — Plugins operate independently
- ❌ **Non-deterministic load order** — Plugin load order depends on filesystem order

---

## Example: Furigana Plugin

This sample plugin converts `{漢字|かんじ}` syntax to ruby annotations:

```javascript
// index.js
(function() {
  'use strict';

  function addFurigana(data) {
    // Match {kanji|reading} pattern
    const furiganaRegex = /\{([^|{}]+)\|([^|{}]+)\}/g;

    const newHtml = data.html.replace(furiganaRegex, function(match, kanji, reading) {
      return '<ruby>' + kanji + '<rp>(</rp><rt>' + reading + '</rt><rp>)</rp></ruby>';
    });

    return { ...data, html: newHtml };
  }

  // Register for both front and back rendering
  __ankiPlugins.registerFilter('card:render:front', addFurigana);
  __ankiPlugins.registerFilter('card:render:back', addFurigana);
})();
```

**Usage:** In your card content, write:
```
{漢字|かんじ} means kanji
```

**Result:** Displays as 漢字(かんじ) with furigana above the kanji.

---

## Example: Study Timer

This sample plugin tracks study session statistics:

```javascript
// index.js
(function() {
  'use strict';

  var sessionStart = null;
  var cardCount = 0;
  var ratingCounts = { 1: 0, 2: 0, 3: 0, 4: 0 };

  // Start timer when entering study mode
  __ankiPlugins.registerAction('deck:study:start', function(data) {
    sessionStart = Date.now();
    cardCount = 0;
    ratingCounts = { 1: 0, 2: 0, 3: 0, 4: 0 };
    console.log('[Study Timer] Session started for deck:', data.deckName);
  });

  // Track each answer
  __ankiPlugins.registerAction('review:answer', function(data) {
    cardCount++;
    if (ratingCounts[data.rating] !== undefined) {
      ratingCounts[data.rating]++;
    }
  });

  // Show summary when session ends
  __ankiPlugins.registerAction('deck:study:end', function(data) {
    if (!sessionStart) return;
    var elapsed = Math.round((Date.now() - sessionStart) / 1000);
    var minutes = Math.floor(elapsed / 60);
    var seconds = elapsed % 60;

    console.log('[Study Timer] Session complete:');
    console.log('  Duration:', minutes + 'm ' + seconds + 's');
    console.log('  Cards reviewed:', cardCount);
    console.log('  Again:', ratingCounts[1],
                '  Hard:', ratingCounts[2],
                '  Good:', ratingCounts[3],
                '  Easy:', ratingCounts[4]);

    sessionStart = null;
  });
})();
```

**Result:** Open the browser console (F12) to see session statistics logged when you finish studying.

---

## Sample Plugins

The app includes two sample plugins in the resources folder:

1. **furigana-example** — Demonstrates filter hooks for transforming card content
2. **study-timer** — Demonstrates action hooks for tracking study sessions

To test them:
1. Copy the sample plugin folders to your plugins directory
2. Restart the app
3. The plugins will appear in the Plugin Manager

---

## Troubleshooting

### Plugin Not Loading

- Check the Plugin Manager for error messages
- Verify `manifest.json` is valid JSON
- Ensure `entry_point` file exists
- Check the browser console for JavaScript errors

### Plugin Not Working

- Verify you're using the correct hook name
- Check that the hook fires at the right time (e.g., `card:render:front` only fires during study)
- Look for runtime errors in the Plugin Manager

### Conflicts with Other Plugins

- Try adjusting priority (lower = runs first)
- Check if another plugin is modifying the same data
