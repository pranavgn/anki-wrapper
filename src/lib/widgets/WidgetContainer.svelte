<script lang="ts">
  import { prefs } from '../prefs.svelte';

  interface Widget {
    id: string;
    type: string;
    title: string;
    component: any;
    props: Record<string, any>;
    order: number;
  }

  let { widgets = [] }: { widgets?: Widget[] } = $props();

  let draggedWidgetId: string | null = $state(null);
  let dragOverWidgetId: string | null = $state(null);

  function handleDragStart(e: DragEvent, widgetId: string) {
    draggedWidgetId = widgetId;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'move';
      e.dataTransfer.setData('text/plain', widgetId);
    }
  }

  function handleDragOver(e: DragEvent, targetWidgetId: string) {
    if (draggedWidgetId === null || draggedWidgetId === targetWidgetId) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    dragOverWidgetId = targetWidgetId;
  }

  function handleDragLeave() {
    dragOverWidgetId = null;
  }

  async function handleDrop(e: DragEvent, targetWidgetId: string) {
    e.preventDefault();
    dragOverWidgetId = null;
    if (draggedWidgetId === null || draggedWidgetId === targetWidgetId) return;

    const draggedIndex = widgets.findIndex(w => w.id === draggedWidgetId);
    const targetIndex = widgets.findIndex(w => w.id === targetWidgetId);

    if (draggedIndex === -1 || targetIndex === -1) return;

    // Reorder widgets
    const newWidgets = [...widgets];
    const [draggedWidget] = newWidgets.splice(draggedIndex, 1);
    newWidgets.splice(targetIndex, 0, draggedWidget);

    // Update order property
    newWidgets.forEach((widget, index) => {
      widget.order = index;
    });

    // Persist order to prefs
    const widgetOrder = newWidgets.map(w => w.id);
    prefs.widget_order = widgetOrder;
    await prefs.save();

    // Update local state
    widgets = newWidgets;

    draggedWidgetId = null;
  }

  function handleDragEnd() {
    draggedWidgetId = null;
    dragOverWidgetId = null;
  }

  // Sort widgets by order
  let sortedWidgets = $derived.by(() => {
    return [...widgets].sort((a, b) => a.order - b.order);
  });
</script>

<div class="widget-container">
  {#each sortedWidgets as widget (widget.id)}
    <div
      class="widget-wrapper"
      draggable="true"
      ondragstart={(e) => handleDragStart(e, widget.id)}
      ondragover={(e) => handleDragOver(e, widget.id)}
      ondragleave={handleDragLeave}
      ondrop={(e) => handleDrop(e, widget.id)}
      ondragend={handleDragEnd}
      style="{dragOverWidgetId === widget.id ? 'outline: 2px dashed var(--accent); outline-offset: 4px;' : ''} {draggedWidgetId === widget.id ? 'opacity: 0.5;' : ''}"
    >
      <div class="neu-raised" style="background: var(--bg-card); border-radius: var(--radius-md); padding: 20px; overflow: hidden;">
        <div class="flex items-center justify-between mb-3" style="cursor: grab;">
          <h3 style="font-family: var(--sans); font-size: 14px; font-weight: 600; color: var(--text-primary);">{widget.title}</h3>
          <svg class="w-4 h-4" style="color: var(--text-muted);" viewBox="0 0 24 24" fill="currentColor">
            <circle cx="9" cy="5" r="1.5"/><circle cx="15" cy="5" r="1.5"/>
            <circle cx="9" cy="12" r="1.5"/><circle cx="15" cy="12" r="1.5"/>
            <circle cx="9" cy="19" r="1.5"/><circle cx="15" cy="19" r="1.5"/>
          </svg>
        </div>
        <div>
          {@render widget.component(widget.props)}
        </div>
      </div>
    </div>
  {/each}
</div>

<style>
  .widget-container {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
    gap: 24px;
  }

  .widget-wrapper {
    transition: opacity 0.2s ease;
  }
</style>
