<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { readFile } from "@tauri-apps/plugin-fs";
  import { onMount, onDestroy } from "svelte";
  import { EditorView, basicSetup } from "codemirror";
  import { html } from "@codemirror/lang-html";
  import { EditorState } from "@codemirror/state";

  let { 
    value = $bindable(''), 
    placeholder = '',
    minHeight = '120px',
    onchange
  }: { 
    value?: string; 
    placeholder?: string;
    minHeight?: string;
    onchange?: (html: string) => void;
  } = $props();

  let editorEl: HTMLDivElement;
  let codeMirrorEl: HTMLDivElement;
  let codeMirrorView: EditorView | null = null;
  
  // Mode state
  let showHtmlSource = $state(false);
  
  // Toolbar state
  let toolbarVisible = $state(false);
  let toolbarX = $state(0);
  let toolbarY = $state(0);
  let activeFormats = $state({ bold: false, italic: false, underline: false });

  // Initialize CodeMirror when switching to HTML source mode
  $effect(() => {
    if (showHtmlSource && codeMirrorEl && !codeMirrorView) {
      codeMirrorView = new EditorView({
        state: EditorState.create({
          doc: value,
          extensions: [
            basicSetup,
            html(),
            EditorView.updateListener.of((update) => {
              if (update.docChanged) {
                value = update.state.doc.toString();
                onchange?.(value);
              }
            }),
            EditorView.theme({
              "&": { height: "100%", fontSize: "14px" },
              ".cm-scroller": { overflow: "auto" },
              ".cm-content": { fontFamily: "monospace" },
            }),
          ],
        }),
        parent: codeMirrorEl,
      });
    } else if (!showHtmlSource && codeMirrorView) {
      // When switching back to WYSIWYG, update the editor
      if (editorEl) {
        editorEl.innerHTML = value;
      }
      codeMirrorView.destroy();
      codeMirrorView = null;
    }
  });

  onDestroy(() => {
    if (codeMirrorView) {
      codeMirrorView.destroy();
    }
  });

  function toggleHtmlSource() {
    showHtmlSource = !showHtmlSource;
  }

  // Initialize editor content on mount
  $effect(() => {
    if (editorEl && value !== editorEl.innerHTML) {
      editorEl.innerHTML = value || '';
    }
  });

  function handleInput() {
    value = editorEl.innerHTML;
    onchange?.(value);
  }

  function handleKeydown(e: KeyboardEvent) {
    const mod = e.ctrlKey || e.metaKey;
    if (mod && e.key === 'b') { e.preventDefault(); commands.bold(); }
    if (mod && e.key === 'i') { e.preventDefault(); commands.italic(); }
    if (mod && e.key === 'u') { e.preventDefault(); commands.underline(); }
    // Pass through to parent for Ctrl+Enter (save) and Ctrl+Shift+C (cloze)
  }

  function updateToolbar() {
    const sel = window.getSelection();
    if (!sel || sel.isCollapsed) {
      toolbarVisible = false;
      return;
    }
    const range = sel.getRangeAt(0);
    const rect = range.getBoundingClientRect();
    toolbarX = rect.left + rect.width / 2;
    toolbarY = rect.top - 48;
    toolbarVisible = true;
    
    // Update active states
    activeFormats = {
      bold: document.queryCommandState('bold'),
      italic: document.queryCommandState('italic'),
      underline: document.queryCommandState('underline'),
    };
  }

  function format(command: string, cmdValue?: string) {
    editorEl.focus();
    // TODO: document.execCommand is deprecated. Consider migrating to
    // the Clipboard API and manual DOM manipulation for formatting commands.
    // For now, it works reliably in Tauri's embedded webview.
    document.execCommand(command, false, cmdValue);
    value = editorEl.innerHTML;
    onchange?.(value);
    updateToolbar();
  }

  function wrapSelectionInTag(tag: string) {
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) return;
    const range = sel.getRangeAt(0);
    const el = document.createElement(tag);
    range.surroundContents(el);
    value = editorEl.innerHTML;
    onchange?.(value);
  }

  const commands = {
    bold:        () => format('bold'),
    italic:      () => format('italic'),
    underline:   () => format('underline'),
    code:        () => wrapSelectionInTag('code'),
    superscript: () => format('superscript'),
    subscript:   () => format('subscript'),
    orderedList: () => format('insertOrderedList'),
    unorderedList: () => format('insertUnorderedList'),
  };

  // MathJax insertion
  function insertMath(type: 'inline' | 'display') {
    const sel = window.getSelection();
    const hasSelection = sel && !sel.isCollapsed;
    
    if (type === 'inline') {
      if (hasSelection) {
        wrapSelectionWith('\\(', '\\)');
      } else {
        insertAtCursor('\\(\\)');
        // Move cursor between the delimiters
        moveCursor(-2);
      }
    } else {
      if (hasSelection) {
        wrapSelectionWith('\\[', '\\]');
      } else {
        insertAtCursor('\\[\\]');
        moveCursor(-2);
      }
    }
    value = editorEl.innerHTML;
    onchange?.(value);
  }

  function wrapSelectionWith(before: string, after: string) {
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) return;
    const range = sel.getRangeAt(0);
    const text = range.toString();
    const wrapper = document.createTextNode(before + text + after);
    range.deleteContents();
    range.insertNode(wrapper);
  }

  function insertAtCursor(text: string) {
    editorEl.focus();
    document.execCommand('insertText', false, text);
  }

  function moveCursor(offset: number) {
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) return;
    const range = sel.getRangeAt(0);
    range.collapse(false);
    // Move cursor by creating a text node reference
    const textNode = range.startContainer;
    if (textNode.nodeType === Node.TEXT_NODE) {
      const textContent = textNode.textContent || '';
      const newOffset = Math.min(offset, textContent.length);
      range.setStart(textNode, newOffset);
      range.setEnd(textNode, newOffset);
    }
  }

  // Image paste handler
  async function handlePaste(e: ClipboardEvent) {
    const files = e.clipboardData?.files;
    if (!files || files.length === 0) return; // let default paste handle text
    
    const imageFile = [...files].find(f => f.type.startsWith('image/'));
    if (!imageFile) return;
    
    e.preventDefault(); // prevent default browser paste behavior
    
    // Convert file to Uint8Array
    const buffer = await imageFile.arrayBuffer();
    const bytes = Array.from(new Uint8Array(buffer));
    
    // Generate a filename with timestamp to avoid conflicts
    const ext = imageFile.type.split('/')[1] || 'png';
    const filename = `paste-${Date.now()}.${ext}`;
    
    try {
      const actualFilename = await invoke<string>('save_media_file', {
        filename,
        data: bytes
      });
      // Insert img tag at cursor position
      document.execCommand('insertHTML', false, 
        `<img src="anki-media://${actualFilename}" 
              style="max-width: 100%; height: auto;" 
              alt="${actualFilename}" />`
      );
      value = editorEl.innerHTML;
      onchange?.(value);
    } catch (err) {
      console.error('Failed to save image:', err);
    }
  }

  // Image file picker
  async function pickImageFile() {
    const path = await open({
      filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'] }],
      multiple: false
    });
    if (!path) return;
    
    // Read the file using Tauri's fs plugin
    const bytes = await readFile(path as string);
    const filename = (path as string).split('/').pop() || 'image.png';
    
    const actualFilename = await invoke<string>('save_media_file', {
      filename,
      data: Array.from(bytes)
    });
    
    // Focus editor and insert
    editorEl.focus();
    document.execCommand('insertHTML', false,
      `<img src="anki-media://${actualFilename}" 
            style="max-width: 100%; height: auto;" />`
    );
    value = editorEl.innerHTML;
    onchange?.(value);
  }
</script>

<div class="rich-text-editor-container">
  <!-- Permanent toolbar -->
  <div class="editor-toolbar" role="toolbar" aria-label="Text formatting">
    <button onclick={pickImageFile} aria-label="Insert image" title="Insert image">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M21.44 11.05l-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48"/>
      </svg>
    </button>
    
    <!-- List buttons -->
    <button onclick={commands.unorderedList} aria-label="Bullet list" title="Bullet list">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <line x1="8" y1="6" x2="21" y2="6"/><line x1="8" y1="12" x2="21" y2="12"/><line x1="8" y1="18" x2="21" y2="18"/>
        <circle cx="4" cy="6" r="1" fill="currentColor"/><circle cx="4" cy="12" r="1" fill="currentColor"/><circle cx="4" cy="18" r="1" fill="currentColor"/>
      </svg>
    </button>
    <button onclick={commands.orderedList} aria-label="Numbered list" title="Numbered list">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <line x1="10" y1="6" x2="21" y2="6"/><line x1="10" y1="12" x2="21" y2="12"/><line x1="10" y1="18" x2="21" y2="18"/>
        <text x="3" y="7" font-size="6" fill="currentColor">1</text><text x="3" y="13" font-size="6" fill="currentColor">2</text><text x="3" y="19" font-size="6" fill="currentColor">3</text>
      </svg>
    </button>
    
    <div class="toolbar-divider"></div>
    
    <!-- MathJax buttons -->
    <button onclick={() => insertMath('inline')} aria-label="Insert inline math" title="Insert inline math (\\(...\\))">
      <span style="font-family: serif; font-style: italic; font-weight: bold;">x²</span>
    </button>
    <button onclick={() => insertMath('display')} aria-label="Insert display math" title="Insert display math (\\[...\\])">
      <span style="font-family: serif; font-weight: bold;">∑</span>
    </button>
    
    <div class="toolbar-spacer"></div>
    
    <!-- HTML Source toggle -->
    <button
      onclick={toggleHtmlSource}
      class={showHtmlSource ? 'active' : ''}
      aria-label="Toggle HTML source"
      aria-pressed={showHtmlSource}
      title="Toggle HTML source"
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/>
      </svg>
    </button>
  </div>

  {#if showHtmlSource}
    <div 
      bind:this={codeMirrorEl} 
      class="codemirror-container"
      style="min-height: {minHeight}"
    ></div>
  {:else}

  <div
    bind:this={editorEl}
    contenteditable="true"
    class="rich-editor"
    style="min-height: {minHeight}"
    data-placeholder={placeholder}
    oninput={handleInput}
    onkeydown={handleKeydown}
    onmouseup={updateToolbar}
    onkeyup={updateToolbar}
    onpaste={handlePaste}
    role="textbox"
    aria-multiline="true"
    aria-label="Card content editor"
  ></div>
  {/if}

  {#if toolbarVisible && !showHtmlSource}
    <div
      class="floating-toolbar"
      style="left: {toolbarX}px; top: {toolbarY}px; transform: translateX(-50%)"
      onmousedown={(e) => e.preventDefault()}
      role="toolbar"
      aria-label="Text formatting"
    >
      <button class:active={activeFormats.bold} onclick={commands.bold} aria-label="Bold" aria-pressed={activeFormats.bold} title="Bold (Ctrl+B)">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
          <path d="M6 4h8a4 4 0 0 1 4 4 4 4 0 0 1-4 4H6z"/>
          <path d="M6 12h9a4 4 0 0 1 4 4 4 4 0 0 1-4 4H6z"/>
        </svg>
      </button>
      <button class:active={activeFormats.italic} onclick={commands.italic} aria-label="Italic" aria-pressed={activeFormats.italic} title="Italic (Ctrl+I)">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="19" y1="4" x2="10" y2="4"/>
          <line x1="14" y1="20" x2="5" y2="20"/>
          <line x1="15" y1="4" x2="9" y2="20"/>
        </svg>
      </button>
      <button class:active={activeFormats.underline} onclick={commands.underline} aria-label="Underline" aria-pressed={activeFormats.underline} title="Underline (Ctrl+U)">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M6 4v6a6 6 0 0 0 12 0V4"/>
          <line x1="4" y1="20" x2="20" y2="20"/>
        </svg>
      </button>
      <div class="divider"></div>
      <button onclick={commands.code} aria-label="Code" title="Code">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="16 18 22 12 16 6"/>
          <polyline points="8 6 2 12 8 18"/>
        </svg>
      </button>
      <button onclick={commands.superscript} aria-label="Superscript" title="Superscript">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="19" y1="5" x2="5" y2="5"/>
          <line x1="12" y1="5" x2="12" y2="19"/>
          <polyline points="9 9 12 6 15 9"/>
        </svg>
      </button>
      <button onclick={commands.subscript} aria-label="Subscript" title="Subscript">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="19" y1="19" x2="5" y2="19"/>
          <line x1="12" y1="5" x2="12" y2="19"/>
          <polyline points="9 15 12 18 15 15"/>
        </svg>
      </button>
    </div>
  {/if}
</div>

<style>
  .rich-text-editor-container {
    position: relative;
    width: 100%;
  }

  .editor-toolbar {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 4px 0 8px 0;
    border-bottom: 1px solid var(--border);
    margin-bottom: 8px;
  }

  .toolbar-divider {
    width: 1px;
    height: 20px;
    background: var(--border);
    margin: 0 4px;
  }

  .toolbar-spacer {
    flex: 1;
  }

  .editor-toolbar button.active {
    background: var(--accent);
    color: white;
  }

  .editor-toolbar button {
    color: var(--text-secondary);
    background: transparent;
    border: none;
    border-radius: 6px;
    padding: 4px 6px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .editor-toolbar button:hover {
    background: var(--bg-subtle);
    color: var(--text-primary);
  }

  .rich-editor {
    font-family: Georgia, serif;
    font-size: 1.1rem;
    line-height: 1.75;
    padding: 12px 0;
    border: none;
    border-bottom: 2px solid var(--border);
    background: transparent;
    outline: none;
    width: 100%;
    color: var(--text-primary);
  }

  .rich-editor:focus {
    border-bottom-color: var(--accent);
  }

  .rich-editor:empty::before {
    content: attr(data-placeholder);
    color: var(--text-secondary);
    pointer-events: none;
  }

  .rich-editor :global(code) {
    background: rgba(0, 0, 0, 0.1);
    padding: 2px 4px;
    border-radius: 3px;
    font-family: 'SF Mono', Monaco, Consolas, monospace;
    font-size: 0.9em;
  }

  .rich-editor :global(img) {
    max-width: 100%;
    height: auto;
    border-radius: 8px;
    margin: 8px 0;
    display: block;
  }

  .floating-toolbar {
    position: fixed;
    z-index: 100;
    display: flex;
    align-items: center;
    gap: 2px;
    background: #1C1917;
    border-radius: 10px;
    padding: 6px 8px;
    box-shadow: 0 4px 20px rgba(0,0,0,0.25);
    pointer-events: all;
  }

  .floating-toolbar button {
    color: white;
    background: transparent;
    border: none;
    border-radius: 6px;
    padding: 4px 6px;
    cursor: pointer;
    opacity: 0.7;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .floating-toolbar button:hover {
    background: rgba(255,255,255,0.1);
    opacity: 1;
  }

  .floating-toolbar button.active {
    background: rgba(255,255,255,0.15);
    opacity: 1;
  }

  .floating-toolbar .divider {
    width: 1px;
    height: 16px;
    background: rgba(255,255,255,0.2);
    margin: 0 4px;
  }

  .codemirror-container {
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
  }

  .codemirror-container :global(.cm-editor) {
    height: 100%;
    min-height: 200px;
  }

  .codemirror-container :global(.cm-scroller) {
    font-family: 'SF Mono', Monaco, Consolas, monospace;
    font-size: 14px;
  }
</style>
