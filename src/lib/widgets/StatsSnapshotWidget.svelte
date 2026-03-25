<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { addToast } from "../toast";

  interface ReviewStats {
    current_streak: number;
    longest_streak: number;
    total_reviews: number;
    retention: {
      overall: number;
      young_retention: number;
      mature_retention: number;
    };
  }

  type CollectionStatus = "loading" | "ready" | "error";
  
  let { collectionStatus = "loading" }: { collectionStatus?: CollectionStatus } = $props();

  let stats: ReviewStats | null = $state(null);
  let isLoading = $state(true);
  let todayReviews = $state(0);

  // Load stats when collection becomes ready
  $effect(() => {
    if (collectionStatus === 'ready') {
      loadStats();
    }
  });

  onMount(async () => {
    // Only load if collection is already ready
    if (collectionStatus === 'ready') {
      await loadStats();
    }
  });

  async function loadStats() {
    isLoading = true;
    try {
      const [statsResult, heatmapResult] = await Promise.all([
        invoke<ReviewStats>("get_review_stats", { deckId: null }),
        invoke<Array<{ date: string; count: number }>>("get_review_heatmap", { days: 1, deckId: null })
      ]);

      stats = statsResult;
      
      // Get today's review count from heatmap
      const today = new Date().toISOString().split('T')[0];
      const todayData = heatmapResult.find(d => d.date === today);
      todayReviews = todayData?.count || 0;
    } catch (error) {
      console.error("Error loading stats:", error);
      addToast("Failed to load statistics", "error");
    } finally {
      isLoading = false;
    }
  }

  function formatPercentage(value: number): string {
    return `${(value * 100).toFixed(1)}%`;
  }
</script>

<div class="stats-snapshot-widget" style="height: 100%; overflow: hidden;">
  {#if isLoading}
    <div class="flex items-center justify-center" style="height: 100%;">
      <div class="loading-spinner"></div>
    </div>
  {:else if !stats}
    <div class="flex items-center justify-center" style="height: 100%;">
      <p style="font-family: var(--sans); font-size: 14px; color: var(--text-secondary);">No statistics available</p>
    </div>
  {:else}
    <div class="stats-grid">
      <!-- Today's Reviews -->
      <div class="metric-card p-3 rounded-xl text-center" style="background: var(--bg-subtle);">
        <div class="metric-value" style="font-family: var(--serif); font-size: 24px; font-weight: 600; color: var(--text-primary);">
          {todayReviews}
        </div>
        <div class="metric-label" style="font-family: var(--sans); font-size: 11px; color: var(--text-secondary); margin-top: 4px;">
          Today
        </div>
      </div>

      <!-- Current Streak -->
      <div class="metric-card p-3 rounded-xl text-center" style="background: var(--bg-subtle);">
        <div class="metric-value" style="font-family: var(--serif); font-size: 24px; font-weight: 600; color: var(--accent);">
          {stats.current_streak}
        </div>
        <div class="metric-label" style="font-family: var(--sans); font-size: 11px; color: var(--text-secondary); margin-top: 4px;">
          Streak
        </div>
      </div>

      <!-- Overall Retention -->
      <div class="metric-card p-3 rounded-xl text-center" style="background: var(--bg-subtle);">
        <div class="metric-value" style="font-family: var(--serif); font-size: 24px; font-weight: 600; color: var(--success);">
          {formatPercentage(stats.retention.overall)}
        </div>
        <div class="metric-label" style="font-family: var(--sans); font-size: 11px; color: var(--text-secondary); margin-top: 4px;">
          Retention
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .stats-snapshot-widget {
    height: 100%;
    overflow: hidden;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
    height: 100%;
    align-content: center;
  }

  .loading-spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--bg-subtle);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .metric-card {
    transition: transform 0.2s ease, box-shadow 0.2s ease;
  }

  .metric-card:hover {
    transform: translateY(-2px);
    box-shadow: var(--neu-subtle);
  }
</style>
