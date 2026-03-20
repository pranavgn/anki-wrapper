<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { fly, fade } from "svelte/transition";
  import { addToast } from "./toast";

  interface Props {
    deckId: number;
    deckName: string;
    onClose: () => void;
  }

  let { deckId, deckName, onClose }: Props = $props();

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

  async function saveOptions() {
    try {
      await invoke("save_deck_options", { deckId, opts });
      addToast("Options saved", "success");
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
                min="0.7"
                max="0.99"
                step="0.01"
                class="w-full"
              />
            </div>
            <div>
              <label class="block text-sm text-text-secondary mb-1">FSRS Weights</label>
              <div class="p-3 bg-bg-subtle rounded-lg">
                <p class="text-xs font-mono text-text-secondary break-all">
                  {opts.fsrs_weights.length > 0 ? opts.fsrs_weights.join(", ") : "Using default weights"}
                </p>
                {#if opts.fsrs_weights.length > 0}
                  <button
                    onclick={resetFsrsWeights}
                    class="mt-2 text-xs text-accent hover:underline"
                  >
                    Reset to Default
                  </button>
                {/if}
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
