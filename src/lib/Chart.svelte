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

  function resolveCSS(value: any): any {
    if (typeof value === 'string' && value.startsWith('var(')) {
      const varName = value.match(/var\(([^)]+)\)/)?.[1];
      if (varName) {
        return getComputedStyle(document.documentElement).getPropertyValue(varName).trim() || value;
      }
    }
    if (Array.isArray(value)) return value.map(resolveCSS);
    if (value && typeof value === 'object') {
      const resolved: any = {};
      for (const [k, v] of Object.entries(value)) {
        resolved[k] = resolveCSS(v);
      }
      return resolved;
    }
    return value;
  }

  function cloneData(d: ChartData): ChartData {
    return JSON.parse(JSON.stringify(d));
  }

  function prepareOptions(o: ChartOptions): ChartOptions {
    return resolveCSS(JSON.parse(JSON.stringify(o)));
  }

  onMount(() => {
    if (canvas) {
      chart = new ChartJS(canvas, {
        type,
        data: cloneData(data),
        options: prepareOptions(options),
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
        chart.options = prepareOptions(options);
      }
      chart.update();
    }
  });
</script>

<div class="w-full h-full">
  <canvas bind:this={canvas}></canvas>
</div>
