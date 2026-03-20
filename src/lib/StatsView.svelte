<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { isTauri } from "@tauri-apps/api/core";
  import Chart from "./Chart.svelte";
  import type { ChartData, ChartOptions } from "chart.js";
  import { addToast } from "./toast";

  // State
  let isLoading = $state(true);
  let todayStats = $state<{ cards_reviewed: number; time_spent_ms: number; streak_days: number } | null>(null);
  let reviewHistory = $state<Array<{ date: string; count: number; again: number; hard: number; good: number; easy: number }>>([]);

  // Load stats on mount
  onMount(async () => {
    const tauriCheck = await isTauri();
    if (!tauriCheck) {
      // Mock data for browser testing
      todayStats = {
        cards_reviewed: 47,
        time_spent_ms: 840000, // 14 minutes
        streak_days: 7
      };
      reviewHistory = generateMockReviewHistory();
      isLoading = false;
      return;
    }

    try {
      const [todayResult, historyResult] = await Promise.all([
        invoke<{ cards_reviewed: number; time_spent_ms: number }>("get_today_stats"),
        invoke<Array<{ timestamp: number; card_id: number; ease: number; interval: number; last_interval: number; ease_factor: number; time_ms: number; review_type: number }>>("get_review_history", { limit: 1000 })
      ]);

      // Transform today's stats to include streak (we'll calculate this)
      todayStats = {
        ...todayResult,
        streak_days: 7 // TODO: calculate actual streak from review history
      };

      // Transform review history to daily aggregates
      reviewHistory = processReviewHistory(historyResult);
    } catch (error) {
      console.error("Error loading stats:", error);
      addToast(error instanceof Error ? error.message : "Failed to load statistics", "error");
      // Fallback to mock data
      todayStats = {
        cards_reviewed: 47,
        time_spent_ms: 840000,
        streak_days: 7
      };
      reviewHistory = generateMockReviewHistory();
    } finally {
      isLoading = false;
    }
  });

  function processReviewHistory(rawHistory: Array<{ timestamp: number; card_id: number; ease: number; interval: number; last_interval: number; ease_factor: number; time_ms: number; review_type: number }>) {
    const dailyMap = new Map<string, { count: number; again: number; hard: number; good: number; easy: number }>();

    rawHistory.forEach(entry => {
      const date = new Date(entry.timestamp * 1000).toISOString().split('T')[0];

      if (!dailyMap.has(date)) {
        dailyMap.set(date, { count: 0, again: 0, hard: 0, good: 0, easy: 0 });
      }

      const day = dailyMap.get(date)!;
      day.count++;

      switch (entry.ease) {
        case 1: day.again++; break;
        case 2: day.hard++; break;
        case 3: day.good++; break;
        case 4: day.easy++; break;
      }
    });

    // Get last 7 days
    const result = [];
    for (let i = 6; i >= 0; i--) {
      const date = new Date();
      date.setDate(date.getDate() - i);
      const dateStr = date.toISOString().split('T')[0];

      const data = dailyMap.get(dateStr) || { count: 0, again: 0, hard: 0, good: 0, easy: 0 };
      result.push({
        date: dateStr,
        count: data.count,
        again: data.again,
        hard: data.hard,
        good: data.good,
        easy: data.easy
      });
    }

    return result;
  }

  function generateMockReviewHistory() {
    const result = [];
    for (let i = 6; i >= 0; i--) {
      const date = new Date();
      date.setDate(date.getDate() - i);
      const dateStr = date.toISOString().split('T')[0];

      result.push({
        date: dateStr,
        count: Math.floor(Math.random() * 50) + 10,
        again: Math.floor(Math.random() * 5),
        hard: Math.floor(Math.random() * 8),
        good: Math.floor(Math.random() * 30),
        easy: Math.floor(Math.random() * 15)
      });
    }
    return result;
  }

  function formatTime(ms: number): string {
    const minutes = Math.floor(ms / 60000);
    return `${minutes} min`;
  }

  // Chart configurations
  let weekChartData = $derived.by(() => {
    if (!reviewHistory.length) return { labels: [], datasets: [] };

    return {
      labels: reviewHistory.map(day => {
        const date = new Date(day.date);
        return date.toLocaleDateString('en-US', { weekday: 'short' });
      }),
      datasets: [{
        data: reviewHistory.map(day => day.count),
        backgroundColor: 'rgba(196, 113, 79, 0.85)',
        borderRadius: 8,
        borderSkipped: false,
      }]
    } as ChartData;
  });

  let weekChartOptions: ChartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    scales: {
      x: {
        grid: {
          display: false,
        },
        ticks: {
          color: 'var(--text-secondary)',
          font: {
            size: 12,
          },
        },
      },
      y: {
        display: false,
      },
    },
    plugins: {
      legend: {
        display: false,
      },
      tooltip: {
        backgroundColor: '#1C1917',
        titleColor: '#FFFFFF',
        bodyColor: '#FFFFFF',
        cornerRadius: 8,
        padding: 10,
        callbacks: {
          title: (context: any) => {
            const dayIndex = context[0].dataIndex;
            const fullDate = reviewHistory[dayIndex].date;
            const date = new Date(fullDate);
            return date.toLocaleDateString('en-US', { weekday: 'long', month: 'short', day: 'numeric' });
          },
          label: (context: any) => `${context.parsed.y} cards reviewed`,
        },
      },
    },
    animation: {
      duration: 800,
      easing: 'easeOutQuart' as any,
    },
  };

  let breakdownChartData = $derived.by(() => {
    if (!reviewHistory.length) return { labels: [], datasets: [] };

    const totals = reviewHistory.reduce(
      (acc, day) => ({
        again: acc.again + day.again,
        hard: acc.hard + day.hard,
        good: acc.good + day.good,
        easy: acc.easy + day.easy,
      }),
      { again: 0, hard: 0, good: 0, easy: 0 }
    );

    return {
      labels: ['Again', 'Hard', 'Good', 'Easy'],
      datasets: [{
        data: [totals.again, totals.hard, totals.good, totals.easy],
        backgroundColor: ['#FCA5A5', '#FCD34D', '#93C5FD', '#6EE7B7'],
        borderWidth: 0,
      }],
    } as ChartData;
  });

  let breakdownChartOptions: ChartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: {
        display: false,
      },
      tooltip: {
        backgroundColor: '#1C1917',
        titleColor: '#FFFFFF',
        bodyColor: '#FFFFFF',
        cornerRadius: 8,
        padding: 10,
        callbacks: {
          label: (context: any) => `${context.parsed} ${context.label.toLowerCase()}`,
        },
      },
    },
    animation: {
      duration: 800,
      easing: 'easeOutQuart' as any,
    },
  };
</script>

<div class="max-w-6xl mx-auto">
  <!-- Today's Stats -->
  <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-8">
    <!-- Reviewed -->
    <div class="bg-bg-card rounded-2xl p-6 shadow-warm border border-border animate-in fade-in slide-in-from-bottom-2" style="animation-delay: 0ms">
      {#if isLoading}
        <div class="animate-pulse">
          <div class="w-6 h-6 bg-bg-subtle rounded mb-2"></div>
          <div class="w-16 h-10 bg-bg-subtle rounded mb-2"></div>
          <div class="w-20 h-4 bg-bg-subtle rounded"></div>
        </div>
      {:else}
        <div class="flex items-start justify-between mb-4">
          <svg class="w-6 h-6 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
          </svg>
        </div>
        <div class="text-4xl font-semibold text-text-primary mb-1">
          {todayStats?.cards_reviewed ?? 0}
        </div>
        <div class="text-sm text-text-secondary uppercase tracking-wider">
          Reviewed
        </div>
      {/if}
    </div>

    <!-- Time -->
    <div class="bg-bg-card rounded-2xl p-6 shadow-warm border border-border animate-in fade-in slide-in-from-bottom-2" style="animation-delay: 60ms">
      {#if isLoading}
        <div class="animate-pulse">
          <div class="w-6 h-6 bg-bg-subtle rounded mb-2"></div>
          <div class="w-16 h-10 bg-bg-subtle rounded mb-2"></div>
          <div class="w-20 h-4 bg-bg-subtle rounded"></div>
        </div>
      {:else}
        <div class="flex items-start justify-between mb-4">
          <svg class="w-6 h-6 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <circle cx="12" cy="12" r="10" stroke-width="2" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6l4 2" />
          </svg>
        </div>
        <div class="text-4xl font-semibold text-text-primary mb-1">
          {formatTime(todayStats?.time_spent_ms ?? 0)}
        </div>
        <div class="text-sm text-text-secondary uppercase tracking-wider">
          Time
        </div>
      {/if}
    </div>

    <!-- Streak -->
    <div class="bg-bg-card rounded-2xl p-6 shadow-warm border border-border animate-in fade-in slide-in-from-bottom-2" style="animation-delay: 120ms">
      {#if isLoading}
        <div class="animate-pulse">
          <div class="w-6 h-6 bg-bg-subtle rounded mb-2"></div>
          <div class="w-16 h-10 bg-bg-subtle rounded mb-2"></div>
          <div class="w-20 h-4 bg-bg-subtle rounded"></div>
        </div>
      {:else}
        <div class="flex items-start justify-between mb-4">
          <svg class="w-6 h-6 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17.657 18.657A8 8 0 016.343 7.343S7 9 9 10c0-2 .5-5 2.986-7C14 5 16.09 5.777 17.656 7.343A7.975 7.975 0 0120 13a7.975 7.975 0 01-2.343 5.657z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.879 16.121A3 3 0 1012.015 11L11 14H9c0 .768.293 1.536.879 2.121z" />
          </svg>
        </div>
        <div class="text-4xl font-semibold text-text-primary mb-1">
          {todayStats?.streak_days ?? 0}
        </div>
        <div class="text-sm text-text-secondary uppercase tracking-wider">
          Streak
        </div>
      {/if}
    </div>
  </div>

  <!-- Charts Row -->
  <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
    <!-- Reviews This Week -->
    <div class="bg-bg-card rounded-2xl p-6 shadow-warm border border-border animate-in fade-in slide-in-from-bottom-2" style="animation-delay: 180ms">
      <h3 class="text-lg font-semibold text-text-primary mb-6">This Week</h3>
      {#if isLoading}
        <div class="skeleton h-64 rounded-lg"></div>
      {:else}
        <div class="h-64">
          <Chart type="bar" data={weekChartData} options={weekChartOptions} />
        </div>
      {/if}
    </div>

    <!-- Answer Breakdown -->
    <div class="bg-bg-card rounded-2xl p-6 shadow-warm border border-border animate-in fade-in slide-in-from-bottom-2" style="animation-delay: 240ms">
      <h3 class="text-lg font-semibold text-text-primary mb-6">Answer Breakdown</h3>
      {#if isLoading}
        <div class="skeleton space-y-4">
          <div class="h-64 rounded-lg"></div>
          <div class="flex justify-center gap-6">
            <div class="w-16 h-4 rounded"></div>
            <div class="w-16 h-4 rounded"></div>
            <div class="w-16 h-4 rounded"></div>
            <div class="w-16 h-4 rounded"></div>
          </div>
        </div>
      {:else}
        <div class="h-64 relative">
          <Chart type="doughnut" data={breakdownChartData} options={breakdownChartOptions} />
          {#if todayStats}
            <div class="absolute inset-0 flex items-center justify-center">
              <div class="text-center">
                <div class="text-3xl font-semibold text-text-primary">
                  {todayStats.cards_reviewed}
                </div>
                <div class="text-sm text-text-secondary">Total</div>
              </div>
            </div>
          {/if}
        </div>

        <!-- Custom Legend -->
        <div class="grid grid-cols-4 gap-4 mt-6">
          <div class="flex items-center gap-2">
            <div class="w-3 h-3 rounded-sm bg-[#FCA5A5]"></div>
            <span class="text-sm text-text-secondary">Again</span>
          </div>
          <div class="flex items-center gap-2">
            <div class="w-3 h-3 rounded-sm bg-[#FCD34D]"></div>
            <span class="text-sm text-text-secondary">Hard</span>
          </div>
          <div class="flex items-center gap-2">
            <div class="w-3 h-3 rounded-sm bg-[#93C5FD]"></div>
            <span class="text-sm text-text-secondary">Good</span>
          </div>
          <div class="flex items-center gap-2">
            <div class="w-3 h-3 rounded-sm bg-[#6EE7B7]"></div>
            <span class="text-sm text-text-secondary">Easy</span>
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>
