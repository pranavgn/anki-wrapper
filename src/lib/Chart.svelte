<script lang="ts">
  import { onMount, onDestroy, untrack } from "svelte";
  import type { ChartType, ChartData, ChartOptions } from "chart.js";

  interface Props {
    type: ChartType;
    data: ChartData;
    options?: ChartOptions;
  }

  let { type, data, options = {} }: Props = $props();

  let canvas: HTMLCanvasElement;
  let chart: any = null;
  let ChartJS: any = null;

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

  onMount(async () => {
    // Lazy-load Chart.js only when component mounts
    if (!ChartJS) {
      const chartModule = await import("chart.js");
      ChartJS = chartModule.Chart;
      chartModule.Chart.register(
        chartModule.Title, chartModule.Tooltip, chartModule.Legend,
        chartModule.BarElement, chartModule.CategoryScale, chartModule.LinearScale,
        chartModule.PointElement, chartModule.LineElement, chartModule.ArcElement,
        chartModule.BarController, chartModule.LineController, chartModule.DoughnutController, chartModule.PieController, chartModule.Filler
      );
    }
    
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
    // Only track data and options as dependencies, not chart
    const d = data;
    const o = options;
    untrack(() => {
      if (chart && d) {
        chart.data = cloneData(d);
        if (o) {
          chart.options = prepareOptions(o);
        }
        chart.update();
      }
    });
  });
</script>

<div class="w-full h-full">
  <canvas bind:this={canvas}></canvas>
</div>
