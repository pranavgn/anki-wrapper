<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { fly, fade } from "svelte/transition";
  import { addToast } from "./toast";

  interface Props {
    deckId: number;
    deckName: string;
    isFiltered?: boolean;
    onClose: () => void;
  }

  let { deckId, deckName, isFiltered = false, onClose }: Props = $props();

  interface DeckOptions {
    config_id: number;
    name: string;
    new_cards_per_day: number;
    learning_steps: number[];
    graduating_interval: number;
    easy_interval: number;
    max_reviews_per_day: number;
    easy_bonus: number;
    interval_modifier: number;
    maximum_interval: number;
    fsrs_enabled: boolean;
    fsrs_weights: number[];
    desired_retention: number;
    lapse_steps: number[];
    lapse_minimum_interval: number;
    leech_threshold: number;
  }

  let isLoading = $state(true);
  let leechAction = $state<"suspend" | "tag">("suspend");
  let opts: DeckOptions = $state({
    config_id: 0,
    name: "",
    new_cards_per_day: 20,
    learning_steps: [1, 10],
    graduating_interval: 1,
    easy_interval: 4,
    max_reviews_per_day: 200,
    easy_bonus: 1.3,
    interval_modifier: 1.0,
    maximum_interval: 36500,
    fsrs_enabled: true,
    fsrs_weights: [],
    desired_retention: 0.9,
    lapse_steps: [10],
    lapse_minimum_interval: 1,
    leech_threshold: 8,
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
        desiredRetention: opts.desired_retention,
      });
      
      optimizationResult = result;
      
      if (result.success && result.weights.length > 0) {
        // Apply the optimized weights
        opts.fsrs_weights = result.weights;
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
      opts.learning_steps = [...opts.learning_steps, step].sort((a, b) => a - b);
      newLearningStep = "";
    }
  }

  function removeLearningStep(index: number) {
    opts.learning_steps = opts.learning_steps.filter((_, i) => i !== index);
  }

  function addLapseStep() {
    const step = parseFloat(newLapseStep);
    if (!isNaN(step) && step > 0) {
      opts.lapse_steps = [...opts.lapse_steps, step].sort((a, b) => a - b);
      newLapseStep = "";
    }
  }

  function removeLapseStep(index: number) {
    opts.lapse_steps = opts.lapse_steps.filter((_, i) => i !== index);
  }

  function resetFsrsWeights() {
    opts.fsrs_weights = [];
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
    
    if (opts.new_cards_per_day < 0) {
      validationErrors.push("New cards per day must be >= 0");
    }
    
    if (opts.learning_steps.length === 0 || opts.learning_steps.some(s => s <= 0)) {
      validationErrors.push("Learning steps must contain positive numbers");
    }
    
    if (opts.graduating_interval < 0) {
      validationErrors.push("Graduating interval must be >= 0");
    }
    
    if (opts.desired_retention < 0.70 || opts.desired_retention > 0.97) {
      validationErrors.push("Desired retention must be between 70% and 97%");
    }
    
    if (opts.max_reviews_per_day < 0) {
      validationErrors.push("Max reviews per day must be >= 0");
    }
    
    if (opts.lapse_steps.some(s => s <= 0)) {
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

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div 
  class="fixed inset-0 bg-black/15 z-50 flex justify-end"
  onclick={handleBackdropClick}
  transition:fade={{ duration: 150 }}
>
  <div 
    class="w-[420px] bg-white h-full border-l border-border flex flex-col"
    transition:fly={{ x: 420, duration: 240 }}
  >
    <!-- Header -->
    <div class="px-6 py-4 border-b border-border flex items-center justify-between">
      <div>
        <h2 class="text-lg font-semibold text-text-primary">Deck Options</h2>
        <p class="text-sm text-text-secondary">{deckName}</p>
      </div>
      <button
        onclick={onClose}
        class="p-2 hover:bg-bg-subtle rounded-lg transition-colors"
      >
        <svg class="h-5 w-5 text-text-secondary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto px-6 py-4">
      {#if isLoading}
        <div class="space-y-4">
          {#each Array(5) as _}
            <div class="h-12 bg-bg-subtle rounded-lg animate-pulse"></div>
          {/each}
        </div>
      {:else if isFiltered}
        <!-- Filtered Deck Options -->
        <div class="space-y-4">
          <div class="bg-accent-soft/20 border border-accent/30 rounded-xl p-4">
            <h3 class="font-medium text-text-primary mb-2">Filtered Deck Actions</h3>
            <p class="text-sm text-text-secondary mb-4">
              This is a filtered deck. Use these actions to manage the cards in this deck.
            </p>
            <div class="flex gap-3">
              <button
                onclick={rebuildFilteredDeck}
                class="flex-1 px-4 py-2 bg-accent text-white rounded-lg hover:bg-accent/90 transition-colors text-sm font-medium"
              >
                Rebuild
              </button>
              <button
                onclick={emptyFilteredDeck}
                class="flex-1 px-4 py-2 bg-bg-subtle text-text-primary rounded-lg hover:bg-bg-subtle/80 transition-colors text-sm font-medium"
              >
                Empty
              </button>
            </div>
          </div>
        </div>
      {:else}
        <!-- New Cards -->
        <details class="group mb-4" open>
          <summary class="flex items-center justify-between cursor-pointer list-none py-2 font-medium text-text-primary">
            <span>New Cards</span>
            <svg class="h-4 w-4 text-text-secondary transition-transform group-open:rotate-180" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
            </svg>
          </summary>
          <div class="space-y-4 pt-2">
            <div>
              <label class="block text-sm text-text-secondary mb-1">Cards per day</label>
              <input
                type="number"
                bind:value={opts.new_cards_per_day}
                min="0"
                max="9999"
                class="w-full px-3 py-2 bg-bg-subtle border border-border rounded-lg text-text-primary"
              />
            </div>
            <div>
              <label class="block text-sm text-text-secondary mb-1">Learning steps (minutes)</label>
              <div class="flex flex-wrap gap-2 mb-2">
                {#each opts.learning_steps as step, i}
                  <span class="inline-flex items-center gap-1 px-2 py-1 bg-bg-subtle rounded text-sm">
                    {step}
                    <button onclick={() => removeLearningStep(i)} class="text-text-secondary hover:text-danger">
                      <svg class="h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                      </svg>
                    </button>
                  </span>
                {/each}
              </div>
              <div class="flex gap-2">
                <input
                  type="number"
                  bind:value={newLearningStep}
                  placeholder="Add step (minutes)"
                  class="flex-1 px-3 py-2 bg-bg-subtle border border-border rounded-lg text-text-primary text-sm"
                  onkeydown={(e) => e.key === "Enter" && addLearningStep()}
                />
                <button onclick={addLearningStep} class="px-3 py-2 bg-accent text-white rounded-lg text-sm">Add</button>
              </div>
            </div>
            <div class="grid grid-cols-2 gap-4">
              <div>
                <label class="block text-sm text-text-secondary mb-1">Graduating interval (days)</label>
                <input
                  type="number"
                  bind:value={opts.graduating_interval}
                  min="0"
                  class="w-full px-3 py-2 bg-bg-subtle border border-border rounded-lg text-text-primary"
                />
              </div>
              <div>
                <label class="block text-sm text-text-secondary mb-1">Easy interval (days)</label>
                <input
                  type="number"
                  bind:value={opts.easy_interval}
                  min="0"
                  class="w-full px-3 py-2 bg-bg-subtle border border-border rounded-lg text-text-primary"
                />
              </div>
            </div>
          </div>
        </details>

        <!-- Reviews -->
        <details class="group mb-4" open>
          <summary class="flex items-center justify-between cursor-pointer list-none py-2 font-medium text-text-primary">
            <span>Reviews</span>
            <svg class="h-4 w-4 text-text-secondary transition-transform group-open:rotate-180" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
            </svg>
          </summary>
          <div class="space-y-4 pt-2">
            <div>
              <label class="block text-sm text-text-secondary mb-1">Max reviews per day</label>
              <input
                type="number"
                bind:value={opts.max_reviews_per_day}
                min="0"
                class="w-full px-3 py-2 bg-bg-subtle border border-border rounded-lg text-text-primary"
              />
            </div>
            <div class="grid grid-cols-2 gap-4">
              <div>
                <label class="block text-sm text-text-secondary mb-1">Easy bonus</label>
                <input
                  type="number"
                  bind:value={opts.easy_bonus}
                  min="0"
                  step="0.01"
                  class="w-full px-3 py-2 bg-bg-subtle border border-border rounded-lg text-text-primary"
                />
              </div>
              <div>
                <label class="block text-sm text-text-secondary mb-1">Interval modifier</label>
                <input
                  type="number"
                  bind:value={opts.interval_modifier}
                  min="0"
                  step="0.01"
                  class="w-full px-3 py-2 bg-bg-subtle border border-border rounded-lg text-text-primary"
                />
              </div>
            </div>
            <div>
              <label class="block text-sm text-text-secondary mb-1">Maximum interval (days)</label>
              <input
                type="number"
                bind:value={opts.maximum_interval}
                min="0"
                class="w-full px-3 py-2 bg-bg-subtle border border-border rounded-lg text-text-primary"
              />
            </div>
          </div>
        </details>

        <!-- FSRS -->
        <details class="group mb-4" open>
          <summary class="flex items-center justify-between cursor-pointer list-none py-2 font-medium text-text-primary">
            <span>FSRS</span>
            <svg class="h-4 w-4 text-text-secondary transition-transform group-open:rotate-180" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
            </svg>
          </summary>
          <div class="space-y-4 pt-2">
            <div class="flex items-center justify-between">
              <label class="text-sm text-text-secondary">Enable FSRS</label>
              <button
                onclick={() => opts.fsrs_enabled = !opts.fsrs_enabled}
                class="relative w-11 h-6 rounded-full transition-colors {opts.fsrs_enabled ? 'bg-accent' : 'bg-bg-subtle'}"
              >
                <span class="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full transition-transform {opts.fsrs_enabled ? 'translate-x-5' : ''}"></span>
              </button>
            </div>
            <div>
              <label class="block text-sm text-text-secondary mb-1">Desired retention: {Math.round(opts.desired_retention * 100)}%</label>
              <input
                type="range"
                bind:value={opts.desired_retention}
                min="0.70"
                max="0.97"
                step="0.01"
                class="w-full"
              />
              <div class="flex justify-between text-xs text-text-secondary mt-1">
                <span>70%</span>
                <span>97%</span>
              </div>
            </div>
            <div>
              <label class="block text-sm text-text-secondary mb-1">FSRS Weights</label>
              <div class="p-3 bg-bg-subtle rounded-lg">
                <p class="text-xs font-mono text-text-secondary break-all">
                  {opts.fsrs_weights.length > 0 ? opts.fsrs_weights.join(", ") : "Using default weights"}
                </p>
                {#if isOptimizing}
                  <p class="text-xs text-accent mt-2">Analyzing review history...</p>
                {:else if optimizationResult}
                  {#if !optimizationResult.success}
                    <p class="text-xs text-warning mt-2">
                      Need at least 400 reviews for reliable FSRS optimization. You have {optimizationResult.review_count}.
                    </p>
                  {:else}
                    <p class="text-xs text-success mt-2">
                      Optimized! Current retention: {Math.round(optimizationResult.current_retention * 100)}% → Predicted: {Math.round(optimizationResult.predicted_retention * 100)}%
                    </p>
                  {/if}
                {:else}
                  <p class="text-xs text-text-secondary mt-2">
                    FSRS weights optimized from your review history.
                  </p>
                {/if}
                <div class="flex gap-2 mt-3">
                  <button
                    onclick={handleOptimizeFsrs}
                    disabled={isOptimizing}
                    class="px-3 py-1.5 bg-accent text-white rounded-lg text-xs hover:bg-accent/90 transition-colors disabled:opacity-50"
                  >
                    {isOptimizing ? 'Optimizing...' : 'Optimize'}
                  </button>
                  {#if opts.fsrs_weights.length > 0}
                    <button
                      onclick={resetFsrsWeights}
                      class="px-3 py-1.5 bg-bg-card border border-border rounded-lg text-xs hover:bg-bg-subtle transition-colors"
                    >
                      Reset
                    </button>
                  {/if}
                </div>
              </div>
            </div>
          </div>
        </details>

        <!-- Lapses -->
        <details class="group mb-4">
          <summary class="flex items-center justify-between cursor-pointer list-none py-2 font-medium text-text-primary">
            <span>Lapses</span>
            <svg class="h-4 w-4 text-text-secondary transition-transform group-open:rotate-180" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
            </svg>
          </summary>
          <div class="space-y-4 pt-2">
            <div>
              <label class="block text-sm text-text-secondary mb-1">Relearning steps (minutes)</label>
              <div class="flex flex-wrap gap-2 mb-2">
                {#each opts.lapse_steps as step, i}
                  <span class="inline-flex items-center gap-1 px-2 py-1 bg-bg-subtle rounded text-sm">
                    {step}
                    <button onclick={() => removeLapseStep(i)} class="text-text-secondary hover:text-danger">
                      <svg class="h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                      </svg>
                    </button>
                  </span>
                {/each}
              </div>
              <div class="flex gap-2">
                <input
                  type="number"
                  bind:value={newLapseStep}
                  placeholder="Add step (minutes)"
                  class="flex-1 px-3 py-2 bg-bg-subtle border border-border rounded-lg text-text-primary text-sm"
                  onkeydown={(e) => e.key === "Enter" && addLapseStep()}
                />
                <button onclick={addLapseStep} class="px-3 py-2 bg-accent text-white rounded-lg text-sm">Add</button>
              </div>
            </div>
            <div class="grid grid-cols-2 gap-4">
              <div>
                <label class="block text-sm text-text-secondary mb-1">Minimum interval (days)</label>
                <input
                  type="number"
                  bind:value={opts.lapse_minimum_interval}
                  min="0"
                  class="w-full px-3 py-2 bg-bg-subtle border border-border rounded-lg text-text-primary"
                />
              </div>
              <div>
                <label class="block text-sm text-text-secondary mb-1">Leech threshold (lapses)</label>
                <input
                  type="number"
                  bind:value={opts.leech_threshold}
                  min="0"
                  class="w-full px-3 py-2 bg-bg-subtle border border-border rounded-lg text-text-primary"
                />
              </div>
            </div>
            <div>
              <label class="block text-sm text-text-secondary mb-1">Leech action</label>
              <select
                bind:value={leechAction}
                class="w-full px-3 py-2 bg-bg-subtle border border-border rounded-lg text-text-primary"
              >
                <option value="suspend">Suspend card</option>
                <option value="tag">Tag only</option>
              </select>
            </div>
          </div>
        </details>
      {/if}
    </div>

    <!-- Footer -->
    <div class="sticky bottom-0 px-6 py-4 bg-white border-t border-border flex gap-3">
      <button
        onclick={onClose}
        class="flex-1 px-4 py-2 border border-border rounded-lg text-text-primary hover:bg-bg-subtle transition-colors"
      >
        Cancel
      </button>
      <button
        onclick={saveOptions}
        class="flex-1 px-4 py-2 bg-accent text-white rounded-lg hover:bg-accent/90 transition-colors"
      >
        Save Options
      </button>
    </div>
  </div>
</div>
