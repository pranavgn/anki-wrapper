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

  // State
  let frontText = $state("");
  let backText = $state("");
  let selectedDeckId = $state<number | null>(null);
  let decks = $state<Array<{ id: number; name: string }>>([]);
  let isSaving = $state(false);
  let saved = $state(false);

  // Tag-related state
  let allTags = $state<string[]>([]);
  let currentTags = $state<string[]>([]);

  // Load decks on mount
  onMount(async () => {
    try {
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
      console.error("Error loading decks:", error);
      addToast(error instanceof Error ? error.message : "Failed to load decks", "error");
    }
  });

  // Handle keyboard shortcuts
  function handleKeydown(event: KeyboardEvent) {
    if (event.ctrlKey && event.key === "Enter") {
      event.preventDefault();
      handleSave();
    }
  }

  async function handleSave() {
    if (!selectedDeckId || !frontText.trim() || !backText.trim() || isSaving) return;

    isSaving = true;
    try {
      // add_basic_card now returns the note_id and accepts tags
      const noteId = await invoke<number>("add_basic_card", {
        deckId: selectedDeckId,
        front: frontText.trim(),
        back: backText.trim(),
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
          // Don't fail the whole save if tags fail
        }
      }

      // Clear fields and show success
      frontText = "";
      backText = "";
      currentTags = [];
      saved = true;

      // Show checkmark for 800ms then reset
      setTimeout(() => {
        saved = false;
      }, 800);

      addToast("Card added successfully!", "success");

      // Refocus on front editor
      setTimeout(() => {
        const frontEditor = document.querySelector('.rich-editor') as HTMLDivElement;
        frontEditor?.focus();
      }, 100);
    } catch (error) {
      console.error("Error saving card:", error);
      addToast(error instanceof Error ? error.message : "Failed to add card", "error");
    } finally {
      isSaving = false;
    }
  }

  // Live preview HTML
  let frontPreview = $derived.by(() => {
    if (!frontText.trim()) return "";
    return preprocessAnkiMath(frontText.trim());
  });

  let backPreview = $derived.by(() => {
    if (!backText.trim()) return "";
    return preprocessAnkiMath(backText.trim());
  });

  // Preview element ref for math rendering
  let previewEl: HTMLElement;

  // Debounced math rendering for preview
  let mathRenderTimeout: ReturnType<typeof setTimeout>;

  $effect(() => {
    // Trigger when front or back content changes
    const _ = frontText + backText;
    clearTimeout(mathRenderTimeout);
    mathRenderTimeout = setTimeout(() => {
      if (previewEl) {
        clearMathJaxCache(previewEl);
        tick().then(() => renderMath(previewEl));
      }
    }, 600); // 600ms debounce - math rendering is expensive
  });
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="grid grid-cols-1 lg:grid-cols-2 gap-8 max-w-6xl mx-auto">
  <!-- Editor Panel -->
  <div class="space-y-6">
    <!-- Front -->
    <div>
      <label class="block text-sm font-medium text-text-secondary uppercase tracking-wider mb-2">
        Front
      </label>
      <RichTextEditor
        bind:value={frontText}
        placeholder="What do you want to remember?"
        minHeight="140px"
      />
      <p class="text-xs text-text-secondary mt-1">Use \(...\) for inline math, \[...\] for block math</p>
    </div>

    <!-- Back -->
    <div>
      <label class="block text-sm font-medium text-text-secondary uppercase tracking-wider mb-2">
        Back
      </label>
      <RichTextEditor
        bind:value={backText}
        placeholder="The answer..."
        minHeight="140px"
      />
      <p class="text-xs text-text-secondary mt-1">Use \(...\) for inline math, \[...\] for block math</p>
    </div>

    <!-- Deck Selector -->
    <div>
      <div class="relative">
        <select
          bind:value={selectedDeckId}
          class="absolute inset-0 w-full h-full opacity-0 cursor-pointer z-10"
          aria-label="Select deck"
        >
          {#each decks as deck}
            <option value={deck.id}>{deck.name}</option>
          {/each}
        </select>
        <div class="relative rounded-xl bg-bg-subtle px-4 py-3 flex items-center justify-between">
          <span class="text-text-primary font-medium">
            {#if selectedDeckId}
              {decks.find(d => d.id === selectedDeckId)?.name ?? "Select deck"}
            {:else}
              Select deck
            {/if}
          </span>
          <svg class="h-4 w-4 text-text-secondary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
          </svg>
        </div>
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
        disabled={!selectedDeckId || !frontText.trim() || !backText.trim() || isSaving}
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
        Preview
      </h3>

      {#if frontPreview || backPreview}
        <div class="bg-bg-card rounded-3xl shadow-warm border border-border overflow-hidden" bind:this={previewEl}>
          <!-- Front -->
          <div class="p-10 min-h-[200px] flex items-center justify-center">
            <div class="text-center card-content">
              <p class="text-1.5xl text-text-primary leading-relaxed font-serif">
                {@html frontPreview}
              </p>
            </div>
          </div>

          <!-- Divider -->
          <div class="border-t border-border"></div>

          <!-- Back -->
          <div class="p-10 min-h-[200px] flex items-center justify-center">
            <div class="text-center card-content">
              <p class="text-xl text-text-primary leading-relaxed font-serif">
                {@html backPreview}
              </p>
            </div>
          </div>

          <!-- Tags in Preview -->
          {#if currentTags.length > 0}
            <div class="px-10 pb-6 flex flex-wrap gap-2 justify-center">
              {#each currentTags as tag}
                <span class="inline-flex bg-accent-soft text-accent text-xs rounded-full px-3 py-1 font-medium">
                  {tag}
                </span>
              {/each}
            </div>
          {/if}
        </div>
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
