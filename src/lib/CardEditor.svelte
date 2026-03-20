<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, tick } from "svelte";
  import { addToast } from "./toast";
  import TagInput from "./TagInput.svelte";
  import RichTextEditor from "./RichTextEditor.svelte";
  import { renderMath, clearMathJaxCache, preprocessAnkiMath } from "./mathjax";

  // Props using Svelte 5 runes
  interface Props {
    onBack: () => void;
  }

  let { onBack }: Props = $props();

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
      const deckStats = await invoke<Array<{ id: number; name: string; new_cards: number; learn_cards: number; review_cards: number }>>("get_deck_stats");
      decks = deckStats.map(deck => ({ id: deck.id, name: deck.name }));
      if (decks.length > 0) {
        selectedDeckId = decks[0].id;
      }
      
      // Load all tags for suggestions
      try {
        allTags = await invoke<string[]>("get_all_tags");
      } catch (e) {
        console.error("Error loading tags:", e);
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
      // Use the new generic add_note command
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
      saved = true;

      // Show checkmark for 800ms then reset
      setTimeout(() => {
        saved = false;
      }, 800);

      addToast("Card added successfully!", "success");

      // Refocus on first editor
      setTimeout(() => {
        const firstEditor = document.querySelector('.rich-editor') as HTMLDivElement;
        firstEditor?.focus();
      }, 100);
    } catch (error) {
      console.error("Error saving card:", error);
      addToast(error instanceof Error ? error.message : "Failed to add card", "error");
    } finally {
      isSaving = false;
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
  let previewEl: HTMLElement;

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

<svelte:window on:keydown={handleKeydown} />

<div class="grid grid-cols-1 lg:grid-cols-2 gap-8 max-w-6xl mx-auto">
  <!-- Editor Panel -->
  <div class="space-y-6">
    <!-- Notetype Selector -->
    <div>
      <label class="block text-sm font-medium text-text-secondary uppercase tracking-wider mb-2">
        Notetype
      </label>
      <div class="relative">
        <select
          bind:value={selectedNotetypeId}
          class="w-full rounded-xl bg-bg-subtle px-4 py-3 text-text-primary font-medium appearance-none cursor-pointer"
          aria-label="Select notetype"
        >
          {#each notetypes as nt}
            <option value={nt.id}>{nt.name}</option>
          {/each}
        </select>
        <svg class="absolute right-4 top-1/2 -translate-y-1/2 h-4 w-4 text-text-secondary pointer-events-none" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
        </svg>
      </div>
    </div>

    <!-- Dynamic Fields -->
    {#if notetypeDetail}
      {#each notetypeDetail.fields as field, index}
        <div>
          <label class="block text-sm font-medium text-text-secondary uppercase tracking-wider mb-2">
            {field.name}
            {#if isCloze && index === 0}
              <span class="text-xs font-normal normal-case text-text-secondary">(Use {'{{c1::...}}'} for cloze)</span>
            {/if}
          </label>
          <RichTextEditor
            value={fieldValues[index] || ""}
            onchange={(v) => handleFieldChange(index, v)}
            placeholder={index === 0 ? "Question / Text" : "Answer / Extra"}
            minHeight="140px"
          />
        </div>
      {/each}
      
      <!-- Cloze helper button -->
      {#if isCloze}
        <button
          onclick={insertCloze}
          class="flex items-center gap-2 px-3 py-1.5 rounded-xl bg-accent/10 text-accent hover:bg-accent/20 text-sm"
        >
          <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
          </svg>
          Insert Cloze {{currentClozeNumber}}
          <span class="text-xs opacity-60">(Ctrl+Shift+C)</span>
        </button>
        <p class="text-xs text-text-secondary">
          Current cloze number: {currentClozeNumber}. After inserting a cloze, the number will auto-increment.
        </p>
      {/if}
    {/if}

    <!-- Deck Selector -->
    <div>
      <label class="block text-sm font-medium text-text-secondary uppercase tracking-wider mb-2">
        Deck
      </label>
      <div class="relative">
        <select
          bind:value={selectedDeckId}
          class="w-full rounded-xl bg-bg-subtle px-4 py-3 text-text-primary font-medium appearance-none cursor-pointer"
          aria-label="Select deck"
        >
          {#each decks as deck}
            <option value={deck.id}>{deck.name}</option>
          {/each}
        </select>
        <svg class="absolute right-4 top-1/2 -translate-y-1/2 h-4 w-4 text-text-secondary pointer-events-none" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
        </svg>
      </div>
    </div>

    <!-- Tags -->
    <div>
      <label class="block text-sm font-medium text-text-secondary uppercase tracking-wider mb-2">
        Tags
      </label>
      <TagInput bind:tags={currentTags} suggestions={allTags} placeholder="Add tag..." />
    </div>

    <!-- Save Button -->
    <div>
      <button
        onclick={handleSave}
        disabled={!canSave()}
        class="w-full py-3 px-6 bg-accent text-white rounded-xl hover:bg-accent/90 disabled:bg-bg-subtle disabled:text-text-secondary font-medium transition-colors cursor-pointer active:scale-95"
      >
        {#if saved}
          <svg class="w-5 h-5 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
          </svg>
        {:else if isSaving}
          Saving...
        {:else}
          Save Card
        {/if}
      </button>
      <p class="text-xs text-text-secondary text-center mt-2">
        Ctrl+Enter to save
      </p>
    </div>
  </div>

  <!-- Live Preview Panel -->
  <div class="space-y-6">
    <div>
      <h3 class="text-sm font-medium text-text-secondary uppercase tracking-wider mb-4">
        Preview {previews.length > 1 ? `(${previews.length} cards)` : ""}
      </h3>

      {#if previews.length > 0}
        {#each previews as preview, idx}
          {#if previews.length > 1}
            <p class="text-xs text-text-secondary mb-2">Card {idx + 1} of {previews.length}</p>
          {/if}
          <div class="bg-bg-card rounded-3xl shadow-warm border border-border overflow-hidden mb-4" bind:this={previewEl}>
            <!-- Front -->
            <div class="p-10 min-h-[150px] flex items-center justify-center">
              <div class="text-center card-content">
                <p class="text-1.5xl text-text-primary leading-relaxed font-serif">
                  {@html preprocessAnkiMath(preview.front)}
                </p>
              </div>
            </div>

            <!-- Divider -->
            <div class="border-t border-border"></div>

            <!-- Back -->
            <div class="p-10 min-h-[150px] flex items-center justify-center">
              <div class="text-center card-content">
                <p class="text-xl text-text-primary leading-relaxed font-serif">
                  {@html preprocessAnkiMath(preview.back)}
                </p>
              </div>
            </div>
          </div>
        {/each}
      {:else}
        <div class="bg-bg-card rounded-3xl shadow-warm border border-border p-10 min-h-[400px] flex items-center justify-center">
          <p class="text-text-secondary italic text-center">
            Your card will appear here
          </p>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .card-content :global(img) {
    max-width: 100%;
    height: auto;
  }

  .card-content :global(audio) {
    width: 100%;
  }

  .card-content :global(.cloze) {
    font-weight: 600;
    color: var(--accent);
  }

  .card-content :global(.cloze-deleted) {
    background-color: var(--accent-soft);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
  }
</style>
