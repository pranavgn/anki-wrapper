<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { addToast } from "./toast";
  import NeuDialog from "./ui/NeuDialog.svelte";

  interface Props {
    deckId: number;
    deckName: string;
    isFiltered?: boolean;
    isOpen: boolean;
    onClose: () => void;
    onRenamed?: (newName: string) => void;
    onDeleted?: () => void;
    onOpenOptions?: () => void;
  }

  let {
    deckId,
    deckName,
    isFiltered = false,
    isOpen,
    onClose,
    onRenamed = () => {},
    onDeleted = () => {},
    onOpenOptions = () => {},
  }: Props = $props();

  let newName = $state(deckName);
  let showDeleteConfirm = $state(false);
  let deleteCards = $state(false);
  let isSaving = $state(false);
  let isDeleting = $state(false);

  // Sync name field when deckName prop changes
  $effect(() => {
    newName = deckName;
  });

  async function handleRename() {
    const trimmed = newName.trim();
    if (!trimmed || trimmed === deckName) {
      onClose();
      return;
    }
    isSaving = true;
    try {
      await invoke("rename_deck", { deckId, newName: trimmed });
      addToast(`Deck renamed to "${trimmed}"`, "success");
      onRenamed(trimmed);
      onClose();
    } catch (e) {
      addToast(e instanceof Error ? e.message : "Failed to rename deck", "error");
    } finally {
      isSaving = false;
    }
  }

  async function handleDelete() {
    isDeleting = true;
    try {
      await invoke("delete_deck", { deckId, deleteCards });
      addToast("Deck deleted", "success");
      showDeleteConfirm = false;
      onDeleted();
      onClose();
    } catch (e) {
      addToast(e instanceof Error ? e.message : "Failed to delete deck", "error");
    } finally {
      isDeleting = false;
    }
  }

  function openOptions() {
    onClose();
    onOpenOptions();
  }
</script>

<NeuDialog {isOpen} onClose={onClose} title="Deck Settings">
  <div style="display: flex; flex-direction: column; gap: 20px; padding: 4px 0;">

    <!-- Rename -->
    <div>
      <label style="font-family: var(--sans); font-size: 12px; font-weight: 600; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.05em; display: block; margin-bottom: 8px;">
        Deck Name
      </label>
      <input
        type="text"
        bind:value={newName}
        onkeydown={(e) => e.key === 'Enter' && handleRename()}
        class="neu-pressed"
        style="
          width: 100%;
          padding: 10px 14px;
          border-radius: var(--radius-sm);
          border: none;
          background: var(--bg-deep);
          color: var(--text-primary);
          font-family: var(--sans);
          font-size: 14px;
          outline: none;
          box-sizing: border-box;
        "
      />
      <button
        onclick={handleRename}
        disabled={isSaving}
        class="neu-raised neu-btn"
        style="
          margin-top: 10px;
          width: 100%;
          padding: 10px;
          border-radius: var(--radius-sm);
          font-family: var(--sans);
          font-size: 13px;
          font-weight: 600;
          color: var(--text-primary);
          cursor: pointer;
        "
      >
        {isSaving ? "Saving…" : "Rename"}
      </button>
    </div>

    <!-- Deck Options -->
    {#if !isFiltered}
      <div style="border-top: 1px solid var(--border); padding-top: 16px;">
        <button
          onclick={openOptions}
          class="neu-subtle neu-btn"
          style="
            width: 100%;
            padding: 10px 14px;
            border-radius: var(--radius-sm);
            font-family: var(--sans);
            font-size: 13px;
            color: var(--text-secondary);
            cursor: pointer;
            display: flex;
            align-items: center;
            gap: 8px;
          "
        >
          <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 7a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4" />
          </svg>
          Algorithm Options (FSRS / SM-2)
        </button>
      </div>
    {/if}

    <!-- Delete -->
    <div style="border-top: 1px solid var(--border); padding-top: 16px;">
      {#if showDeleteConfirm}
        <p style="font-family: var(--sans); font-size: 13px; color: var(--text-secondary); margin-bottom: 12px;">
          Are you sure you want to delete <strong>{deckName}</strong>?
        </p>
        <label style="display: flex; align-items: center; gap: 8px; font-family: var(--sans); font-size: 13px; color: var(--text-secondary); margin-bottom: 14px; cursor: pointer;">
          <input type="checkbox" bind:checked={deleteCards} />
          Also delete all cards in this deck
        </label>
        <div style="display: flex; gap: 8px;">
          <button
            onclick={() => showDeleteConfirm = false}
            class="neu-subtle neu-btn"
            style="flex: 1; padding: 9px; border-radius: var(--radius-sm); font-family: var(--sans); font-size: 13px; color: var(--text-secondary); cursor: pointer;"
          >
            Cancel
          </button>
          <button
            onclick={handleDelete}
            disabled={isDeleting}
            style="
              flex: 1;
              padding: 9px;
              border-radius: var(--radius-sm);
              font-family: var(--sans);
              font-size: 13px;
              font-weight: 600;
              color: white;
              background: var(--danger, #ef4444);
              border: none;
              cursor: pointer;
            "
          >
            {isDeleting ? "Deleting…" : "Delete"}
          </button>
        </div>
      {:else}
        <button
          onclick={() => showDeleteConfirm = true}
          class="neu-subtle neu-btn"
          style="
            width: 100%;
            padding: 10px 14px;
            border-radius: var(--radius-sm);
            font-family: var(--sans);
            font-size: 13px;
            color: var(--danger, #ef4444);
            cursor: pointer;
            display: flex;
            align-items: center;
            gap: 8px;
          "
        >
          <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
          </svg>
          Delete Deck
        </button>
      {/if}
    </div>
  </div>
</NeuDialog>
