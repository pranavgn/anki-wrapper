<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import NeuSelect from "./ui/NeuSelect.svelte";
  import {
    pickAndImportApkg,
    pickAndImportColpkg,
    pickAndImportText,
    ImportError,
    type TextImportOptions,
  } from "./importer";
  import { addToast } from "./toast";
  import NeuDialog from "./ui/NeuDialog.svelte";

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
    short_name: string;
    level: number;
    new_count: number;
    learn_count: number;
    review_count: number;
    card_count: number;
    is_filtered: boolean;
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

  // Text import options (snake_case to match Rust struct)
  let selectedDeckId = $state(1);
  let notetypeName = $state("Basic");
  let delimiter = $state<"tab" | "comma" | "semicolon">("comma");
  let htmlEnabled = $state(false);
  let duplicatePolicy = $state<"update" | "preserve" | "ignore">("ignore");
  let importMode = $state<"csv" | "cloze">("csv");
  let clozeTags = $state("");
  let autoCloze = $state(true);

  // Available decks for imports
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
      const result = await invoke<DeckStat[]>("get_all_decks");
      availableDecks = result;
      if (availableDecks.length > 0) {
        selectedDeckId = availableDecks[0].id;
        clozeDeckId = availableDecks[0].id;
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
      if (importMode === "csv") {
        // Use snake_case keys to match the Rust TextImportOptions struct
        const options: TextImportOptions = {
          deck_id: selectedDeckId,
          notetype_name: notetypeName,
          delimiter: delimiter === "tab" ? "\t" : delimiter === "comma" ? "," : ";",
          html_enabled: htmlEnabled,
          duplicate_policy: duplicatePolicy,
        };
        importResult = await pickAndImportText(options);
      } else {
        // Cloze mode - use the same pickAndImportText function
        // The Rust backend will handle cloze detection based on file content
        const tags = clozeTags
          .split(/[,\s]+/)
          .map((t) => t.trim())
          .filter((t) => t.length > 0);

        const options: TextImportOptions = {
          deck_id: selectedDeckId,
          notetype_name: "Cloze", // Force cloze notetype
          delimiter: "\n", // Line-based for cloze text
          html_enabled: false,
          duplicate_policy: "ignore",
        };
        importResult = await pickAndImportText(options);
      }
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

  function getDelimiterLabel(d: string): string {
    if (d === "tab") return "Tab";
    if (d === "comma") return "Comma";
    if (d === "semicolon") return "Semicolon";
    return d;
  }
</script>

<NeuDialog {isOpen} onClose={handleClose} title="Import Deck" size="lg">
  <!-- Back button -->
  {#if currentStep !== "format"}
    <button
      class="back-button"
      onclick={goBack}
    >
      <svg class="back-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
      </svg>
      Back
    </button>
  {/if}

  <!-- Step 1: Format Picker -->
  {#if currentStep === "format"}
    <div class="format-grid">
      <button
        class="format-card neu-raised neu-btn"
        onclick={() => selectFormat("apkg")}
      >
        <div class="format-header">
          <div class="format-info">
            <div class="format-name">Anki Package (.apkg)</div>
            <div class="format-description">Import one or more decks</div>
          </div>
          <span class="format-badge badge-recommended">Recommended</span>
        </div>
      </button>

      <button
        class="format-card neu-raised neu-btn"
        onclick={() => selectFormat("text")}
      >
        <div class="format-info">
          <div class="format-name">Text / CSV (.csv, .tsv, .txt, .md)</div>
          <div class="format-description">Import CSV, TSV, or text files as flashcards</div>
        </div>
      </button>

      <button
        class="format-card neu-raised neu-btn"
        onclick={() => selectFormat("colpkg")}
      >
        <div class="format-header">
          <div class="format-info">
            <div class="format-name">Collection Package (.colpkg)</div>
            <div class="format-description">Replace your entire collection from a backup</div>
          </div>
          <span class="format-badge badge-destructive">Destructive</span>
        </div>
      </button>
    </div>
  {/if}

  <!-- Step 2a: APKG Import -->
  {#if currentStep === "apkg"}
    <div class="import-step">
      <button
        class="file-picker neu-pressed"
        onclick={handleApkgImport}
        disabled={isLoading}
      >
        {#if isLoading}
          <svg class="spinner" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          <span class="file-picker-text">Importing...</span>
        {:else}
          <svg class="file-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" />
          </svg>
          <span class="file-picker-text">Click to choose a file</span>
          <span class="file-picker-hint">.apkg files only</span>
        {/if}
      </button>

      {#if errorMessage}
        <p class="error-message">{errorMessage}</p>
      {/if}
    </div>
  {/if}

  <!-- Step 2b: COLPKG Import -->
  {#if currentStep === "colpkg"}
    <div class="import-step">
      {#if !showColpkgWarning}
        <button
          class="file-picker neu-pressed"
          onclick={showColpkgConfirm}
          disabled={isLoading}
        >
          <svg class="file-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4" />
          </svg>
          <span class="file-picker-text">Click to choose a file</span>
          <span class="file-picker-hint">.colpkg files only</span>
        </button>
      {:else}
        <div class="warning-box">
          <p class="warning-text">
            This will replace your entire local collection with the contents of the .colpkg file.
            This cannot be undone.
          </p>
        </div>
        <div class="warning-actions">
          <button
            class="warning-btn neu-subtle"
            onclick={() => (showColpkgWarning = false)}
          >
            Cancel
          </button>
          <button
            class="warning-btn danger-btn"
            onclick={handleColpkgImport}
            disabled={isLoading}
          >
            {isLoading ? "Importing..." : "Yes, Replace Collection"}
          </button>
        </div>
      {/if}

      {#if errorMessage}
        <p class="error-message">{errorMessage}</p>
      {/if}
    </div>
  {/if}

  <!-- Step 2c: Text Import (CSV/TSV/Cloze) -->
  {#if currentStep === "text"}
    <div class="text-import-form">
       <div class="form-group">
         <label for="import-deck-select" class="form-label">Deck</label>
         <NeuSelect
           id="import-deck-select"
           options={availableDecks.map(d => ({ value: d.id, label: d.name }))}
           bind:value={selectedDeckId}
           size="sm"
           searchable={true}
         />
       </div>

       <div class="form-group">
         <label for="import-mode-group" class="form-label">Import Mode</label>
         <div class="button-group" role="radiogroup" id="import-mode-group" aria-label="Import Mode">
           <button
             role="radio"
             aria-checked={importMode === "csv"}
             class="group-btn {importMode === 'csv' ? 'active' : 'neu-subtle'}"
             onclick={() => (importMode = "csv")}
           >
             CSV / TSV
           </button>
           <button
             role="radio"
             aria-checked={importMode === "cloze"}
             class="group-btn {importMode === 'cloze' ? 'active' : 'neu-subtle'}"
             onclick={() => (importMode = "cloze")}
           >
             Cloze Text
           </button>
         </div>
       </div>

       {#if importMode === "csv"}
         <div class="form-group">
           <label for="import-notetype" class="form-label">Note Type</label>
           <input
             id="import-notetype"
             type="text"
             class="form-input neu-pressed"
             bind:value={notetypeName}
             placeholder="Basic"
           />
         </div>

         <div class="form-group">
           <label for="delimiter-group" class="form-label">Delimiter</label>
           <div class="button-group" role="radiogroup" id="delimiter-group" aria-label="Delimiter">
             {#each ["tab", "comma", "semicolon"] as d}
               <button
                 role="radio"
                 aria-checked={delimiter === d}
                 class="group-btn {delimiter === d ? 'active' : 'neu-subtle'}"
                 onclick={() => (delimiter = d as typeof delimiter)}
               >
                 {getDelimiterLabel(d)}
               </button>
             {/each}
           </div>
         </div>

        <div class="form-row">
          <label class="form-label">Allow HTML in fields</label>
          <button
            class="toggle-switch {htmlEnabled ? 'active' : ''}"
            onclick={() => (htmlEnabled = !htmlEnabled)}
            role="switch"
            aria-checked={htmlEnabled}
          >
            <span class="toggle-knob"></span>
          </button>
        </div>

        <div class="form-group">
          <label class="form-label">Duplicate Handling</label>
          <div class="button-group">
            {#each ["update", "preserve", "ignore"] as policy}
              <button
                class="group-btn {duplicatePolicy === policy ? 'active' : 'neu-subtle'}"
                onclick={() => (duplicatePolicy = policy as typeof duplicatePolicy)}
              >
                {policy.charAt(0).toUpperCase() + policy.slice(1)}
              </button>
            {/each}
          </div>
        </div>
       {:else}
         <div class="form-group">
           <label for="cloze-tags" class="form-label">Tags (comma-separated)</label>
           <input
             id="cloze-tags"
             type="text"
             class="form-input neu-pressed"
             bind:value={clozeTags}
             placeholder="e.g. culture, exam-prep"
           />
         </div>

         <div class="form-row">
           <div>
             <label class="form-label">Auto-generate cloze deletions</label>
             <p class="form-hint">Wraps key terms in {{c1::…}} for lines without existing cloze syntax</p>
           </div>
           <button
             class="toggle-switch {autoCloze ? 'active' : ''}"
             onclick={() => (autoCloze = !autoCloze)}
             role="switch"
             aria-checked={autoCloze}
           >
             <span class="toggle-knob"></span>
           </button>
         </div>
       {/if}

      <button
        class="import-btn"
        onclick={handleTextImport}
        disabled={isLoading}
      >
        {isLoading ? "Importing..." : "Choose File"}
      </button>

      {#if errorMessage}
        <p class="error-message">{errorMessage}</p>
      {/if}
    </div>
  {/if}

  <!-- Step 2d: Cloze Text/Markdown Import -->
  {#if currentStep === "cloze"}
    <div class="text-import-form">
      <p class="cloze-intro">
        Import a text or markdown file as cloze deletion cards. Each paragraph or line becomes
        a separate card. Markdown headers and formatting are stripped automatically.
      </p>

      <div class="form-group">
        <label for="cloze-deck-select" class="form-label">Deck</label>
        <NeuSelect
          id="cloze-deck-select"
          options={availableDecks.map(d => ({ value: d.id, label: d.name }))}
          bind:value={clozeDeckId}
          size="sm"
          searchable={true}
        />
      </div>

      <div class="form-group">
        <label for="cloze-tags" class="form-label">Tags (comma-separated)</label>
        <input
          id="cloze-tags"
          type="text"
          class="form-input neu-pressed"
          bind:value={clozeTags}
          placeholder="e.g. culture, exam-prep"
        />
      </div>

      <div class="form-row">
        <div>
          <label class="form-label">Auto-generate cloze deletions</label>
          <p class="form-hint">Wraps key terms in {'{{c1::…}}'} for lines without existing cloze syntax</p>
        </div>
        <button
          class="toggle-switch {autoCloze ? 'active' : ''}"
          onclick={() => (autoCloze = !autoCloze)}
          role="switch"
          aria-checked={autoCloze}
        >
          <span class="toggle-knob"></span>
        </button>
      </div>

      <button
        class="import-btn"
        onclick={handleClozeImport}
        disabled={isLoading}
      >
        {#if isLoading}
          Importing...
        {:else}
          Choose File (.txt, .md, .csv)
        {/if}
      </button>

      {#if errorMessage}
        <p class="error-message">{errorMessage}</p>
      {/if}
    </div>
  {/if}

  <!-- Step 3: Result -->
  {#if currentStep === "result" && importResult}
    <div class="result-step">
      <svg class="result-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
      </svg>
      <h3 class="result-title">Import Complete</h3>

      <div class="result-stats neu-pressed">
        <div class="stat-item">
          <div class="stat-label">Notes Added</div>
          <div class="stat-value">{importResult.notes_added}</div>
        </div>
        <div class="stat-item">
          <div class="stat-label">Notes Updated</div>
          <div class="stat-value">{importResult.notes_updated}</div>
        </div>
        <div class="stat-item">
          <div class="stat-label">Notes Skipped</div>
          <div class="stat-value">{importResult.notes_skipped}</div>
        </div>
        <div class="stat-item">
          <div class="stat-label">Decks Added</div>
          <div class="stat-value">
            {importResult.decks_added.length > 0
              ? importResult.decks_added.join(", ")
              : "—"}
          </div>
        </div>
      </div>

      <button class="done-btn" onclick={handleDone}>
        Done
      </button>
    </div>
  {/if}
</NeuDialog>

<style>
  .back-button {
    display: flex;
    align-items: center;
    gap: 6px;
    background: none;
    border: none;
    cursor: pointer;
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-secondary);
    padding: 4px 0;
    margin-bottom: 12px;
  }

  .back-button:hover {
    color: var(--text-primary);
  }

  .back-icon {
    width: 16px;
    height: 16px;
  }

  .format-grid {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .format-card {
    width: 100%;
    padding: 16px 20px;
    text-align: left;
    border: none;
    cursor: pointer;
    border-radius: var(--radius-md);
  }

  .format-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
  }

  .format-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .format-name {
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .format-description {
    font-family: var(--sans);
    font-size: 12px;
    color: var(--text-secondary);
  }

  .format-badge {
    font-family: var(--sans);
    font-size: 10px;
    font-weight: 600;
    padding: 3px 8px;
    border-radius: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    flex-shrink: 0;
  }

  .badge-recommended {
    background: var(--accent-soft, rgba(196, 113, 79, 0.1));
    color: var(--accent);
  }

  .badge-new {
    background: rgba(59, 130, 246, 0.1);
    color: #3B82F6;
  }

  .badge-destructive {
    background: var(--danger-soft, rgba(192, 68, 74, 0.1));
    color: var(--danger, #c0444a);
  }

  .import-step {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 20px 0;
  }

  .file-picker {
    width: 100%;
    padding: 32px 20px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .file-picker:hover:not(:disabled) {
    transform: scale(1.01);
  }

  .file-picker:disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }

  .file-icon,
  .spinner {
    width: 32px;
    height: 32px;
    color: var(--text-secondary);
  }

  .spinner {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .file-picker-text {
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .file-picker-hint {
    font-family: var(--sans);
    font-size: 12px;
    color: var(--text-muted);
  }

  .warning-box {
    padding: 16px;
    background: var(--danger-soft, rgba(192, 68, 74, 0.08));
    border-radius: var(--radius-md);
  }

  .warning-text {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-primary);
    margin: 0;
  }

  .warning-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    margin-top: 12px;
  }

  .warning-btn {
    padding: 8px 16px;
    font-family: var(--sans);
    font-size: 13px;
    font-weight: 500;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }

  .danger-btn {
    background: var(--danger, #c0444a);
    color: white;
  }

  .danger-btn:disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }

  .error-message {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--danger, #c0444a);
    text-align: center;
    margin: 0;
  }

  .text-import-form {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .cloze-intro {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.5;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-label {
    font-family: var(--sans);
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
  }

  .form-hint {
    font-family: var(--sans);
    font-size: 11px;
    color: var(--text-muted);
    margin: 2px 0 0 0;
  }

  .form-select,
  .form-input {
    width: 100%;
    padding: 10px 14px;
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-primary);
    background: transparent;
    border: none;
    outline: none;
  }

  .form-select {
    cursor: pointer;
    appearance: none;
  }

  .form-input::placeholder {
    color: var(--text-muted);
  }

  .button-group {
    display: flex;
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .group-btn {
    flex: 1;
    padding: 10px 12px;
    font-family: var(--sans);
    font-size: 12px;
    font-weight: 500;
    border: none;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .group-btn.active {
    background: var(--accent);
    color: white;
  }

  .form-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 0;
    gap: 12px;
  }

  .toggle-switch {
    position: relative;
    width: 44px;
    height: 24px;
    border-radius: 12px;
    background: var(--bg-deep);
    border: none;
    cursor: pointer;
    transition: background 0.2s ease;
    box-shadow: var(--neu-down);
    flex-shrink: 0;
  }

  .toggle-switch.active {
    background: var(--accent);
  }

  .toggle-knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: white;
    box-shadow: 0 1px 3px rgba(0,0,0,0.15);
    transition: left 0.2s ease;
  }

  .toggle-switch.active .toggle-knob {
    left: 23px;
  }

  .import-btn {
    width: 100%;
    padding: 12px 20px;
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 600;
    color: white;
    background: var(--accent);
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: opacity 0.15s ease;
  }

  .import-btn:hover:not(:disabled) {
    opacity: 0.9;
  }

  .import-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .result-step {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 20px 0;
  }

  .result-icon {
    width: 48px;
    height: 48px;
    color: var(--accent);
  }

  .result-title {
    font-family: var(--sans);
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .result-stats {
    width: 100%;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    padding: 16px;
    border-radius: var(--radius-md);
  }

  .stat-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    text-align: center;
  }

  .stat-label {
    font-family: var(--sans);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }

  .stat-value {
    font-family: var(--serif, var(--sans));
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .done-btn {
    padding: 10px 32px;
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 600;
    color: white;
    background: var(--accent);
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
  }
</style>
