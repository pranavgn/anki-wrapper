<script lang="ts">
  let {
    checked = $bindable(false),
    disabled = false,
    onchange = () => {},
  }: {
    checked?: boolean;
    disabled?: boolean;
    onchange?: () => void;
  } = $props();

  function toggle() {
    if (disabled) return;
    checked = !checked;
    onchange();
  }
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  {disabled}
  onclick={toggle}
  class="neu-toggle"
  class:active={checked}
  class:is-disabled={disabled}
>
  <span class="neu-toggle-thumb" class:on={checked}></span>
</button>

<style>
  .neu-toggle {
    position: relative;
    display: inline-flex;
    align-items: center;
    width: 44px;
    height: 24px;
    border-radius: 12px;
    background: var(--bg-deep);
    box-shadow: var(--neu-down);
    transition: background 0.15s ease;
    cursor: pointer;
    border: none;
    padding: 0;
  }

  .neu-toggle.active {
    background: var(--accent);
  }

  .neu-toggle.is-disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .neu-toggle-thumb {
    position: absolute;
    display: block;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--text-muted);
    left: 3px;
    transition: left 0.15s ease, background 0.15s ease;
    box-shadow: 0 1px 3px rgba(0,0,0,0.1);
  }

  .neu-toggle-thumb.on {
    left: 23px;
    background: white;
  }
</style>
