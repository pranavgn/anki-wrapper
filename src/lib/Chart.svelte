<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Chart as ChartJS, Title, Tooltip, Legend, BarElement, CategoryScale, LinearScale, PointElement, LineElement, ArcElement } from "chart.js";
  import type { ChartType, ChartData, ChartOptions } from "chart.js";

  // Register Chart.js components
  ChartJS.register(Title, Tooltip, Legend, BarElement, CategoryScale, LinearScale, PointElement, LineElement, ArcElement);

  // Props using Svelte 5 runes
  interface Props {
    type: ChartType;
    data: ChartData;
    options?: ChartOptions;
  }

  let { type, data, options = {} }: Props = $props();

  let canvas: HTMLCanvasElement;
  let chart: ChartJS | null = null;

  onMount(() => {
    if (canvas) {
      chart = new ChartJS(canvas, {
        type,
        data,
        options
      });
    }
  });

  onDestroy(() => {
    if (chart) {
      chart.destroy();
    }
  });

  // Reactive data updates
  $effect(() => {
    if (chart && data) {
      chart.data = data;
      chart.update();
    }
  });
</script>

<div class="w-full h-full">
  <canvas bind:this={canvas}></canvas>
</div>
