<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import {
    pickAndImportApkg,
    pickAndImportColpkg,
    pickAndImportText,
    ImportError,
    type TextImportOptions,
  } from "./importer";
  import { addToast } from "./toast";

  // Props
  let {
    isOpen = false,
    collectionStatus = "loading",
    onClose,
    onImportComplete,
  }: {
    isOpen?: boolean;
    collectionStatus?: "loading" | "ready" | "error";
    onClose?: () => void;
    onImportComplete?: () => void;
  } = $props();

  // Types
  type ImportStep = "format" | "apkg" | "colpkg" | "text" | "result";
  type ImportFormat = "apkg" | "colpkg" | "text" | null;

  type DeckStat = {
    id: number;
    name: string;
    new_cards: number;
    learn_cards: number;
    review_cards: number;
  };

  type ImportLog = {
    notes_added: number;
    notes_updated: number;
    notes_skipped: number;
    decks_added: string[];
  };

  // State
  let currentStep: ImportStep = $state("format");
  let selectedFormat: ImportFormat = $state(null);
  let isLoading = $state(false);
  let errorMessage = $state("");
  let importResult: ImportLog | null = $state(null);
  let showColpkgWarning = $state(false);

  // Text import options
  let selectedDeckId = $state(1);
  let notetypeName = $state("Basic");
  let delimiter = $state<"tab" | "comma" | "semicolon">("comma");
  let htmlEnabled = $state(false);
  let duplicatePolicy = $state<"update" | "preserve" | "ignore">("ignore");

  // Available decks for text import
  let availableDecks: DeckStat[] = $state([]);

  // Reset when modal opens
  $effect(() => {
    if (isOpen) {
      currentStep = "format";
      selectedFormat = null;
      isLoading = false;
      errorMessage = "";
      importResult = null;
      showColpkgWarning = false;
      loadDecks();
    }
  });

  async function loadDecks() {
    try {
      availableDecks = await invoke<DeckStat[]>("get_deck_stats");
      if (availableDecks.length > 0) {
        selectedDeckId = availableDecks[0].id;
      }
    } catch (e) {
      console.error("Failed to load decks:", e);
    }
  }

  function selectFormat(format: ImportFormat) {
    selectedFormat = format;
    errorMessage = "";
    if (format === "apkg") {
      currentStep = "apkg";
    } else if (format === "colpkg") {
      currentStep = "colpkg";
    } else if (format === "text") {
      currentStep = "text";
    }
  }

  function goBack() {
    if (currentStep === "result") {
      currentStep = "format";
      selectedFormat = null;
      importResult = null;
    } else if (currentStep === "colpkg" && showColpkgWarning) {
      showColpkgWarning = false;
    } else {
      currentStep = "format";
      selectedFormat = null;
    }
    errorMessage = "";
  }

  async function handleApkgImport() {
    isLoading = true;
    errorMessage = "";
    try {
      importResult = await pickAndImportApkg();
      currentStep = "result";
    } catch (e) {
      if (e instanceof ImportError && e.isCancelled) {
        // User cancelled, do nothing
      } else {
        errorMessage = e instanceof Error ? e.message : "Import failed";
      }
    } finally {
      isLoading = false;
    }
  }

  function showColpkgConfirm() {
    showColpkgWarning = true;
  }

  async function handleColpkgImport() {
    isLoading = true;
    errorMessage = "";
    try {
      await pickAndImportColpkg();
      currentStep = "result";
    } catch (e) {
      if (e instanceof ImportError && e.isCancelled) {
        // User cancelled, do nothing
      } else {
        errorMessage = e instanceof Error ? e.message : "Import failed";
      }
    } finally {
      isLoading = false;
      showColpkgWarning = false;
    }
  }

  async function handleTextImport() {
    isLoading = true;
    errorMessage = "";
    try {
      const options: TextImportOptions = {
        deck_id: selectedDeckId,
        notetype_name: notetypeName,
        delimiter: delimiter === "tab" ? "\t" : delimiter === "comma" ? "," : ";",
        html_enabled: htmlEnabled,
        duplicate_policy: duplicatePolicy,
      };
      importResult = await pickAndImportText(options);
      currentStep = "result";
    } catch (e) {
      if (e instanceof ImportError && e.isCancelled) {
        // User cancelled, do nothing
      } else {
        errorMessage = e instanceof Error ? e.message : "Import failed";
      }
    } finally {
      isLoading = false;
    }
  }

  function handleDone() {
    onImportComplete?.();
    onClose?.();
  }

  function handleClose() {
    onClose?.();
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      handleClose();
    }
  }

  function getDelimiterLabel(d: string): string {
    if (d === "tab") return "Tab";
    if (d === "comma") return "Comma";
    if (d === "semicolon") return "Semicolon";
    return d;
  }
</script>

{#if isOpen}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/25 backdrop-blur-sm animate-in fade-in duration-150"
    onclick={handleBackdropClick}
    onkeydown={(e) => e.key === "Escape" && handleClose()}
  >
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="bg-bg-card rounded-3xl p-8 max-w-lg w-full mx-4 shadow-xl animate-in zoom-in-95 duration-200"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <!-- Header -->
      <div class="flex justify-between items-center mb-6">
        <h2 class="text-xl font-semibold text-text-primary">Import Deck</h2>
        <button
          class="p-2 text-text-secondary hover:text-text-primary hover:bg-bg-subtle rounded-lg transition-colors cursor-pointer"
          onclick={handleClose}
        >
          <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- Back button -->
      {#if currentStep !== "format"}
        <button
          class="mb-4 text-sm text-text-secondary hover:text-text-primary transition-colors cursor-pointer flex items-center gap-1"
          onclick={goBack}
        >
          <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
          </svg>
          Back
        </button>
      {/if}

      <!-- Step 1: Format Picker -->
      {#if currentStep === "format"}
        <div class="space-y-3">
          <button
            class="w-full p-4 border border-border rounded-xl hover:bg-bg-subtle transition-colors cursor-pointer text-left"
            onclick={() => selectFormat("apkg")}
          >
            <div class="flex justify-between items-center">
              <div>
                <div class="font-medium text-text-primary">Anki Package (.apkg)</div>
                <div class="text-sm text-text-secondary">Import one or more decks</div>
              </div>
              <span class="px-2 py-1 bg-success/10 text-success text-xs font-medium rounded-full">
                Recommended
              </span>
            </div>
          </button>

          <button
            class="w-full p-4 border border-border rounded-xl hover:bg-bg-subtle transition-colors cursor-pointer text-left"
            onclick={() => selectFormat("colpkg")}
          >
            <div class="flex justify-between items-center">
              <div>
                <div class="font-medium text-text-primary">Collection Package (.colpkg)</div>
                <div class="text-sm text-text-secondary">Replace your entire collection from a backup</div>
              </div>
              <span class="px-2 py-1 bg-danger/10 text-danger text-xs font-medium rounded-full">
                Destructive
              </span>
            </div>
          </button>

          <button
            class="w-full p-4 border border-border rounded-xl hover:bg-bg-subtle transition-colors cursor-pointer text-left"
            onclick={() => selectFormat("text")}
          >
            <div>
              <div class="font-medium text-text-primary">Text / CSV (.txt, .csv)</div>
              <div class="text-sm text-text-secondary">Tab or comma-separated flashcard data</div>
            </div>
          </button>
        </div>
      {/if}

      <!-- Step 2a: APKG Import -->
      {#if currentStep === "apkg"}
        <div class="py-8">
          <button
            class="w-full py-12 border-2 border-dashed border-border rounded-xl hover:border-accent hover:bg-accent-soft transition-colors cursor-pointer flex flex-col items-center justify-center gap-2"
            onclick={handleApkgImport}
            disabled={isLoading}
          >
            {#if isLoading}
              <svg class="h-8 w-8 text-accent animate-spin" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              <span class="text-text-secondary">Importing...</span>
            {:else}
              <svg class="h-8 w-8 text-text-secondary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" />
              </svg>
              <span class="text-text-primary font-medium">Click to choose a file</span>
              <span class="text-text-secondary text-sm">.apkg files only</span>
            {/if}
          </button>

          {#if errorMessage}
            <p class="mt-4 text-sm text-danger text-center">{errorMessage}</p>
          {/if}
        </div>
      {/if}

      <!-- Step 2b: COLPKG Import -->
      {#if currentStep === "colpkg"}
        <div class="py-8">
          {#if !showColpkgWarning}
            <button
              class="w-full py-12 border-2 border-dashed border-border rounded-xl hover:border-accent hover:bg-accent-soft transition-colors cursor-pointer flex flex-col items-center justify-center gap-2"
              onclick={showColpkgConfirm}
              disabled={isLoading}
            >
              <svg class="h-8 w-8 text-text-secondary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4" />
              </svg>
              <span class="text-text-primary font-medium">Click to choose a file</span>
              <span class="text-text-secondary text-sm">.colpkg files only</span>
            </button>
          {:else}
            <div class="border border-danger/30 bg-danger/5 rounded-xl p-4 mb-4">
              <p class="text-sm text-danger">
                This will replace your entire local collection with the contents of the .colpkg file. This cannot be undone.
              </p>
            </div>
            <div class="flex gap-3">
              <button
                class="flex-1 px-4 py-2 border border-border rounded-xl hover:bg-bg-subtle transition-colors cursor-pointer"
                onclick={() => (showColpkgWarning = false)}
              >
                Cancel
              </button>
              <button
                class="flex-1 px-4 py-2 bg-danger text-white rounded-xl hover:bg-danger/90 transition-colors cursor-pointer"
                onclick={handleColpkgImport}
                disabled={isLoading}
              >
                {isLoading ? "Importing..." : "Yes, Replace Collection"}
              </button>
            </div>
          {/if}

          {#if errorMessage}
            <p class="mt-4 text-sm text-danger text-center">{errorMessage}</p>
          {/if}
        </div>
      {/if}

      <!-- Step 2c: Text/CSV Import -->
      {#if currentStep === "text"}
        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-text-primary mb-1">Deck</label>
            <select
              class="w-full px-3 py-2 border border-border rounded-xl bg-bg-card text-text-primary focus:outline-none focus:ring-2 focus:ring-accent/30"
              bind:value={selectedDeckId}
            >
              {#each availableDecks as deck}
                <option value={deck.id}>{deck.name}</option>
              {/each}
            </select>
          </div>

          <div>
            <label class="block text-sm font-medium text-text-primary mb-1">Note Type</label>
            <input
              type="text"
              class="w-full px-3 py-2 border border-border rounded-xl bg-bg-card text-text-primary focus:outline-none focus:ring-2 focus:ring-accent/30"
              bind:value={notetypeName}
              placeholder="Basic"
            />
          </div>

          <div>
            <label class="block text-sm font-medium text-text-primary mb-2">Delimiter</label>
            <div class="flex rounded-xl border border-border overflow-hidden">
              {#each ["tab", "comma", "semicolon"] as d}
                <button
                  class="flex-1 px-3 py-2 text-sm transition-colors cursor-pointer {delimiter === d ? 'bg-accent text-white' : 'bg-bg-card text-text-primary hover:bg-bg-subtle'}"
                  onclick={() => (delimiter = d as typeof delimiter)}
                >
                  {getDelimiterLabel(d)}
                </button>
              {/each}
            </div>
          </div>

          <div class="flex items-center justify-between">
            <label class="text-sm font-medium text-text-primary">Allow HTML in fields</label>
            <button
              class="relative w-12 h-6 rounded-full transition-colors cursor-pointer {htmlEnabled ? 'bg-accent' : 'bg-border'}"
              onclick={() => (htmlEnabled = !htmlEnabled)}
            >
              <span
                class="absolute top-1 left-1 w-4 h-4 bg-white rounded-full shadow transition-transform {htmlEnabled ? 'translate-x-6' : 'translate-x-0'}"
              ></span>
            </button>
          </div>

          <div>
            <label class="block text-sm font-medium text-text-primary mb-2">Duplicate Handling</label>
            <div class="flex rounded-xl border border-border overflow-hidden">
              {#each ["update", "preserve", "ignore"] as policy}
                <button
                  class="flex-1 px-3 py-2 text-sm transition-colors cursor-pointer {duplicatePolicy === policy ? 'bg-accent text-white' : 'bg-bg-card text-text-primary hover:bg-bg-subtle'}"
                  onclick={() => (duplicatePolicy = policy as typeof duplicatePolicy)}
                >
                  {policy.charAt(0).toUpperCase() + policy.slice(1)}
                </button>
              {/each}
            </div>
          </div>

          <button
            class="w-full px-4 py-3 bg-accent text-white rounded-xl hover:bg-accent/90 transition-colors cursor-pointer disabled:opacity-50"
            onclick={handleTextImport}
            disabled={isLoading}
          >
            {isLoading ? "Importing..." : "Choose File"}
          </button>

          {#if errorMessage}
            <p class="text-sm text-danger text-center">{errorMessage}</p>
          {/if}
        </div>
      {/if}

      <!-- Step 3: Result -->
      {#if currentStep === "result" && importResult}
        <div class="text-center py-4">
          <svg class="h-16 w-16 text-success mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
          </svg>
          <h3 class="text-xl font-semibold text-text-primary mb-6">Import Complete</h3>

          <div class="grid grid-cols-2 gap-4 text-left bg-bg-subtle rounded-xl p-4 mb-6">
            <div>
              <div class="text-xs text-text-secondary uppercase tracking-wide">Notes Added</div>
              <div class="text-lg font-semibold text-text-primary">{importResult.notes_added}</div>
            </div>
            <div>
              <div class="text-xs text-text-secondary uppercase tracking-wide">Notes Updated</div>
              <div class="text-lg font-semibold text-text-primary">{importResult.notes_updated}</div>
            </div>
            <div>
              <div class="text-xs text-text-secondary uppercase tracking-wide">Notes Skipped</div>
              <div class="text-lg font-semibold text-text-primary">{importResult.notes_skipped}</div>
            </div>
            <div>
              <div class="text-xs text-text-secondary uppercase tracking-wide">Decks Added</div>
              <div class="text-lg font-semibold text-text-primary">
                {importResult.decks_added.length > 0 ? importResult.decks_added.join(", ") : "None"}
              </div>
            </div>
          </div>

          <button
            class="w-full px-4 py-3 bg-accent text-white rounded-xl hover:bg-accent/90 transition-colors cursor-pointer"
            onclick={handleDone}
          >
            Done
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}
