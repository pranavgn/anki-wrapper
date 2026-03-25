<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { addToast } from "./toast";
  import NeuDialog from "./ui/NeuDialog.svelte";
  import NeuSelect from "./ui/NeuSelect.svelte";

  interface Props {
    deckId: number;
    deckName: string;
    isFiltered?: boolean;
    isOpen: boolean;
    onClose: () => void;
  }

  let { deckId, deckName, isFiltered = false, isOpen, onClose }: Props = $props();

  interface DeckOptions {
    configId: number;
    name: string;
    newCardsPerDay: number;
    learningSteps: number[];
    graduatingInterval: number;
    easyInterval: number;
    maxReviewsPerDay: number;
    easyBonus: number;
    intervalModifier: number;
    maximumInterval: number;
    fsrsEnabled: boolean;
    fsrsWeights: number[];
    desiredRetention: number;
    lapseSteps: number[];
    lapseMinimumInterval: number;
    leechThreshold: number;
  }

  let isLoading = $state(true);
  let leechAction = $state<"suspend" | "tag">("suspend");
  let opts: DeckOptions = $state({
    configId: 0,
    name: "",
    newCardsPerDay: 20,
    learningSteps: [1, 10],
    graduatingInterval: 1,
    easyInterval: 4,
    maxReviewsPerDay: 200,
    easyBonus: 1.3,
    intervalModifier: 1.0,
    maximumInterval: 36500,
    fsrsEnabled: true,
    fsrsWeights: [],
    desiredRetention: 0.9,
    lapseSteps: [10],
    lapseMinimumInterval: 1,
    leechThreshold: 8,
  });

  let newLearningStep = $state("");
  let newLapseStep = $state("");
  let validationErrors: string[] = $state([]);
  
  // FSRS optimization state
  let isOptimizing = $state(false);
  let optimizationResult = $state<{
    weights: number[];
    current_retention: number;
    predicted_retention: number;
    review_count: number;
    success: boolean;
  } | null>(null);
  
  async function handleOptimizeFsrs() {
    isOptimizing = true;
    optimizationResult = null;
    
    try {
      const result = await invoke<{
        weights: number[];
        current_retention: number;
        predicted_retention: number;
        review_count: number;
        success: boolean;
      }>("optimize_fsrs_weights", {
        deckId: deckId,
        desiredRetention: opts.desiredRetention,
      });
      
      optimizationResult = result;
      
      if (result.success && result.weights.length > 0) {
        // Apply the optimized weights
        opts.fsrsWeights = result.weights;
        addToast("FSRS weights optimized successfully", "success");
      }
    } catch (e) {
      addToast(`Optimization failed: ${e}`, "error");
    } finally {
      isOptimizing = false;
    }
  }

  onMount(async () => {
    try {
      const result = await invoke<DeckOptions>("get_deck_options", { deckId });
      opts = result;
    } catch (e) {
      console.error("Error loading deck options:", e);
      addToast("Failed to load deck options", "error");
    } finally {
      isLoading = false;
    }
  });

  function addLearningStep() {
    const step = parseFloat(newLearningStep);
    if (!isNaN(step) && step > 0) {
      opts.learningSteps = [...opts.learningSteps, step].sort((a, b) => a - b);
      newLearningStep = "";
    }
  }

  function removeLearningStep(index: number) {
    opts.learningSteps = opts.learningSteps.filter((_, i) => i !== index);
  }

  function addLapseStep() {
    const step = parseFloat(newLapseStep);
    if (!isNaN(step) && step > 0) {
      opts.lapseSteps = [...opts.lapseSteps, step].sort((a, b) => a - b);
      newLapseStep = "";
    }
  }

  function removeLapseStep(index: number) {
    opts.lapseSteps = opts.lapseSteps.filter((_, i) => i !== index);
  }

  function resetFsrsWeights() {
    opts.fsrsWeights = [];
  }

  async function rebuildFilteredDeck() {
    try {
      const count = await invoke<number>("rebuild_filtered_deck", { deckId });
      addToast(`Rebuilt with ${count} cards`, "success");
    } catch (e) {
      console.error("Error rebuilding filtered deck:", e);
      addToast("Failed to rebuild deck", "error");
    }
  }

  async function emptyFilteredDeck() {
    try {
      await invoke("empty_filtered_deck", { deckId });
      addToast("Deck emptied - cards returned to original decks", "success");
    } catch (e) {
      console.error("Error emptying filtered deck:", e);
      addToast("Failed to empty deck", "error");
    }
  }

  function validateOptions(): boolean {
    validationErrors = [];
    
    if (opts.newCardsPerDay < 0) {
      validationErrors.push("New cards per day must be >= 0");
    }
    
    if (opts.learningSteps.length === 0 || opts.learningSteps.some(s => s <= 0)) {
      validationErrors.push("Learning steps must contain positive numbers");
    }
    
    if (opts.graduatingInterval < 0) {
      validationErrors.push("Graduating interval must be >= 0");
    }
    
    if (opts.desiredRetention < 0.70 || opts.desiredRetention > 0.97) {
      validationErrors.push("Desired retention must be between 70% and 97%");
    }
    
    if (opts.maxReviewsPerDay < 0) {
      validationErrors.push("Max reviews per day must be >= 0");
    }
    
    if (opts.lapseSteps.some(s => s <= 0)) {
      validationErrors.push("Relearning steps must contain positive numbers");
    }
    
    return validationErrors.length === 0;
  }

  async function saveOptions() {
    if (!validateOptions()) {
      addToast(validationErrors.join("; "), "error");
      return;
    }
    
    try {
      await invoke("save_deck_options", { deckId, opts });
      addToast("Options saved successfully", "success");
      onClose();
    } catch (e) {
      addToast(`Failed to save: ${e}`, "error");
    }
  }
</script>

<NeuDialog {isOpen} {onClose} title="Deck Options" size="sm">
  <div class="deck-options">
    <p class="deck-name">{deckName}</p>
    
    <!-- Warning about stub implementation -->
    <p class="text-xs text-warning mb-4" style="color: var(--warning);">
      Note: Deck option changes are not yet persisted. This feature is under development.
    </p>

    {#if isLoading}
      <div class="loading-state">
        {#each {length: 5} as _}
          <div class="skeleton-line"></div>
        {/each}
      </div>
    {:else if isFiltered}
      <!-- Filtered Deck Options -->
      <div class="filtered-deck-section">
        <h3 class="section-title">Filtered Deck Actions</h3>
        <p class="section-description">
          This is a filtered deck. Use these actions to manage the cards in this deck.
        </p>
        <div class="action-buttons">
          <button
            onclick={rebuildFilteredDeck}
            class="action-btn primary-btn"
          >
            Rebuild
          </button>
          <button
            onclick={emptyFilteredDeck}
            class="action-btn neu-subtle neu-btn"
          >
            Empty
          </button>
        </div>
      </div>
    {:else}
      <!-- New Cards -->
      <div class="option-section">
        <h3 class="section-title">New Cards</h3>
        
         <div class="option-row">
           <label for="new-cards-per-day" class="option-label">Cards per day</label>
           <input
             id="new-cards-per-day"
             type="number"
             bind:value={opts.newCardsPerDay}
             min="0"
             max="9999"
             class="option-input neu-pressed"
           />
         </div>

         <div class="option-row">
           <label for="learning-steps-input" class="option-label">Learning steps (minutes)</label>
           <div class="steps-container">
             {#each opts.learningSteps as step, i}
               <span class="step-pill neu-subtle">
                 {step}
                 <button onclick={() => removeLearningStep(i)} class="step-remove" aria-label="Remove learning step {step}">
                   <svg class="remove-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                     <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                   </svg>
                 </button>
               </span>
             {/each}
           </div>
           <div class="step-input-row">
             <input
               id="learning-steps-input"
               type="number"
               bind:value={newLearningStep}
               placeholder="Add step (minutes)"
               class="step-input neu-pressed"
               onkeydown={(e) => e.key === "Enter" && addLearningStep()}
             />
             <button onclick={addLearningStep} class="add-step-btn neu-subtle neu-btn">Add</button>
           </div>
         </div>

         <div class="option-row-grid">
           <div class="option-row">
             <label for="graduating-interval" class="option-label">Graduating interval (days)</label>
             <input
               id="graduating-interval"
               type="number"
               bind:value={opts.graduatingInterval}
               min="0"
               class="option-input neu-pressed"
             />
           </div>
           <div class="option-row">
             <label for="easy-interval" class="option-label">Easy interval (days)</label>
             <input
               id="easy-interval"
               type="number"
               bind:value={opts.easyInterval}
               min="0"
               class="option-input neu-pressed"
             />
           </div>
         </div>
      </div>

      <!-- Reviews -->
      <div class="option-section">
        <h3 class="section-title">Reviews</h3>
        
         <div class="option-row">
           <label for="max-reviews-per-day" class="option-label">Max reviews per day</label>
           <input
             id="max-reviews-per-day"
             type="number"
             bind:value={opts.maxReviewsPerDay}
             min="0"
             class="option-input neu-pressed"
           />
         </div>

         <div class="option-row-grid">
           <div class="option-row">
             <label for="easy-bonus" class="option-label">Easy bonus</label>
             <input
               id="easy-bonus"
               type="number"
               bind:value={opts.easyBonus}
               min="0"
               step="0.01"
               class="option-input neu-pressed"
             />
           </div>
           <div class="option-row">
             <label for="interval-modifier" class="option-label">Interval modifier</label>
             <input
               id="interval-modifier"
               type="number"
               bind:value={opts.intervalModifier}
               min="0"
               step="0.01"
               class="option-input neu-pressed"
             />
           </div>
        </div>

        <div class="option-row">
          <label for="maximum-interval" class="option-label">Maximum interval (days)</label>
          <input
            id="maximum-interval"
            type="number"
            bind:value={opts.maximumInterval}
            min="0"
            class="option-input neu-pressed"
          />
        </div>
      </div>

      <!-- FSRS -->
      <div class="option-section">
        <h3 class="section-title">FSRS</h3>
        
        <div class="option-row toggle-row">
          <label for="fsrs-toggle" class="option-label">Enable FSRS</label>
          <button
            id="fsrs-toggle"
            onclick={() => opts.fsrsEnabled = !opts.fsrsEnabled}
            class="toggle-btn {opts.fsrsEnabled ? 'active' : ''}"
            aria-label={opts.fsrsEnabled ? "Disable FSRS" : "Enable FSRS"}
          >
            <span class="toggle-knob"></span>
          </button>
        </div>

        <div class="option-row">
          <label for="desired-retention" class="option-label">Desired retention: {Math.round(opts.desiredRetention * 100)}%</label>
          <input
            id="desired-retention"
            type="range"
            bind:value={opts.desiredRetention}
            min="0.70"
            max="0.97"
            step="0.01"
            class="range-slider"
          />
          <div class="range-labels">
            <span>70%</span>
            <span>97%</span>
          </div>
        </div>

        <div class="option-row">
          <label for="fsrs-weights" class="option-label">FSRS Weights</label>
          <div class="weights-display neu-pressed">
            <p class="weights-text">
              {opts.fsrsWeights.length > 0 ? opts.fsrsWeights.join(", ") : "Using default weights"}
            </p>
            {#if isOptimizing}
              <p class="weights-status optimizing">Analyzing review history...</p>
            {:else if optimizationResult}
              {#if !optimizationResult.success}
                <p class="weights-status warning">
                  Need at least 400 reviews for reliable FSRS optimization. You have {optimizationResult.review_count}.
                </p>
              {:else}
                <p class="weights-status success">
                  Optimized! Current retention: {Math.round(optimizationResult.current_retention * 100)}% → Predicted: {Math.round(optimizationResult.predicted_retention * 100)}%
                </p>
              {/if}
            {:else}
              <p class="weights-status">
                FSRS weights optimized from your review history.
              </p>
            {/if}
            <div class="weights-actions">
              <button
                onclick={handleOptimizeFsrs}
                disabled={isOptimizing}
                class="optimize-btn neu-subtle neu-btn"
              >
                {isOptimizing ? 'Optimizing...' : 'Optimize'}
              </button>
              {#if opts.fsrsWeights.length > 0}
                <button
                  onclick={resetFsrsWeights}
                  class="reset-btn neu-subtle neu-btn"
                >
                  Reset
                </button>
              {/if}
            </div>
          </div>
        </div>
      </div>

      <!-- Lapses -->
      <div class="option-section">
        <h3 class="section-title">Lapses</h3>
        
        <div class="option-row">
          <label for="relearning-steps" class="option-label">Relearning steps (minutes)</label>
          <div class="steps-container">
            {#each opts.lapseSteps as step, i}
              <span class="step-pill neu-subtle">
                {step}
                <button onclick={() => removeLapseStep(i)} class="step-remove" aria-label="Remove step {step}">
                  <svg class="remove-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </span>
            {/each}
          </div>
          <div class="step-input-row">
            <input
              type="number"
              bind:value={newLapseStep}
              placeholder="Add step (minutes)"
              class="step-input neu-pressed"
              onkeydown={(e) => e.key === "Enter" && addLapseStep()}
            />
            <button onclick={addLapseStep} class="add-step-btn neu-subtle neu-btn">Add</button>
          </div>
        </div>

        <div class="option-row-grid">
          <div class="option-row">
            <label for="lapse-minimum-interval" class="option-label">Minimum interval (days)</label>
            <input
              id="lapse-minimum-interval"
              type="number"
              bind:value={opts.lapseMinimumInterval}
              min="0"
              class="option-input neu-pressed"
            />
          </div>
          <div class="option-row">
            <label for="leech-threshold" class="option-label">Leech threshold (lapses)</label>
            <input
              id="leech-threshold"
              type="number"
              bind:value={opts.leechThreshold}
              min="0"
              class="option-input neu-pressed"
            />
          </div>
        </div>

        <div class="option-row">
          <label for="leech-action" class="option-label">Leech action</label>
          <NeuSelect
            id="leech-action"
            options={[
              { value: 'suspend', label: 'Suspend card' },
              { value: 'tag', label: 'Tag only' }
            ]}
            bind:value={leechAction}
            size="sm"
          />
        </div>
      </div>
    {/if}
  </div>

  <div class="dialog-footer">
    <button
      onclick={onClose}
      class="footer-btn cancel-btn neu-subtle neu-btn"
    >
      Cancel
    </button>
    <button
      onclick={saveOptions}
      class="footer-btn save-btn"
    >
      Save Options
    </button>
  </div>
</NeuDialog>

<style>
  .deck-options {
    display: flex;
    flex-direction: column;
    gap: 20px;
    max-height: 60vh;
    overflow-y: auto;
    padding-right: 8px;
  }

  .deck-name {
    font-family: var(--serif);
    font-size: 16px;
    color: var(--text-secondary);
    margin: 0;
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .skeleton-line {
    height: 48px;
    background: var(--bg-subtle);
    border-radius: var(--radius-sm);
    animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  .filtered-deck-section {
    background: color-mix(in srgb, var(--accent) 8%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent) 20%, transparent);
    border-radius: var(--radius-md);
    padding: 16px;
  }

  .section-title {
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 8px 0;
  }

  .section-description {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0 0 16px 0;
    line-height: 1.5;
  }

  .action-buttons {
    display: flex;
    gap: 12px;
  }

  .action-btn {
    flex: 1;
    padding: 10px 16px;
    font-family: var(--sans);
    font-size: 13px;
    font-weight: 500;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .primary-btn {
    background: var(--accent);
    color: white;
  }

  .primary-btn:hover {
    background: color-mix(in srgb, var(--accent) 90%, black);
  }

  .option-section {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding-bottom: 20px;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 30%, transparent);
  }

  .option-section:last-child {
    border-bottom: none;
    padding-bottom: 0;
  }

  .option-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .option-row-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .option-label {
    font-family: var(--sans);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .option-input {
    width: 100%;
    padding: 10px 12px;
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-primary);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    outline: none;
  }

  .toggle-row {
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
  }

  .toggle-btn {
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

  .toggle-btn.active {
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

  .toggle-btn.active .toggle-knob {
    left: 23px;
  }

  .range-slider {
    width: 100%;
    height: 6px;
    border-radius: 3px;
    background: var(--bg-deep);
    appearance: none;
    cursor: pointer;
  }

  .range-slider::-webkit-slider-thumb {
    appearance: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--accent);
    cursor: pointer;
    box-shadow: 0 2px 6px rgba(0,0,0,0.15);
  }

  .range-slider::-moz-range-thumb {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--accent);
    cursor: pointer;
    border: none;
    box-shadow: 0 2px 6px rgba(0,0,0,0.15);
  }

  .range-labels {
    display: flex;
    justify-content: space-between;
    font-family: var(--sans);
    font-size: 11px;
    color: var(--text-muted);
  }

  .steps-container {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 8px;
  }

  .step-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    font-family: var(--sans);
    font-size: 12px;
    color: var(--text-primary);
    border-radius: var(--radius-sm);
  }

  .step-remove {
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-muted);
    padding: 0;
    margin-left: 2px;
  }

  .step-remove:hover {
    color: var(--danger);
  }

  .remove-icon {
    width: 12px;
    height: 12px;
  }

  .step-input-row {
    display: flex;
    gap: 8px;
  }

  .step-input {
    flex: 1;
    padding: 8px 12px;
    font-family: var(--sans);
    font-size: 12px;
    color: var(--text-primary);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    outline: none;
  }

  .add-step-btn {
    padding: 8px 12px;
    font-family: var(--sans);
    font-size: 12px;
    font-weight: 500;
    color: var(--accent);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }

  .add-step-btn:hover {
    background: var(--accent-soft);
  }

  .weights-display {
    padding: 12px;
    border-radius: var(--radius-sm);
  }

  .weights-text {
    font-family: 'SF Mono', Monaco, Consolas, monospace;
    font-size: 11px;
    color: var(--text-secondary);
    word-break: break-all;
    margin: 0 0 8px 0;
  }

  .weights-status {
    font-family: var(--sans);
    font-size: 11px;
    color: var(--text-muted);
    margin: 0 0 12px 0;
  }

  .weights-status.optimizing {
    color: var(--accent);
  }

  .weights-status.warning {
    color: var(--warning);
  }

  .weights-status.success {
    color: var(--success);
  }

  .weights-actions {
    display: flex;
    gap: 8px;
  }

  .optimize-btn,
  .reset-btn {
    padding: 6px 12px;
    font-family: var(--sans);
    font-size: 11px;
    font-weight: 500;
    color: var(--accent);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }

  .optimize-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .reset-btn {
    color: var(--text-secondary);
  }

  .dialog-footer {
    display: flex;
    gap: 12px;
    margin-top: 24px;
    padding-top: 16px;
    border-top: 1px solid color-mix(in srgb, var(--border) 30%, transparent);
  }

  .footer-btn {
    flex: 1;
    padding: 12px 16px;
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 500;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .cancel-btn {
    color: var(--text-secondary);
  }

  .save-btn {
    background: var(--accent);
    color: white;
  }

  .save-btn:hover {
    background: color-mix(in srgb, var(--accent) 90%, black);
  }
</style>
