<script lang="ts">
  interface Widget {
    id: string;
    type: string;
    title: string;
    component: any;
    props: Record<string, any>;
    order: number;
    gridHeight?: number; // number of grid units (1 unit = 180px). Default 1.
  }

  let { widgets = [] }: { widgets?: Widget[] } = $props();

  // Sort widgets by order
  let sortedWidgets = $derived.by(() => {
    return [...widgets].sort((a, b) => a.order - b.order);
  });

  // Map widget types to grid height units
  function getGridHeight(widget: Widget): number {
    if (widget.gridHeight) return widget.gridHeight;
    // Default heights per widget type
    switch (widget.type) {
      case 'calendar': return 3;
      case 'schedule': return 3;
      case 'stats': return 1;
      default: return 1;         // Plugin widgets default to 1 unit
    }
  }
</script>

<div class="widget-grid">
  {#each sortedWidgets as widget (widget.id)}
    <div
      class="widget-cell"
      style="--grid-units: {getGridHeight(widget)};"
    >
      <div class="widget-card neu-raised" style="background: var(--bg-card); border-radius: var(--radius-md); overflow: visible;">
        <div class="widget-header">
          <h3 style="font-family: var(--sans); font-size: 13px; font-weight: 600; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.04em;">{widget.title}</h3>
        </div>
        <div class="widget-body">
          {@render widget.component(widget.props)}
        </div>
      </div>
    </div>
  {/each}
</div>

<style>
  .widget-grid {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .widget-cell {
    /* Each unit = 180px. Height is enforced by the --grid-units variable. */
    height: calc(var(--grid-units) * 180px);
    min-height: calc(var(--grid-units) * 180px);
    max-height: calc(var(--grid-units) * 180px);
    overflow: visible;
  }

  .widget-card {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: 16px 20px;
    border: 1px solid var(--border);
    overflow: visible;
  }

  .widget-header {
    margin-bottom: 12px;
    flex-shrink: 0;
  }

  .widget-body {
    flex: 1;
    min-height: 0;
    overflow: visible;
    scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
  }

  .widget-body::-webkit-scrollbar {
    width: 3px;
  }
  .widget-body::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 2px;
  }
</style>
