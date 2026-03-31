<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { scheduleStore } from "../scheduleStore.svelte";
  import {
    SESSION_TYPE_THEMES,
    DURATION_OPTIONS,
    defaultSchedulerData,
    type SessionType,
    type SchedulerPopupData,
  } from "../types/scheduler";

  interface Props {
    open: boolean;
    initialDate?: string;
    initialDeckId?: number | null;
    initialDeckName?: string | null;
    onclose: () => void;
  }

  let { open = $bindable(), initialDate, initialDeckId = null, initialDeckName = null, onclose }: Props = $props();

  let formData = $state<SchedulerPopupData>(defaultSchedulerData());
  let isSaving = $state(false);
  let popupEl: HTMLDivElement | undefined = $state();

  // Reset form when opened
  let prevOpen = false;
  $effect(() => {
    if (open && !prevOpen) {
      formData = defaultSchedulerData(initialDate);
      formData.deckId = initialDeckId;
      formData.deckName = initialDeckName;
    }
    prevOpen = open;
  });

  // Click-outside to close
  let justOpened = false;
  function handleClickOutside(e: MouseEvent) {
    if (!open || !popupEl || justOpened) return;
    if (!popupEl.contains(e.target as Node)) {
      onclose();
    }
  }

  onMount(() => {
    // Use setTimeout so the opening click doesn't immediately close
    setTimeout(() => document.addEventListener('click', handleClickOutside), 0);
    return () => document.removeEventListener('click', handleClickOutside);
  });

  // Mark as just opened to prevent immediate close
  $effect(() => {
    if (open) {
      justOpened = true;
      setTimeout(() => { justOpened = false; }, 100);
    }
  });

  function setType(t: SessionType) {
    formData.sessionType = t;
  }

  async function handleSave() {
    isSaving = true;
    // Resolve deck name from store if we have an ID but no name
    if (formData.deckId && !formData.deckName) {
      const deck = scheduleStore.decks.find(d => d.id === formData.deckId);
      if (deck) formData.deckName = deck.name;
    }
    await scheduleStore.addSession(formData);
    isSaving = false;
    onclose();
  }
</script>

{#if open}
  <div
    class="floating-scheduler"
    bind:this={popupEl}
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.key === 'Escape' && onclose()}
    role="dialog"
    aria-modal="true"
    aria-label="Schedule session"
  >
    <div class="fs-header">
      <span class="fs-title">Schedule session</span>
      <button class="fs-close neu-btn" onclick={onclose} aria-label="Close scheduler">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M18 6L6 18M6 6l12 12"/></svg>
      </button>
    </div>

    <div class="fs-fields">
      <div class="fs-field">
        <label class="fs-label" for="fs-date">Date</label>
        <input id="fs-date" type="date" class="fs-input" bind:value={formData.date} />
      </div>

      <div class="fs-row">
        <div class="fs-field">
          <label class="fs-label" for="fs-time">Time</label>
          <input id="fs-time" type="time" class="fs-input" bind:value={formData.time} />
        </div>
        <div class="fs-field">
          <label class="fs-label" for="fs-duration">Duration</label>
          <select id="fs-duration" class="fs-input" bind:value={formData.duration}>
            {#each DURATION_OPTIONS as opt}
              <option value={opt.value}>{opt.label}</option>
            {/each}
          </select>
        </div>
      </div>

      <div class="fs-field">
        <label class="fs-label" for="fs-deck">Deck</label>
        <select id="fs-deck" class="fs-input" bind:value={formData.deckId} onchange={(e) => {
          const deck = scheduleStore.decks.find(d => d.id === Number((e.target as HTMLSelectElement).value));
          formData.deckName = deck?.name || null;
        }}>
          <option value={null}>All decks</option>
          {#each scheduleStore.decks as deck}
            <option value={deck.id}>{deck.name}</option>
          {/each}
        </select>
      </div>

      <div class="fs-field">
        <span class="fs-label" id="fs-type-label">Type</span>
        <div class="fs-types" role="group" aria-labelledby="fs-type-label">
          {#each Object.entries(SESSION_TYPE_THEMES) as [key, theme]}
            <button
              class="fs-type-btn"
              style="
                background: {formData.sessionType === key ? theme.bg : 'var(--bg-subtle)'};
                color: {formData.sessionType === key ? theme.text : 'var(--text-secondary)'};
              "
              onclick={() => setType(key as SessionType)}
            >
              {theme.label}
            </button>
          {/each}
        </div>
      </div>
    </div>

    <div class="fs-actions">
      <button class="fs-cancel neu-subtle neu-btn" onclick={onclose}>Cancel</button>
      <button class="fs-save neu-btn" onclick={handleSave} disabled={isSaving}>
        {isSaving ? 'Saving...' : 'Schedule'}
      </button>
    </div>
  </div>
{/if}

<style>
  .floating-scheduler {
    position: absolute;
    right: 0;
    top: 100%;
    margin-top: 8px;
    z-index: 50;
    width: 300px;
    background: var(--bg-card);
    box-shadow: var(--neu-up), 0 12px 40px rgba(0,0,0,0.12);
    border-radius: var(--radius-md);
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .fs-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .fs-title {
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .fs-close {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--bg-subtle);
    border: none;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    cursor: pointer;
    box-shadow: var(--neu-subtle);
  }
  .fs-close:hover { background: var(--bg-deep); }

  .fs-fields {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .fs-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .fs-field {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .fs-label {
    font-family: var(--sans);
    font-size: 10px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .fs-input {
    padding: 7px 10px;
    border-radius: var(--radius-sm);
    border: none;
    background: var(--bg-deep);
    box-shadow: var(--neu-down);
    color: var(--text-primary);
    font-size: 12px;
    font-family: var(--sans);
    width: 100%;
    box-sizing: border-box;
  }
  .fs-input:focus {
    outline: 2px solid var(--accent);
    outline-offset: 0;
  }

  .fs-types {
    display: flex;
    gap: 6px;
  }

  .fs-type-btn {
    flex: 1;
    padding: 5px 8px;
    font-size: 11px;
    font-weight: 500;
    font-family: var(--sans);
    border-radius: 20px;
    border: none;
    cursor: pointer;
    box-shadow: var(--neu-subtle);
    transition: all 0.12s;
  }

  .fs-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  .fs-cancel {
    padding: 7px 14px;
    font-size: 12px;
    font-weight: 500;
    font-family: var(--sans);
    color: var(--text-secondary);
  }

  .fs-save {
    padding: 7px 18px;
    font-size: 12px;
    font-weight: 500;
    font-family: var(--sans);
    background: var(--accent);
    color: #fff;
    border: none;
    border-radius: var(--radius-sm);
    box-shadow: 3px 3px 8px rgba(196,113,79,0.2), -2px -2px 6px rgba(255,255,255,0.1);
  }
  .fs-save:hover { opacity: 0.92; }
  .fs-save:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
