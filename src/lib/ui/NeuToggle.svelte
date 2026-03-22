<script lang="ts">
  export let checked: boolean = false;
  export let disabled: boolean = false;

  function toggle() {
    if (!disabled) {
      checked = !checked;
    }
  }
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  {disabled}
  on:click={toggle}
  class="relative inline-flex items-center cursor-pointer"
  style="
    width: 44px;
    height: 24px;
    border-radius: 12px;
    background: {checked ? 'var(--accent)' : 'var(--bg-deep)'};
    box-shadow: var(--neu-down);
    transition: background 0.2s ease;
    opacity: {disabled ? 0.5 : 1};
    cursor: {disabled ? 'not-allowed' : 'pointer'};
    border: none;
    padding: 0;
  "
>
  <span
    class="absolute block rounded-full"
    style="
      width: 18px;
      height: 18px;
      background: {checked ? 'white' : 'var(--text-muted)'};
      left: {checked ? '23px' : '3px'};
      transition: left 0.2s ease, background 0.2s ease;
      box-shadow: 0 1px 3px rgba(0,0,0,0.1);
    "
  ></span>
</button>
