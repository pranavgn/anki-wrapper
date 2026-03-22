<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { fly } from "svelte/transition";
  import { addToast } from "./toast";

  // Props
  let {
    cardId = null,
    noteId = null,
    onClose = () => {},
    onFlagChange = (cardId: number, flag: number) => {},
    onEdit = (card: any) => {}
  }: {
    cardId: number | null;
    noteId?: number | null;
    onClose?: () => void;
    onFlagChange?: (cardId: number, flag: number) => void;
    onEdit?: (card: any) => void;
  } = $props();

  // State
  let detail: any = $state(null);
  let loading = $state(false);
  let showDeleteConfirm = $state(false);

  // Flag colors
  const FLAG_COLORS = [
    { value: 0, color: '#6b7280', label: 'None' },
    { value: 1, color: '#ef4444', label: 'Red' },
    { value: 2, color: '#f97316', label: 'Orange' },
    { value: 3, color: '#eab308', label: 'Yellow' },
    { value: 4, color: '#22c55e', label: 'Green' },
    { value: 5, color: '#3b82f6', label: 'Blue' },
  ];

  // Load card detail when cardId changes
  $effect(() => {
    if (cardId) {
      loadDetail(cardId);
    } else {
      detail = null;
    }
  });

  async function loadDetail(cid: number) {
    loading = true;
    try {
      detail = await invoke('get_card_detail', { cardId: cid });
    } catch (e) {
      addToast('Failed to load card details', 'error');
    } finally {
      loading = false;
    }
  }

  async function handleFlagChange(flag: number) {
    if (!detail) return;
    try {
      await invoke('set_card_flag', { cardId: detail.card_id, flag });
      detail = { ...detail, flag };
      onFlagChange(detail.card_id, flag);
    } catch (e) {
      addToast('Failed to set flag', 'error');
    }
  }

  async function handleSuspend() {
    if (!detail) return;
    try {
      await invoke('suspend_cards', { cardIds: [detail.card_id] });
      addToast('Card suspended', 'success');
      onClose();
    } catch (e) {
      addToast('Failed to suspend card', 'error');
    }
  }

  async function handleDelete() {
    if (!detail) return;
    try {
      await invoke('delete_notes', { noteIds: [detail.note_id] });
      addToast('Note deleted', 'success');
      onClose();
    } catch (e) {
      addToast('Failed to delete note', 'error');
    }
  }

  function getQueueLabel(queue: number): string {
    switch (queue) {
      case -2: return 'New';
      case 1:
      case 3: return 'Learning';
      case 2: return 'Review';
      case -1: return 'Suspended';
      default: if (queue < -1) return 'Buried';
    }
    return 'Unknown';
  }

  function formatEase(ease: number): string {
    return (ease / 10).toFixed(0) + '%';
  }
</script>

{#if cardId}
  <div
    class="h-full flex flex-col bg-bg-card"
    transition:fly={{ x: 20, duration: 240 }}
  >
    <!-- Header -->
    <div class="flex items-center justify-between p-4 border-b border-border/30">
      <h3 class="font-medium">Card Details</h3>
      <button 
        onclick={onClose} 
        class="p-1 text-text-secondary hover:text-text-primary"
        aria-label="Close panel"
      >
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>

    {#if loading}
      <!-- Loading skeleton -->
      <div class="flex-1 p-4 space-y-4">
        <div class="h-48 bg-bg-subtle rounded-xl animate-pulse"></div>
        <div class="space-y-2">
          {#each Array(6) as _}
            <div class="h-4 bg-bg-subtle rounded animate-pulse"></div>
          {/each}
        </div>
      </div>
    {:else if detail}
      <div class="flex-1 overflow-y-auto p-4 space-y-4">
        <!-- Card Preview -->
        <div class="neu-raised rounded-xl overflow-hidden">
          <div class="p-4 border-b border-border/30">
            <div class="font-serif text-sm">{@html detail.front_html}</div>
          </div>
          <div class="p-4">
            <div class="font-serif text-sm">{@html detail.back_html}</div>
          </div>
        </div>

        <!-- Metadata Grid -->
        <div class="bg-bg-subtle rounded-xl p-3 space-y-2">
          <div class="grid grid-cols-2 gap-2 text-xs">
            <div class="text-text-secondary">Deck</div>
            <div class="text-sm text-text-primary">{detail.deck_name}</div>
          </div>
          <div class="grid grid-cols-2 gap-2 text-xs">
            <div class="text-text-secondary">Note type</div>
            <div class="text-sm text-text-primary">{detail.notetype_name}</div>
          </div>
          <div class="grid grid-cols-2 gap-2 text-xs">
            <div class="text-text-secondary">Interval</div>
            <div class="text-sm text-text-primary">{detail.interval} days</div>
          </div>
          <div class="grid grid-cols-2 gap-2 text-xs">
            <div class="text-text-secondary">Ease</div>
            <div class="text-sm text-text-primary">{formatEase(detail.ease)}</div>
          </div>
          <div class="grid grid-cols-2 gap-2 text-xs">
            <div class="text-text-secondary">Lapses</div>
            <div class="text-sm text-text-primary">{detail.lapses}</div>
          </div>
          <div class="grid grid-cols-2 gap-2 text-xs">
            <div class="text-text-secondary">Due</div>
            <div class="text-sm text-text-primary">{detail.due_str}</div>
          </div>
          <div class="grid grid-cols-2 gap-2 text-xs">
            <div class="text-text-secondary">Queue</div>
            <div class="text-sm text-text-primary">{getQueueLabel(detail.queue)}</div>
          </div>
          
          <!-- Tags -->
          <div class="pt-2 border-t border-border/30">
            <div class="text-xs text-text-secondary mb-2">Tags</div>
            <div class="flex flex-wrap gap-1">
              {#if detail.tags && detail.tags.length > 0}
                {#each detail.tags as tag}
                  <span class="px-2 py-0.5 bg-bg-card text-text-secondary text-xs rounded-full">
                    {tag}
                  </span>
                {/each}
              {:else}
                <span class="text-xs text-text-secondary italic">No tags</span>
              {/if}
            </div>
          </div>
        </div>

        <!-- Flag Picker -->
        <div>
          <div class="text-xs text-text-secondary mb-2">Flag</div>
          <div class="flex gap-1">
            {#each FLAG_COLORS as flag}
              <button
                onclick={() => handleFlagChange(flag.value)}
                class="w-8 h-8 rounded-lg flex items-center justify-center transition-all {detail.flag === flag.value ? 'ring-2 ring-offset-2 ring-gray-400' : 'hover:scale-110'}"
                style="background-color: {flag.color}20; border: 2px solid {flag.color}"
                title={flag.label}
              >
                {#if detail.flag === flag.value}
                  <svg class="h-4 w-4" style="color: {flag.color}" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd" />
                  </svg>
                {/if}
              </button>
            {/each}
          </div>
        </div>

        <!-- Review History -->
        <div>
          <div class="text-sm font-semibold mb-2">Review History</div>
          {#if detail.review_history && detail.review_history.length > 0}
            <div class="space-y-2 max-h-48 overflow-y-auto">
              {#each detail.review_history.slice(0, 10) as entry}
                <div class="flex items-center gap-2 text-xs">
                  <span class="text-text-secondary w-16">Today</span>
                  <span class="px-2 py-0.5 rounded text-white {entry.rating === 1 ? 'bg-red-500' : entry.rating === 2 ? 'bg-amber-500' : entry.rating === 3 ? 'bg-blue-500' : 'bg-green-500'}">
                    {entry.rating === 1 ? 'Again' : entry.rating === 2 ? 'Hard' : entry.rating === 3 ? 'Good' : 'Easy'}
                  </span>
                  <span class="text-text-secondary">+{entry.interval_days}d</span>
                  <span class="text-text-secondary ml-auto">{entry.time_taken_secs}s</span>
                </div>
              {/each}
            </div>
          {:else}
            <p class="text-xs text-text-secondary italic">No reviews yet — this is a new card.</p>
          {/if}
        </div>

        <!-- Action Buttons -->
        <div class="flex gap-2 pt-2">
          <button
            onclick={() => onEdit(detail)}
            class="flex-1 flex items-center justify-center gap-2 px-3 py-2 neu-subtle neu-btn rounded-xl text-sm hover:bg-bg-subtle transition-colors"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
            </svg>
            Edit
          </button>
          
          <button
            onclick={handleSuspend}
            class="flex-1 flex items-center justify-center gap-2 px-3 py-2 neu-subtle neu-btn rounded-xl text-sm hover:bg-bg-subtle transition-colors"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 9v6m4-6v6m7-3a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            Suspend
          </button>
          
          {#if showDeleteConfirm}
            <div class="flex-1 flex gap-1">
              <button
                onclick={() => showDeleteConfirm = false}
                class="flex-1 px-3 py-2 neu-subtle neu-btn rounded-xl text-sm hover:bg-bg-subtle transition-colors"
              >
                Cancel
              </button>
              <button
                onclick={handleDelete}
                class="flex-1 px-3 py-2 border border-red-500 text-red-500 rounded-xl text-sm hover:bg-red-50 transition-colors"
              >
                Delete
              </button>
            </div>
          {:else}
            <button
              onclick={() => showDeleteConfirm = true}
              class="flex-1 flex items-center justify-center gap-2 px-3 py-2 neu-subtle neu-btn rounded-xl text-sm text-danger hover:bg-red-50 transition-colors"
            >
              <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
              </svg>
              Delete
            </button>
          {/if}
        </div>
      </div>
    {/if}
  </div>
{/if}
