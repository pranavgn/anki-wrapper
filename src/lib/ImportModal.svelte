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

  type NotetypeOption = {
    id: number;
    name: string;
    kind: string;
  };

  type ImportLog = {
    notes_added: number;
    notes_updated: number;
    notes_skipped: number;
    decks_added: string[];
    notetype_used?: string;
  };

  // State
  let currentStep: ImportStep = $state("format");
  let selectedFormat: ImportFormat = $state(null);
  let isLoading = $state(false);
  let errorMessage = $state("");
  let importResult: ImportLog | null = $state(null);
  let showColpkgWarning = $state(false);

  // Text import options (unified for txt, csv, AND cloze)
  let selectedDeckId = $state(1);
  let selectedNotetypeId = $state(0);
  let notetypeName = $state("Basic");
  let delimiter = $state<"tab" | "comma" | "semicolon">("comma");
  let htmlEnabled = $state(false);
  let duplicatePolicy = $state<"update" | "preserve" | "ignore">("ignore");

  // Available decks & notetypes
  let availableDecks: DeckStat[] = $state([]);
  let availableNotetypes: NotetypeOption[] = $state([]);

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
      loadNotetypes();
    }
  });

  async function loadDecks() {
    try {
      const result = await invoke<DeckStat[]>("get_all_decks");
      availableDecks = result;
      if (availableDecks.length > 0) {
        selectedDeckId = availableDecks[0].id;
      }
    } catch (e) {
      console.error("Failed to load decks:", e);
    }
  }

  async function loadNotetypes() {
    try {
      const result = await invoke<NotetypeOption[]>("get_all_notetypes");
      availableNotetypes = result;
      // Default to "Basic" if available
      const basic = result.find((n) => n.name === "Basic");
      if (basic) {
        selectedNotetypeId = basic.id;
        notetypeName = basic.name;
      } else if (result.length > 0) {
        selectedNotetypeId = result[0].id;
        notetypeName = result[0].name;
      }
    } catch (e) {
      console.error("Failed to load notetypes:", e);
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
        // User cancelled
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
        // User cancelled
      } else {
        errorMessage = e instanceof Error ? e.message : "Import failed";
      }
    } finally {
      isLoading = false;
      showColpkgWarning = false;
    }
  }

  /**
   * Unified text import handler.
   * Works for .txt, .csv, .tsv AND .cloze files — the notetype picker
   * lets the user choose "Cloze" (or any other notetype) and
   * anki-core's import_csv handles the rest.
   */
  async function handleTextImport() {
    isLoading = true;
    errorMessage = "";
    try {
      const options: TextImportOptions = {
        deckId: selectedDeckId,
        notetypeName: notetypeName,
        delimiter:
          delimiter === "tab" ? "\t" : delimiter === "comma" ? "," : ";",
        htmlEnabled: htmlEnabled,
        duplicatePolicy: duplicatePolicy,
      };
      importResult = await pickAndImportText(options);
      currentStep = "result";
    } catch (e) {
      if (e instanceof ImportError && e.isCancelled) {
        // User cancelled
      } else {
        errorMessage = e instanceof Error ? e.message : "Import failed";
      }
    } finally {
      isLoading = false;
    }
  }

  function handleNotetypeChange(ntId: number) {
    selectedNotetypeId = ntId;
    const nt = availableNotetypes.find((n) => n.id === ntId);
    if (nt) notetypeName = nt.name;
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

  function getNotetypeHint(kind: string): string {
    if (kind === "cloze") return "cloze";
    return "";
  }
</script>

<NeuDialog {isOpen} onClose={handleClose} title="Import" size="lg">
  <!-- Back button -->
  {#if currentStep !== "format"}
    <button class="back-button" onclick={goBack}>
      <svg
        class="back-icon"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M15 19l-7-7 7-7"
        />
      </svg>
      Back
    </button>
  {/if}

  <!-- ───────── Step 1: Format Picker ───────── -->
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
        onclick={() => selectFormat("colpkg")}
      >
        <div class="format-header">
          <div class="format-info">
            <div class="format-name">Collection Package (.colpkg)</div>
            <div class="format-description"
              >Replace your entire collection from a backup</div
            >
          </div>
          <span class="format-badge badge-destructive">Destructive</span>
        </div>
      </button>

      <button
        class="format-card neu-raised neu-btn"
        onclick={() => selectFormat("text")}
      >
        <div class="format-info">
          <div class="format-name">Text / CSV / Cloze</div>
          <div class="format-description">
            Import .txt, .csv, .tsv, or .cloze flashcard data — supports Basic
            and Cloze notetypes
          </div>
        </div>
      </button>
    </div>
  {/if}

  <!-- ───────── Step 2a: APKG ───────── -->
  {#if currentStep === "apkg"}
    <div class="import-step">
      <button
        class="file-picker neu-pressed"
        onclick={handleApkgImport}
        disabled={isLoading}
      >
        {#if isLoading}
          <svg class="spinner" fill="none" viewBox="0 0 24 24">
            <circle
              class="opacity-25"
              cx="12"
              cy="12"
              r="10"
              stroke="currentColor"
              stroke-width="4"
            ></circle>
            <path
              class="opacity-75"
              fill="currentColor"
              d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
            ></path>
          </svg>
          <span class="file-picker-text">Importing...</span>
        {:else}
          <svg
            class="file-icon"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"
            />
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

  <!-- ───────── Step 2b: COLPKG ───────── -->
  {#if currentStep === "colpkg"}
    <div class="import-step">
      {#if !showColpkgWarning}
        <button
          class="file-picker neu-pressed"
          onclick={showColpkgConfirm}
          disabled={isLoading}
        >
          <svg
            class="file-icon"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4"
            />
          </svg>
          <span class="file-picker-text">Click to choose a file</span>
          <span class="file-picker-hint">.colpkg files only</span>
        </button>
      {:else}
        <div class="warning-box">
          <p class="warning-text">
            This will replace your entire local collection with the contents of
            the .colpkg file. This cannot be undone.
          </p>
        </div>
        <div class="warning-actions">
          <button
            class="warning-btn neu-subtle"
            onclick={() => (showColpkgWarning = false)}>Cancel</button
          >
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

  <!-- ───────── Step 2c: Unified Text / CSV / Cloze Import ───────── -->
  {#if currentStep === "text"}
    <div class="text-import-form">
      <div class="form-group">
        <label for="import-deck-select" class="form-label">Deck</label>
        <NeuSelect
          id="import-deck-select"
          options={availableDecks.map((d) => ({ value: d.id, label: d.name }))}
          bind:value={selectedDeckId}
          size="sm"
          searchable={true}
        />
      </div>

      <!-- Notetype selector — includes Cloze and all custom notetypes -->
      <div class="form-group">
        <label for="import-notetype-select" class="form-label">
          Note Type
          {#if availableNotetypes.find((n) => n.id === selectedNotetypeId)?.kind === "cloze"}
            <span class="notetype-badge">cloze</span>
          {/if}
        </label>
        <NeuSelect
          id="import-notetype-select"
          options={availableNotetypes.map((n) => ({
            value: n.id,
            label: n.name + (n.kind === "cloze" ? "  ⌁" : ""),
          }))}
          value={selectedNotetypeId}
          onchange={(v) => handleNotetypeChange(v)}
          size="sm"
          searchable={true}
        />
        <span class="form-hint">
          Choose "Cloze" for files with <code>{`{{c1::...}}`}</code> syntax
        </span>
      </div>

      <div class="form-group">
        <label for="delimiter-group" class="form-label">Delimiter</label>
        <div
          class="button-group"
          role="radiogroup"
          id="delimiter-group"
          aria-label="Delimiter"
        >
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
              class="group-btn {duplicatePolicy === policy
                ? 'active'
                : 'neu-subtle'}"
              onclick={() =>
                (duplicatePolicy = policy as typeof duplicatePolicy)}
            >
              {policy.charAt(0).toUpperCase() + policy.slice(1)}
            </button>
          {/each}
        </div>
      </div>

      <button
        class="import-btn"
        onclick={handleTextImport}
        disabled={isLoading}
      >
        {isLoading ? "Importing..." : "Choose File & Import"}
      </button>

      {#if errorMessage}
        <p class="error-message">{errorMessage}</p>
      {/if}
    </div>
  {/if}

  <!-- ───────── Step 3: Result ───────── -->
  {#if currentStep === "result" && importResult}
    <div class="result-step">
      <svg
        class="result-icon"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M5 13l4 4L19 7"
        />
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
        {#if importResult.notetype_used}
          <div class="stat-item">
            <div class="stat-label">Note Type</div>
            <div class="stat-value">{importResult.notetype_used}</div>
          </div>
        {/if}
      </div>

      <button class="import-btn" onclick={handleDone}>Done</button>
    </div>
  {/if}
</NeuDialog>

<style>
  .back-button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    font-family: var(--sans);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    background: none;
    border: none;
    cursor: pointer;
    border-radius: var(--radius-sm);
    margin-bottom: 12px;
  }

  .back-button:hover {
    color: var(--text-primary);
    background: var(--bg-subtle);
  }

  .back-icon {
    width: 14px;
    height: 14px;
  }

  .format-grid {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .format-card {
    width: 100%;
    text-align: left;
    padding: 16px 20px;
    border: none;
    cursor: pointer;
    border-radius: var(--radius-md);
    transition: transform 0.1s ease;
  }

  .format-card:hover {
    transform: translateY(-1px);
  }

  .format-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
  }

  .format-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
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
    color: var(--text-muted);
  }

  .format-badge {
    font-family: var(--sans);
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 3px 8px;
    border-radius: 4px;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .badge-recommended {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--accent);
  }

  .badge-destructive {
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    color: var(--danger);
  }

  .import-step {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
  }

  .file-picker {
    width: 100%;
    padding: 32px 24px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .file-picker:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .file-icon {
    width: 32px;
    height: 32px;
    color: var(--text-muted);
  }

  .file-picker-text {
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .file-picker-hint {
    font-family: var(--sans);
    font-size: 11px;
    color: var(--text-muted);
  }

  .spinner {
    width: 24px;
    height: 24px;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .warning-box {
    padding: 16px;
    background: color-mix(in srgb, var(--danger) 8%, transparent);
    border-radius: var(--radius-md);
    border: 1px solid color-mix(in srgb, var(--danger) 20%, transparent);
  }

  .warning-text {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-primary);
    line-height: 1.5;
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
    background: var(--danger);
    color: white;
  }

  .danger-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .error-message {
    font-family: var(--sans);
    font-size: 12px;
    color: var(--danger);
    margin-top: 4px;
  }

  /* ── Unified text/cloze import form ── */

  .text-import-form {
    display: flex;
    flex-direction: column;
    gap: 16px;
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
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .notetype-badge {
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    padding: 1px 5px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--accent) 15%, transparent);
    color: var(--accent);
    letter-spacing: 0.04em;
  }

  .form-hint {
    font-family: var(--sans);
    font-size: 11px;
    color: var(--text-muted);
    font-style: italic;
  }

  .form-hint code {
    font-family: "SF Mono", Monaco, Consolas, monospace;
    font-size: 10px;
    background: var(--bg-deep);
    padding: 1px 4px;
    border-radius: 3px;
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
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
    transition: left 0.2s ease;
  }

  .toggle-switch.active .toggle-knob {
    left: 23px;
  }

  .import-btn {
    width: 100%;
    padding: 12px;
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 600;
    color: white;
    background: var(--accent);
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .import-btn:hover {
    background: color-mix(in srgb, var(--accent) 88%, black);
  }

  .import-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* ── Result ── */

  .result-step {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
  }

  .result-icon {
    width: 48px;
    height: 48px;
    color: var(--success, #34c759);
  }

  .result-title {
    font-family: var(--sans);
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
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
  }

  .stat-label {
    font-family: var(--sans);
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  .stat-value {
    font-family: var(--sans);
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
  }
</style>
