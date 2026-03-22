<script lang="ts">
  import NeuDialog from "./ui/NeuDialog.svelte";

  interface Props {
    isOpen: boolean;
    onClose: () => void;
  }

  let { isOpen, onClose }: Props = $props();

  const shortcuts = [
    {
      group: "Study",
      items: [
        { key: "Space", description: "Show Answer" },
        { key: "1", description: "Again" },
        { key: "2", description: "Hard" },
        { key: "3", description: "Good" },
        { key: "4", description: "Easy" },
      ]
    },
    {
      group: "Editor",
      items: [
        { key: "Ctrl+Enter", description: "Save Card" },
        { key: "Ctrl+Shift+C", description: "Insert Cloze" },
      ]
    },
    {
      group: "Navigation",
      items: [
        { key: "?", description: "Toggle Shortcuts" },
        { key: "Esc", description: "Close/Back" },
        { key: "Ctrl+Z", description: "Undo" },
      ]
    },
    {
      group: "Flags",
      items: [
        { key: "⌘0", description: "Remove Flag" },
        { key: "⌘1", description: "Flag Red" },
        { key: "⌘2", description: "Flag Orange" },
        { key: "⌘3", description: "Flag Green" },
        { key: "⌘4", description: "Flag Blue" },
        { key: "⌘5", description: "Flag Pink" },
        { key: "⌘6", description: "Flag Turquoise" },
      ]
    }
  ];
</script>

<NeuDialog {isOpen} {onClose} title="Keyboard Shortcuts" size="md">
  <div class="shortcuts-container">
    {#each shortcuts as group}
      <div class="shortcut-group">
        <h3 class="group-header">{group.group}</h3>
        <div class="shortcuts-grid">
          {#each group.items as item}
            <div class="shortcut-item">
              <kbd class="shortcut-key neu-subtle">{item.key}</kbd>
              <span class="shortcut-description">{item.description}</span>
            </div>
          {/each}
        </div>
      </div>
    {/each}
  </div>
</NeuDialog>

<style>
  .shortcuts-container {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .shortcut-group {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .group-header {
    font-family: var(--sans);
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-muted);
    margin: 0;
  }

  .shortcuts-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
  }

  .shortcut-item {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .shortcut-key {
    font-family: 'SF Mono', Monaco, Consolas, monospace;
    font-size: 12px;
    color: var(--text-primary);
    padding: 4px 8px;
    min-width: 60px;
    text-align: center;
  }

  .shortcut-description {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-secondary);
  }
</style>
