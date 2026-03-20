<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { readFile } from "@tauri-apps/plugin-fs";

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
  
  // Toolbar state
  let toolbarVisible = $state(false);
  let toolbarX = $state(0);
  let toolbarY = $state(0);
  let activeFormats = $state({ bold: false, italic: false, underline: false });

  // Initialize editor content on mount
  $effect(() => {
    if (editorEl && value !== editorEl.innerHTML) {
      editorEl.innerHTML = value;
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
  };

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
  <div class="editor-toolbar">
    <button onclick={pickImageFile} title="Insert image">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M21.44 11.05l-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48"/>
      </svg>
    </button>
  </div>

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
  ></div>

  {#if toolbarVisible}
    <div 
      class="floating-toolbar"
      style="left: {toolbarX}px; top: {toolbarY}px; transform: translateX(-50%)"
      onmousedown={(e) => e.preventDefault()}
    >
      <button class:active={activeFormats.bold} onclick={commands.bold} title="Bold (Ctrl+B)">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
          <path d="M6 4h8a4 4 0 0 1 4 4 4 4 0 0 1-4 4H6z"/>
          <path d="M6 12h9a4 4 0 0 1 4 4 4 4 0 0 1-4 4H6z"/>
        </svg>
      </button>
      <button class:active={activeFormats.italic} onclick={commands.italic} title="Italic (Ctrl+I)">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="19" y1="4" x2="10" y2="4"/>
          <line x1="14" y1="20" x2="5" y2="20"/>
          <line x1="15" y1="4" x2="9" y2="20"/>
        </svg>
      </button>
      <button class:active={activeFormats.underline} onclick={commands.underline} title="Underline (Ctrl+U)">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M6 4v6a6 6 0 0 0 12 0V4"/>
          <line x1="4" y1="20" x2="20" y2="20"/>
        </svg>
      </button>
      <div class="divider"></div>
      <button onclick={commands.code} title="Code">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="16 18 22 12 16 6"/>
          <polyline points="8 6 2 12 8 18"/>
        </svg>
      </button>
      <button onclick={commands.superscript} title="Superscript">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="19" y1="5" x2="5" y2="5"/>
          <line x1="12" y1="5" x2="12" y2="19"/>
          <polyline points="9 9 12 6 15 9"/>
        </svg>
      </button>
      <button onclick={commands.subscript} title="Subscript">
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
    gap: 4px;
    padding: 4px 0 8px 0;
    border-bottom: 1px solid var(--border);
    margin-bottom: 8px;
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
</style>
