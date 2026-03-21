<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    Chart as ChartJS,
    Title, Tooltip, Legend,
    BarElement, CategoryScale, LinearScale,
    PointElement, LineElement, ArcElement,
    BarController, LineController, DoughnutController, PieController, Filler
  } from "chart.js";
  import type { ChartType, ChartData, ChartOptions } from "chart.js";

  ChartJS.register(
    Title, Tooltip, Legend,
    BarElement, CategoryScale, LinearScale,
    PointElement, LineElement, ArcElement,
    BarController, LineController, DoughnutController, PieController, Filler
  );

  interface Props {
    type: ChartType;
    data: ChartData;
    options?: ChartOptions;
  }

  let { type, data, options = {} }: Props = $props();

  let canvas: HTMLCanvasElement;
  let chart: ChartJS | null = null;

  function cloneData(d: ChartData): ChartData {
    return JSON.parse(JSON.stringify(d));
  }

  function cloneOptions(o: ChartOptions): ChartOptions {
    return JSON.parse(JSON.stringify(o));
  }

  onMount(() => {
    if (canvas) {
      chart = new ChartJS(canvas, {
        type,
        data: cloneData(data),
        options: cloneOptions(options),
      });
    }
  });

  onDestroy(() => {
    if (chart) {
      chart.destroy();
      chart = null;
    }
  });

  $effect(() => {
    if (chart && data) {
      chart.data = cloneData(data);
      if (options) {
        chart.options = cloneOptions(options);
      }
      chart.update();
    }
  });
</script>

<div class="w-full h-full">
  <canvas bind:this={canvas}></canvas>
</div>
