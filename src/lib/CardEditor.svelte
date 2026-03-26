<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, tick } from "svelte";
  import { addToast } from "./toast";
  import TagInput from "./TagInput.svelte";
  import RichTextEditor from "./RichTextEditor.svelte";
  import NeuDialog from "./ui/NeuDialog.svelte";
  import NeuSelect from "./ui/NeuSelect.svelte";
  import { renderMath, clearMathJaxCache, preprocessAnkiMath } from "./mathjax";

  // Props using Svelte 5 runes
  interface EditCardData {
    cardId: number;
    noteId: number;
    front: string;
    back: string;
    deckId: number;
    tags: string[];
  }

  interface Props {
    onBack: () => void;
    editCard?: EditCardData | null;
  }

  let { onBack, editCard = null }: Props = $props();

  // Notetype types
  interface NotetypeInfo {
    id: number;
    name: string;
    kind: string;
    field_count: number;
    template_count: number;
  }

  interface FieldInfo {
    ord: number;
    name: string;
  }

  interface NotetypeDetail {
    id: number;
    name: string;
    kind: string;
    fields: FieldInfo[];
    templates: Array<{
      ord: number;
      name: string;
      front_html: string;
      back_html: string;
    }>;
    css: string;
  }

  // State
  let notetypes = $state<NotetypeInfo[]>([]);
  let selectedNotetypeId = $state<number | null>(null);
  let notetypeDetail = $state<NotetypeDetail | null>(null);
  
  // Dynamic field values - array indexed by field ord
  let fieldValues = $state<string[]>([]);
  
  let selectedDeckId = $state<number | null>(null);
  let decks = $state<Array<{ id: number; name: string }>>([]);
  let isSaving = $state(false);
  let saved = $state(false);

  // Tag-related state
  let allTags = $state<string[]>([]);
  let currentTags = $state<string[]>([]);

  // Track current cloze number for cloze insertion
  let currentClozeNumber = $state(1);

  // Delete confirmation dialog
  let showDeleteConfirm = $state(false);

  // Determine if we're in edit mode
  let isEditMode = $derived(!!editCard);

  // Load notetypes and decks on mount
  onMount(async () => {
    try {
      // Load notetypes
      const ntInfo = await invoke<NotetypeInfo[]>("get_all_notetypes");
      notetypes = ntInfo;
      
      // Auto-select first notetype (usually Basic)
      if (notetypes.length > 0) {
        selectedNotetypeId = notetypes[0].id;
      }
      
      // Load decks
      const result = await invoke<Array<{ id: number; name: string; short_name: string; level: number; new_count: number; learn_count: number; review_count: number; card_count: number; is_filtered: boolean }>>("get_all_decks");
      decks = result.map(deck => ({ id: deck.id, name: deck.name }));
      if (decks.length > 0) {
        selectedDeckId = decks[0].id;
      }
      
      // Load all tags for suggestions
      try {
        allTags = await invoke<string[]>("get_all_tags");
      } catch (e) {
        console.error("Error loading tags:", e);
      }

      // If in edit mode, populate fields
      if (editCard) {
        selectedDeckId = editCard.deckId;
        currentTags = [...editCard.tags];
        // Field values will be set after notetype detail loads
      }
    } catch (error) {
      console.error("Error loading data:", error);
      addToast(error instanceof Error ? error.message : "Failed to load data", "error");
    }
  });

  // Load notetype detail when selected notetype changes
  $effect(() => {
    if (selectedNotetypeId) {
      loadNotetypeDetail(selectedNotetypeId);
    }
  });

  async function loadNotetypeDetail(ntid: number) {
    try {
      const detail = await invoke<NotetypeDetail>("get_notetype_detail", { notetypeId: ntid });
      notetypeDetail = detail;
      
      // Reset field values to empty strings for each field
      fieldValues = new Array(detail.fields.length).fill("");
      
      // If in edit mode, populate with existing card data
      if (editCard && detail.fields.length >= 2) {
        fieldValues[0] = editCard.front;
        fieldValues[1] = editCard.back;
      }
      
      // Reset cloze number
      currentClozeNumber = 1;
    } catch (e) {
      console.error("Error loading notetype detail:", e);
    }
  }

  // Handle keyboard shortcuts
  function handleKeydown(event: KeyboardEvent) {
    // Ctrl+Shift+C for cloze
    if (event.ctrlKey && event.shiftKey && event.key === "C") {
      event.preventDefault();
      insertCloze();
    }
    // Ctrl+Enter for save
    if (event.ctrlKey && event.key === "Enter") {
      event.preventDefault();
      handleSave();
    }
  }

  // Insert cloze syntax at cursor position in the active field
  function insertCloze() {
    // Find the active rich text editor (the one with focus or last focused)
    const editors = document.querySelectorAll('.rich-editor');
    let activeEditor: HTMLDivElement | null = null;
    
    for (const editor of editors) {
      if (editor === document.activeElement || editor.contains(document.activeElement)) {
        activeEditor = editor as HTMLDivElement;
        break;
      }
    }
    
    // If no editor has focus, use the first one
    if (!activeEditor && editors.length > 0) {
      activeEditor = editors[0] as HTMLDivElement;
    }
    
    if (!activeEditor) return;
    
    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0) return;
    
    const range = selection.getRangeAt(0);
    const selectedText = range.toString();
    
    if (selectedText) {
      // Wrap selected text in cloze syntax with current cloze number
      const clozeText = `{{c${currentClozeNumber}::${selectedText}}}`;
      
      // Replace the selected text
      range.deleteContents();
      range.insertNode(document.createTextNode(clozeText));
      
      // Move cursor after the inserted text
      range.collapse(false);
      selection.removeAllRanges();
      selection.addRange(range);
      
      // Update the field value from the editor's innerHTML
      const fieldIndex = Array.from(editors).indexOf(activeEditor);
      if (fieldIndex >= 0) {
        fieldValues[fieldIndex] = activeEditor.innerHTML;
      }
      
      // Increment cloze number for next cloze
      currentClozeNumber++;
    }
  }

  // Handle field value changes from rich text editors
  function handleFieldChange(index: number, value: string) {
    fieldValues[index] = value;
  }

  async function handleSave() {
    if (!selectedDeckId || !selectedNotetypeId || !notetypeDetail || isSaving) return;
    
    // Check that all required fields have content
    const hasContent = fieldValues.some(v => v.trim().length > 0);
    if (!hasContent) {
      addToast("Please fill in at least one field", "error");
      return;
    }

    isSaving = true;
    try {
      if (isEditMode && editCard) {
        // Update existing card
        await invoke("update_note", {
          noteId: editCard.noteId,
          fields: fieldValues,
          tags: currentTags
        });

        // Update deck if changed
        if (selectedDeckId !== editCard.deckId) {
          await invoke("set_card_deck", {
            cardId: editCard.cardId,
            deckId: selectedDeckId
          });
        }

        addToast("Card updated successfully!", "success");
      } else {
        // Add new card
        const noteId = await invoke<number>("add_note", {
          deckId: selectedDeckId,
          notetypeId: selectedNotetypeId,
          fields: fieldValues,
          tags: currentTags
        });

        // Set tags on the note if any tags were added
        if (currentTags.length > 0) {
          try {
            await invoke("set_note_tags", {
              noteId: noteId,
              tags: currentTags
            });
          } catch (tagError) {
            console.error("Error setting tags:", tagError);
          }
        }

        // Clear fields and show success
        fieldValues = new Array(notetypeDetail.fields.length).fill("");
        currentTags = [];
        currentClozeNumber = 1;

        addToast("Card added successfully!", "success");

        // Refocus on first editor
        setTimeout(() => {
          const firstEditor = document.querySelector('.rich-editor') as HTMLDivElement;
          firstEditor?.focus();
        }, 100);
      }

      saved = true;

      // Show checkmark for 800ms then reset
      setTimeout(() => {
        saved = false;
      }, 800);
    } catch (error) {
      console.error("Error saving card:", error);
      addToast(error instanceof Error ? error.message : "Failed to save card", "error");
    } finally {
      isSaving = false;
    }
  }

  async function handleDelete() {
    if (!editCard) return;
    
    try {
      await invoke("delete_note", { noteId: editCard.noteId });
      addToast("Card deleted successfully!", "success");
      showDeleteConfirm = false;
      onBack();
    } catch (error) {
      console.error("Error deleting card:", error);
      addToast(error instanceof Error ? error.message : "Failed to delete card", "error");
    }
  }

  // Check if current notetype is cloze
  let isCloze = $derived(notetypeDetail?.kind === "cloze");

  // Live preview - for cloze, show one card per cloze number
  let previews = $derived.by(() => {
    if (!notetypeDetail || fieldValues.every(v => !v.trim())) return [];
    
    if (isCloze) {
      // For cloze, generate preview for each cloze number
      const text = fieldValues[0] || "";
      if (!text.trim()) return [];
      
      // Find all unique cloze numbers
      const clozeRegex = /\{\{c(\d+)::/g;
      const clozeNumbers = new Set<number>();
      let match;
      while ((match = clozeRegex.exec(text)) !== null) {
        clozeNumbers.add(parseInt(match[1]));
      }
      
      if (clozeNumbers.size === 0) {
        // No cloze syntax yet, show basic preview
        return [{
          front: text,
          back: fieldValues[1] || ""
        }];
      }
      
      // Generate a preview for each cloze number
      return Array.from(clozeNumbers).sort((a, b) => a - b).map(cnum => {
        // Replace cloze markers for front (hide cloze) and back (show cloze)
        const frontReplaced = text
          .replace(/\{\{c(\d+)::([^}]+)\}\}/g, (match, cnumStr, content) => {
            const cn = parseInt(cnumStr);
            return cn === cnum ? `[...]` : content;
          });
        
        const backReplaced = text
          .replace(/\{\{c(\d+)::([^}]+)\}\}/g, (match, cnumStr, content) => {
            const cn = parseInt(cnumStr);
            return cn === cnum ? `<span class="cloze">${content}</span>` : content;
          });
        
        return {
          front: frontReplaced,
          back: backReplaced + (fieldValues[1] ? "<br><br>" + fieldValues[1] : "")
        };
      });
    } else {
      // For regular notypes, show front/back
      return [{
        front: fieldValues[0] || "",
        back: fieldValues[1] || ""
      }];
    }
  });

  // Preview element ref for math rendering
  let previewEl = $state<HTMLElement | null>(null);

  // Debounced math rendering for preview
  let mathRenderTimeout: ReturnType<typeof setTimeout>;

  $effect(() => {
    // Trigger when previews change
    const _ = JSON.stringify(previews);
    clearTimeout(mathRenderTimeout);
    mathRenderTimeout = setTimeout(() => {
      if (previewEl) {
        clearMathJaxCache(previewEl);
        tick().then(() => renderMath(previewEl));
      }
    }, 600);
  });

  // Determine if save should be enabled
  let canSave = $derived(() => {
    if (!selectedDeckId || !selectedNotetypeId || isSaving) return false;
    return fieldValues.some(v => v.trim().length > 0);
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="card-editor-container animate-slide-left">
  <!-- Title -->
  <h1 class="editor-title">
    {isEditMode ? "Edit Card" : "New Card"}
  </h1>

  <!-- Editor Fields -->
  <div class="editor-fields">
    <!-- Dynamic Fields -->
    {#if notetypeDetail}
      {#each notetypeDetail.fields as field, index}
        <div class="field-group">
          <label class="field-label" for="field-{index}">
              {field.name}
              {#if isCloze && index === 0}
                <span class="cloze-hint">(Use {'{{c1::...}}'} for cloze)</span>
              {/if}
            </label>
          <div class="editor-container neu-pressed">
            <RichTextEditor
              value={fieldValues[index] || ""}
              onchange={(v) => handleFieldChange(index, v)}
              placeholder={index === 0 ? "Question / Text" : "Answer / Extra"}
              minHeight="140px"
            />
          </div>
        </div>
      {/each}
      
      <!-- Cloze helper button -->
      {#if isCloze}
        <button
          onclick={insertCloze}
          class="cloze-btn neu-subtle neu-btn"
        >
          <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
          </svg>
          Insert Cloze {currentClozeNumber}
          <span class="cloze-shortcut">(Ctrl+Shift+C)</span>
        </button>
        <p class="cloze-info">
          Current cloze number: {currentClozeNumber}. After inserting a cloze, the number will auto-increment.
        </p>
      {/if}
    {/if}

    <!-- Deck Selector -->
    <div class="field-group">
      <label class="field-label" for="deck-select">Deck</label>
      <div class="deck-selector">
        <NeuSelect
          options={decks.map(d => ({ value: d.id, label: d.name }))}
          bind:value={selectedDeckId}
          size="sm"
          searchable={true}
          aria-label="Select deck"
        />
      </div>
    </div>

    <!-- Tags -->
    <div class="field-group">
      <label class="field-label" for="tags-input">Tags</label>
      <div class="tags-container neu-pressed">
        <TagInput bind:tags={currentTags} suggestions={allTags} placeholder="Add tag..." />
      </div>
    </div>

    <!-- Save Button -->
    <div class="save-section">
      <button
        onclick={handleSave}
        disabled={!canSave()}
        class="save-btn"
      >
        {#if saved}
          <span class="saved-text">✓ Saved!</span>
        {:else if isSaving}
          Saving...
        {:else}
          {isEditMode ? "Update Card" : "Save Card"}
        {/if}
      </button>
      <p class="save-hint">Ctrl+Enter to save</p>

      <!-- Delete Button (Edit Mode Only) -->
      {#if isEditMode}
        <button
          onclick={() => showDeleteConfirm = true}
          class="delete-btn"
        >
          Delete Card
        </button>
      {/if}
    </div>
  </div>

  <!-- Live Preview -->
  {#if previews.length > 0}
    <div class="preview-section">
      <h2 class="preview-title">
        Preview {previews.length > 1 ? `(${previews.length} cards)` : ""}
      </h2>

      {#each previews as preview, idx}
        {#if previews.length > 1}
          <p class="preview-card-label">Card {idx + 1} of {previews.length}</p>
        {/if}
        <div class="preview-card neu-raised" bind:this={previewEl}>
          <!-- Front -->
          <div class="preview-front">
            <div class="preview-content">
              <p class="preview-text">
                {@html preprocessAnkiMath(preview.front)}
              </p>
            </div>
          </div>

          <!-- Divider -->
          <div class="preview-divider"></div>

          <!-- Back -->
          <div class="preview-back">
            <div class="preview-content">
              <p class="preview-text">
                {@html preprocessAnkiMath(preview.back)}
              </p>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Delete Confirmation Dialog -->
<NeuDialog
  isOpen={showDeleteConfirm}
  onClose={() => showDeleteConfirm = false}
  title="Delete Card"
  size="sm"
>
  <div class="delete-dialog-content">
    <p class="delete-message">
      Are you sure you want to delete this card? This action cannot be undone.
    </p>
    <div class="delete-actions">
      <button
        onclick={() => showDeleteConfirm = false}
        class="cancel-btn neu-subtle neu-btn"
      >
        Cancel
      </button>
      <button
        onclick={handleDelete}
        class="confirm-delete-btn"
      >
        Delete
      </button>
    </div>
  </div>
</NeuDialog>

<style>
  .card-editor-container {
    max-width: 620px;
    margin: 0 auto;
    padding: 44px 24px;
  }

  .animate-slide-left {
    animation: slideLeft 0.3s ease-out;
  }

  @keyframes slideLeft {
    from {
      opacity: 0;
      transform: translateX(18px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }

  .editor-title {
    font-family: var(--serif);
    font-size: 28px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 32px 0;
    letter-spacing: -0.02em;
  }

  .editor-fields {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .field-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .field-label {
    font-family: var(--sans);
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-muted);
  }

  .cloze-hint {
    font-weight: 400;
    text-transform: none;
    letter-spacing: normal;
    color: var(--text-secondary);
    margin-left: 8px;
  }

  .editor-container {
    padding: 0;
    overflow: hidden;
  }

  .deck-selector {
    position: relative;
    display: flex;
    align-items: center;
  }


  .tags-container {
    padding: 12px;
  }

  .cloze-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    font-family: var(--sans);
    font-size: 13px;
    font-weight: 500;
    color: var(--accent);
    cursor: pointer;
    border: none;
    width: fit-content;
  }

  .cloze-shortcut {
    font-size: 11px;
    opacity: 0.6;
  }

  .cloze-info {
    font-family: var(--sans);
    font-size: 12px;
    color: var(--text-secondary);
    margin: 4px 0 0 0;
  }

  .save-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    margin-top: 8px;
  }

  .save-btn {
    width: 100%;
    padding: 16px 24px;
    font-family: var(--serif);
    font-size: 18px;
    font-weight: 600;
    color: white;
    background: linear-gradient(135deg, var(--accent), color-mix(in srgb, var(--accent) 80%, #000));
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: 0 4px 12px rgba(196, 113, 79, 0.3);
  }

  .save-btn:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 6px 16px rgba(196, 113, 79, 0.4);
  }

  .save-btn:active:not(:disabled) {
    transform: translateY(0);
  }

  .save-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    box-shadow: none;
  }

  .saved-text {
    color: white;
  }

  .save-btn:has(.saved-text) {
    background: var(--success);
    box-shadow: 0 4px 12px rgba(107, 143, 113, 0.3);
  }

  .save-hint {
    font-family: var(--sans);
    font-size: 11px;
    color: var(--text-muted);
    margin: 0;
  }

  .delete-btn {
    width: 100%;
    padding: 14px 24px;
    font-family: var(--sans);
    font-size: 15px;
    font-weight: 500;
    color: var(--danger);
    background: transparent;
    border: 1px solid var(--danger);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.2s ease;
    margin-top: 8px;
  }

  .delete-btn:hover {
    background: var(--danger);
    color: white;
  }

  .preview-section {
    margin-top: 40px;
  }

  .preview-title {
    font-family: var(--sans);
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-muted);
    margin: 0 0 16px 0;
  }

  .preview-card-label {
    font-family: var(--sans);
    font-size: 12px;
    color: var(--text-secondary);
    margin: 0 0 8px 0;
  }

  .preview-card {
    overflow: hidden;
    margin-bottom: 16px;
  }

  .preview-front {
    padding: 28px 24px;
    min-height: 120px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .preview-divider {
    height: 1px;
    background: color-mix(in srgb, var(--border) 30%, transparent);
  }

  .preview-back {
    padding: 28px 24px;
    min-height: 120px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-card-raised);
  }

  .preview-content {
    width: 100%;
    text-align: center;
  }

  .preview-text {
    font-family: var(--serif);
    font-size: 20px;
    line-height: 1.7;
    color: var(--text-primary);
    margin: 0;
  }

  .preview-back .preview-text {
    font-size: 18px;
  }

  .preview-card :global(img) {
    max-width: 100%;
    height: auto;
  }

  .preview-card :global(audio) {
    width: 100%;
  }

  .preview-card :global(.cloze) {
    font-weight: 600;
    color: var(--accent);
  }

  .preview-card :global(.cloze-deleted) {
    background-color: var(--accent-soft);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
  }

  /* Delete Dialog */
  .delete-dialog-content {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .delete-message {
    font-family: var(--sans);
    font-size: 15px;
    line-height: 1.6;
    color: var(--text-secondary);
    margin: 0;
  }

  .delete-actions {
    display: flex;
    gap: 12px;
  }

  .cancel-btn {
    flex: 1;
    padding: 12px 20px;
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 500;
    color: var(--text-secondary);
    cursor: pointer;
    border: none;
  }

  .confirm-delete-btn {
    flex: 1;
    padding: 12px 20px;
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 500;
    color: white;
    background: var(--danger);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .confirm-delete-btn:hover {
    opacity: 0.9;
  }
</style>
