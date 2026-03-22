<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { isTauri } from "@tauri-apps/api/core";
  import Chart from "./Chart.svelte";
  import type { ChartData, ChartOptions } from "chart.js";
  import { addToast } from "./toast";

  // Types
  interface ForecastDay {
    day: number;
    new: number;
    review: number;
    learning: number;
  }

  interface DailyReview {
    date: string;
    count: number;
    time_secs: number;
  }

  interface CardTypeStats {
    new: number;
    learning: number;
    young: number;
    mature: number;
  }

  interface RetentionStats {
    young_retention: number;
    mature_retention: number;
    overall: number;
  }

  interface DailyCount {
    date: string;
    count: number;
  }

  interface ReviewStats {
    forecast: ForecastDay[];
    daily_reviews: DailyReview[];
    hourly_breakdown: number[];
    card_types: CardTypeStats;
    retention: RetentionStats;
    cards_added: DailyCount[];
    current_streak: number;
    longest_streak: number;
    total_reviews: number;
    total_cards: number;
    total_notes: number;
    average_ease: number;
    average_interval_days: number;
  }

  interface DeckInfo {
    id: number;
    name: string;
    card_count: number;
    is_filtered: boolean;
  }

  // State
  let isLoading = $state(true);
  let hasError = $state(false);
  let decks = $state<DeckInfo[]>([]);
  let selectedDeckId = $state<number | null>(null);
  let stats = $state<ReviewStats | null>(null);

  // Load decks and stats on mount
  onMount(async () => {
    const tauriCheck = await isTauri();
    if (!tauriCheck) {
      // Mock data for browser testing
      stats = generateMockStats();
      isLoading = false;
      return;
    }

    try {
      // Load decks
      const allDecks = await invoke<any[]>("get_all_decks");
      decks = allDecks.map((d: any) => ({
        id: d.id,
        name: d.name,
        card_count: d.card_count,
        is_filtered: d.is_filtered
      }));
      
      // Load stats (all decks by default)
      await loadStats(null);
    } catch (error) {
      console.error("Error loading stats:", error);
      addToast(error instanceof Error ? error.message : "Failed to load statistics", "error");
      hasError = true;
    } finally {
      isLoading = false;
    }
  });

  async function loadStats(deckId: number | null) {
    isLoading = true;
    try {
      if (deckId === null) {
        stats = await invoke<ReviewStats>("get_review_stats", { deckId: null });
      } else {
        stats = await invoke<ReviewStats>("get_deck_specific_stats", { deckId });
      }
      selectedDeckId = deckId;
    } catch (error) {
      console.error("Error loading stats:", error);
      addToast(error instanceof Error ? error.message : "Failed to load statistics", "error");
    } finally {
      isLoading = false;
    }
  }

  function handleDeckChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    const value = target.value;
    const deckId = value === "all" ? null : parseInt(value, 10);
    loadStats(deckId);
  }

  async function handleRetry() {
    hasError = false;
    isLoading = true;
    try {
      const allDecks = await invoke<any[]>("get_all_decks");
      decks = allDecks.map((d: any) => ({
        id: d.id,
        name: d.name,
        card_count: d.card_count,
        is_filtered: d.is_filtered
      }));
      await loadStats(null);
    } catch (error) {
      console.error("Error loading stats:", error);
      addToast(error instanceof Error ? error.message : "Failed to load statistics", "error");
      hasError = true;
    } finally {
      isLoading = false;
    }
  }

  function generateMockStats(): ReviewStats {
    const forecast: ForecastDay[] = [];
    for (let i = 1; i <= 30; i++) {
      forecast.push({
        day: i,
        new: Math.floor(Math.random() * 10),
        review: Math.floor(Math.random() * 30),
        learning: Math.floor(Math.random() * 5),
      });
    }

    const dailyReviews: DailyReview[] = [];
    for (let i = 29; i >= 0; i--) {
      const date = new Date();
      date.setDate(date.getDate() - i);
      dailyReviews.push({
        date: date.toISOString().split('T')[0],
        count: Math.floor(Math.random() * 50) + 10,
        time_secs: Math.floor(Math.random() * 1800),
      });
    }

    const hourlyBreakdown: number[] = [];
    for (let i = 0; i < 24; i++) {
      hourlyBreakdown.push(Math.floor(Math.random() * 20));
    }

    const cardsAdded: DailyCount[] = [];
    for (let i = 29; i >= 0; i--) {
      const date = new Date();
      date.setDate(date.getDate() - i);
      cardsAdded.push({
        date: date.toISOString().split('T')[0],
        count: Math.floor(Math.random() * 15),
      });
    }

    return {
      forecast,
      daily_reviews: dailyReviews,
      hourly_breakdown: hourlyBreakdown,
      card_types: {
        new: 100,
        learning: 15,
        young: 200,
        mature: 500,
      },
      retention: {
        young_retention: 0.85,
        mature_retention: 0.92,
        overall: 0.90,
      },
      cards_added: cardsAdded,
      current_streak: 7,
      longest_streak: 14,
      total_reviews: 5000,
      total_cards: 815,
      total_notes: 600,
      average_ease: 2.65,
      average_interval_days: 45.5,
    };
  }

  function formatTime(secs: number): string {
    const hours = Math.floor(secs / 3600);
    const minutes = Math.floor((secs % 3600) / 60);
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    }
    return `${minutes} min`;
  }

  function formatPercentage(value: number): string {
    return `${(value * 100).toFixed(1)}%`;
  }

  // Chart: Forecast (stacked bar)
  let forecastChartData = $derived.by(() => {
    if (!stats?.forecast.length) return { labels: [], datasets: [] };

    const labels = stats.forecast.slice(0, 14).map(d => {
      const date = new Date();
      date.setDate(date.getDate() + d.day);
      return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
    });
    
    return {
      labels,
      datasets: [
        {
          label: 'New',
          data: stats.forecast.slice(0, 14).map(d => d.new),
          backgroundColor: 'rgba(196, 113, 79, 0.75)',
          borderRadius: 4,
        },
        {
          label: 'Learning',
          data: stats.forecast.slice(0, 14).map(d => d.learning),
          backgroundColor: 'rgba(107, 143, 113, 0.7)',
          borderRadius: 4,
        },
        {
          label: 'Review',
          data: stats.forecast.slice(0, 14).map(d => d.review),
          backgroundColor: 'rgba(196, 154, 79, 0.75)',
          borderRadius: 4,
        },
      ],
    } as ChartData;
  });

  let forecastChartOptions: ChartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    scales: {
      x: {
        stacked: true,
        grid: { display: false },
        ticks: { color: 'var(--text-secondary)', font: { size: 11, family: 'DM Sans' } },
      },
      y: {
        stacked: true,
        grid: { color: 'rgba(0,0,0,0.06)' },
        ticks: { color: 'var(--text-secondary)', font: { family: 'DM Sans' } },
      },
    },
    plugins: {
      legend: {
        position: 'top',
        labels: { color: 'var(--text-secondary)', boxWidth: 12, padding: 10, font: { family: 'DM Sans' } },
      },
      tooltip: {
        backgroundColor: '#2C2825',
        titleColor: '#fff',
        bodyColor: '#fff',
        borderColor: 'transparent',
        borderWidth: 0,
        cornerRadius: 10,
        padding: 10,
      },
    },
  };

  // Chart: Daily Reviews (bar)
  let dailyReviewsChartData = $derived.by(() => {
    if (!stats?.daily_reviews.length) return { labels: [], datasets: [] };

    const last30 = stats.daily_reviews.slice(-30);
    
    return {
      labels: last30.map(d => {
        const date = new Date(d.date);
        return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
      }),
      datasets: [{
        label: 'Reviews',
        data: last30.map(d => d.count),
        backgroundColor: 'rgba(196, 113, 79, 0.75)',
        borderRadius: 6,
        borderSkipped: false,
      }],
    } as ChartData;
  });

  let dailyReviewsChartOptions: ChartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    scales: {
      x: {
        grid: { display: false },
        ticks: { color: 'var(--text-secondary)', font: { size: 10, family: 'DM Sans' }, maxRotation: 45 },
      },
      y: {
        grid: { color: 'rgba(0,0,0,0.06)' },
        ticks: { color: 'var(--text-secondary)', font: { family: 'DM Sans' } },
      },
    },
    plugins: {
      legend: { display: false },
      tooltip: {
        backgroundColor: '#2C2825',
        titleColor: '#fff',
        bodyColor: '#fff',
        borderColor: 'transparent',
        borderWidth: 0,
        cornerRadius: 10,
        padding: 10,
      },
    },
  };

  // Chart: Hourly Breakdown (bar)
  let hourlyChartData = $derived.by(() => {
    if (!stats?.hourly_breakdown) return { labels: [], datasets: [] };

    const hours = Array.from({ length: 24 }, (_, i) => 
      i === 0 ? '12am' : i < 12 ? `${i}am` : i === 12 ? '12pm' : `${i-12}pm`
    );

    return {
      labels: hours,
      datasets: [{
        label: 'Reviews',
        data: stats.hourly_breakdown,
        backgroundColor: 'rgba(107, 143, 113, 0.7)',
        borderRadius: 4,
        borderSkipped: false,
      }],
    } as ChartData;
  });

  let hourlyChartOptions: ChartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    scales: {
      x: {
        grid: { display: false },
        ticks: { color: 'var(--text-secondary)', font: { size: 10, family: 'DM Sans' } },
      },
      y: {
        grid: { color: 'rgba(0,0,0,0.06)' },
        ticks: { color: 'var(--text-secondary)', font: { family: 'DM Sans' } },
      },
    },
    plugins: {
      legend: { display: false },
      tooltip: {
        backgroundColor: '#2C2825',
        titleColor: '#fff',
        bodyColor: '#fff',
        borderColor: 'transparent',
        borderWidth: 0,
        cornerRadius: 10,
        padding: 10,
      },
    },
  };

  // Chart: Card Types (doughnut)
  let cardTypesChartData = $derived.by(() => {
    if (!stats) return { labels: [], datasets: [] };

    return {
      labels: ['New', 'Learning', 'Young', 'Mature'],
      datasets: [{
        data: [
          stats.card_types.new,
          stats.card_types.learning,
          stats.card_types.young,
          stats.card_types.mature,
        ],
        backgroundColor: ['rgba(196, 113, 79, 0.75)', 'rgba(107, 143, 113, 0.7)', 'rgba(196, 154, 79, 0.75)', 'rgba(196, 113, 79, 0.5)'],
        borderWidth: 0,
      }],
    } as ChartData;
  });

  let cardTypesChartOptions: ChartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: {
        position: 'right',
        labels: { 
          color: 'var(--text-secondary)', 
          boxWidth: 12, 
          padding: 15,
          font: { family: 'DM Sans' }
        },
      },
      tooltip: {
        backgroundColor: '#2C2825',
        titleColor: '#fff',
        bodyColor: '#fff',
        borderColor: 'transparent',
        borderWidth: 0,
        cornerRadius: 10,
        padding: 10,
      },
    },
  } as ChartOptions;

  // Chart: Cards Added (bar)
  let cardsAddedChartData = $derived.by(() => {
    if (!stats?.cards_added.length) return { labels: [], datasets: [] };

    const last30 = stats.cards_added.slice(-30);
    
    return {
      labels: last30.map(d => {
        const date = new Date(d.date);
        return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
      }),
      datasets: [{
        label: 'Cards Added',
        data: last30.map(d => d.count),
        backgroundColor: 'rgba(196, 113, 79, 0.75)',
        borderRadius: 6,
        borderSkipped: false,
      }],
    } as ChartData;
  });

  let cardsAddedChartOptions: ChartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    scales: {
      x: {
        grid: { display: false },
        ticks: { color: 'var(--text-secondary)', font: { size: 10, family: 'DM Sans' }, maxRotation: 45 },
      },
      y: {
        grid: { color: 'rgba(0,0,0,0.06)' },
        ticks: { color: 'var(--text-secondary)', font: { family: 'DM Sans' } },
      },
    },
    plugins: {
      legend: { display: false },
      tooltip: {
        backgroundColor: '#2C2825',
        titleColor: '#fff',
        bodyColor: '#fff',
        borderColor: 'transparent',
        borderWidth: 0,
        cornerRadius: 10,
        padding: 10,
      },
    },
  };
</script>

<div class="stats-container">
  <!-- Header with Deck Filter -->
  <div class="stats-header">
    <h1 class="stats-title">Statistics</h1>
    
    <!-- Deck Filter (hidden on error) -->
    {#if !hasError}
      <div class="deck-filter">
        <label for="deck-select" class="filter-label">Deck:</label>
        <div class="filter-select-wrapper neu-pressed">
          <select
            id="deck-select"
            class="filter-select"
            value={selectedDeckId === null ? "all" : selectedDeckId.toString()}
            onchange={handleDeckChange}
          >
            <option value="all">All Decks</option>
            {#each decks.filter(d => !d.is_filtered) as deck}
              <option value={deck.id.toString()}>{deck.name}</option>
            {/each}
          </select>
          <svg class="select-arrow" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
          </svg>
        </div>
      </div>
    {/if}
  </div>

  <!-- Error State -->
  {#if hasError && !isLoading}
    <div class="error-card neu-raised">
      <div class="error-content">
        <svg class="error-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <p class="error-title">Failed to load statistics</p>
        <p class="error-message">There was a problem retrieving your study data.</p>
      </div>
      <button
        onclick={handleRetry}
        class="retry-btn"
      >
        Retry
      </button>
    </div>
  {/if}

  <!-- Stats Content (hidden on error) -->
  {#if !hasError}
    <!-- Stats Summary Row -->
    <div class="stats-summary">
      <!-- Total Cards -->
      <div class="stat-card neu-raised">
        {#if isLoading}
          <div class="skeleton-line w-16 h-4 mb-2"></div>
          <div class="skeleton-line w-12 h-8"></div>
        {:else}
          <div class="stat-label">Total Cards</div>
          <div class="stat-value">{stats?.total_cards ?? 0}</div>
        {/if}
      </div>

      <!-- Total Notes -->
      <div class="stat-card neu-raised">
        {#if isLoading}
          <div class="skeleton-line w-16 h-4 mb-2"></div>
          <div class="skeleton-line w-12 h-8"></div>
        {:else}
          <div class="stat-label">Total Notes</div>
          <div class="stat-value">{stats?.total_notes ?? 0}</div>
        {/if}
      </div>

      <!-- Total Reviews -->
      <div class="stat-card neu-raised">
        {#if isLoading}
          <div class="skeleton-line w-16 h-4 mb-2"></div>
          <div class="skeleton-line w-12 h-8"></div>
        {:else}
          <div class="stat-label">Total Reviews</div>
          <div class="stat-value">{stats?.total_reviews ?? 0}</div>
        {/if}
      </div>

      <!-- Average Ease -->
      <div class="stat-card neu-raised">
        {#if isLoading}
          <div class="skeleton-line w-16 h-4 mb-2"></div>
          <div class="skeleton-line w-12 h-8"></div>
        {:else}
          <div class="stat-label">Avg. Ease</div>
          <div class="stat-value">{stats?.average_ease.toFixed(2) ?? '0.00'}</div>
        {/if}
      </div>

      <!-- Average Interval -->
      <div class="stat-card neu-raised">
        {#if isLoading}
          <div class="skeleton-line w-20 h-4 mb-2"></div>
          <div class="skeleton-line w-16 h-8"></div>
        {:else}
          <div class="stat-label">Avg. Interval</div>
          <div class="stat-value">{stats?.average_interval_days.toFixed(1) ?? '0'}d</div>
        {/if}
      </div>

      <!-- Retention -->
      <div class="stat-card neu-raised">
        {#if isLoading}
          <div class="skeleton-line w-16 h-4 mb-2"></div>
          <div class="skeleton-line w-12 h-8"></div>
        {:else}
          <div class="stat-label">Retention</div>
          <div class="stat-value">{formatPercentage(stats?.retention.overall ?? 0)}</div>
        {/if}
      </div>
    </div>

    <!-- Streak Section -->
    <div class="streak-card neu-raised">
      <div class="streak-content">
        <div class="streak-icon-wrapper">
          <svg class="streak-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17.657 18.657A8 8 0 016.343 7.343S7 9 9 10c0-2 .5-5 2.986-7C14 5 16.09 5.777 17.656 7.343A7.975 7.975 0 0120 13a7.975 7.975 0 01-2.343 5.657z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.879 16.121A3 3 0 1012.015 11L11 14H9c0 .768.293 1.536.879 2.121z" />
          </svg>
        </div>
        <div class="streak-info">
          <div class="streak-label">Current Streak</div>
          <div class="streak-value">
            {stats?.current_streak ?? 0} <span class="streak-unit">days</span>
          </div>
        </div>
      </div>
      <div class="streak-secondary">
        <div class="streak-label">Longest Streak</div>
        <div class="streak-secondary-value">
          {stats?.longest_streak ?? 0} <span class="streak-unit">days</span>
        </div>
      </div>
    </div>

    <!-- Charts Grid -->
    <div class="charts-grid">
      <!-- Review Forecast -->
      <div class="chart-card neu-raised">
        <h3 class="chart-title">Review Forecast</h3>
        {#if isLoading}
          <div class="skeleton h-64 rounded-lg"></div>
        {:else}
          <div class="chart-container">
            <Chart type="bar" data={forecastChartData} options={forecastChartOptions} />
          </div>
        {/if}
      </div>

      <!-- Daily Reviews -->
      <div class="chart-card neu-raised">
        <h3 class="chart-title">Reviews per Day (Last 30 Days)</h3>
        {#if isLoading}
          <div class="skeleton h-64 rounded-lg"></div>
        {:else}
          <div class="chart-container">
            <Chart type="bar" data={dailyReviewsChartData} options={dailyReviewsChartOptions} />
          </div>
        {/if}
      </div>

      <!-- Hourly Breakdown -->
      <div class="chart-card neu-raised">
        <h3 class="chart-title">Study Time (Hourly)</h3>
        {#if isLoading}
          <div class="skeleton h-64 rounded-lg"></div>
        {:else}
          <div class="chart-container">
            <Chart type="bar" data={hourlyChartData} options={hourlyChartOptions} />
          </div>
        {/if}
      </div>

      <!-- Card Types -->
      <div class="chart-card neu-raised">
        <h3 class="chart-title">Card Types</h3>
        {#if isLoading}
          <div class="skeleton h-64 rounded-lg"></div>
        {:else}
          <div class="chart-container">
            <Chart type="doughnut" data={cardTypesChartData} options={cardTypesChartOptions} />
          </div>
          
          <!-- Card Type Legend -->
          <div class="card-types-legend">
            <div class="legend-item">
              <div class="legend-color" style="background: rgba(196, 113, 79, 0.75)"></div>
              <span class="legend-text">New: {stats?.card_types.new ?? 0}</span>
            </div>
            <div class="legend-item">
              <div class="legend-color" style="background: rgba(107, 143, 113, 0.7)"></div>
              <span class="legend-text">Learning: {stats?.card_types.learning ?? 0}</span>
            </div>
            <div class="legend-item">
              <div class="legend-color" style="background: rgba(196, 154, 79, 0.75)"></div>
              <span class="legend-text">Young: {stats?.card_types.young ?? 0}</span>
            </div>
            <div class="legend-item">
              <div class="legend-color" style="background: rgba(196, 113, 79, 0.5)"></div>
              <span class="legend-text">Mature: {stats?.card_types.mature ?? 0}</span>
            </div>
          </div>
        {/if}
      </div>

      <!-- Cards Added -->
      <div class="chart-card neu-raised chart-wide">
        <h3 class="chart-title">Cards Added (Last 30 Days)</h3>
        {#if isLoading}
          <div class="skeleton h-48 rounded-lg"></div>
        {:else}
          <div class="chart-container chart-short">
            <Chart type="bar" data={cardsAddedChartData} options={cardsAddedChartOptions} />
          </div>
        {/if}
      </div>
    </div>

    <!-- Retention Stats -->
    <div class="retention-card neu-raised">
      <h3 class="chart-title">Retention Rates</h3>
      {#if isLoading}
        <div class="retention-skeleton">
          <div class="skeleton-line w-24 h-16 rounded"></div>
          <div class="skeleton-line w-24 h-16 rounded"></div>
          <div class="skeleton-line w-24 h-16 rounded"></div>
        </div>
      {:else}
        <div class="retention-stats">
          <div class="retention-item">
            <div class="retention-value">{formatPercentage(stats?.retention.young_retention ?? 0)}</div>
            <div class="retention-label">Young Card Retention</div>
          </div>
          <div class="retention-item">
            <div class="retention-value">{formatPercentage(stats?.retention.mature_retention ?? 0)}</div>
            <div class="retention-label">Mature Card Retention</div>
          </div>
          <div class="retention-item">
            <div class="retention-value retention-accent">{formatPercentage(stats?.retention.overall ?? 0)}</div>
            <div class="retention-label">Overall Retention</div>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .stats-container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 32px 24px;
  }

  .stats-header {
    display: flex;
    flex-direction: column;
    gap: 16px;
    margin-bottom: 32px;
  }

  @media (min-width: 768px) {
    .stats-header {
      flex-direction: row;
      align-items: center;
      justify-content: space-between;
    }
  }

  .stats-title {
    font-family: var(--serif);
    font-size: 28px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
    letter-spacing: -0.02em;
  }

  .deck-filter {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .filter-label {
    font-family: var(--sans);
    font-size: 14px;
    color: var(--text-secondary);
  }

  .filter-select-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .filter-select {
    padding: 10px 36px 10px 14px;
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    background: transparent;
    border: none;
    appearance: none;
    cursor: pointer;
    outline: none;
  }

  .select-arrow {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    width: 16px;
    height: 16px;
    color: var(--text-secondary);
    pointer-events: none;
  }

  .error-card {
    padding: 32px;
    text-align: center;
    margin-bottom: 32px;
  }

  .error-content {
    margin-bottom: 20px;
  }

  .error-icon {
    width: 48px;
    height: 48px;
    margin: 0 auto 16px;
    color: var(--danger);
  }

  .error-title {
    font-family: var(--sans);
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 8px 0;
  }

  .error-message {
    font-family: var(--sans);
    font-size: 14px;
    color: var(--text-secondary);
    margin: 0;
  }

  .retry-btn {
    padding: 10px 24px;
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 500;
    color: white;
    background: var(--accent);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .retry-btn:hover {
    opacity: 0.9;
  }

  .stats-summary {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 16px;
    margin-bottom: 24px;
  }

  @media (min-width: 768px) {
    .stats-summary {
      grid-template-columns: repeat(4, 1fr);
    }
  }

  @media (min-width: 1024px) {
    .stats-summary {
      grid-template-columns: repeat(6, 1fr);
    }
  }

  .stat-card {
    padding: 16px;
  }

  .stat-label {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-secondary);
    margin-bottom: 4px;
  }

  .stat-value {
    font-family: var(--serif);
    font-size: 24px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .streak-card {
    display: flex;
    flex-direction: column;
    gap: 20px;
    padding: 24px;
    margin-bottom: 24px;
  }

  @media (min-width: 768px) {
    .streak-card {
      flex-direction: row;
      align-items: center;
      justify-content: space-between;
    }
  }

  .streak-content {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .streak-icon-wrapper {
    padding: 12px;
    background: var(--accent-soft);
    border-radius: var(--radius-md);
  }

  .streak-icon {
    width: 32px;
    height: 32px;
    color: var(--accent);
  }

  .streak-info {
    display: flex;
    flex-direction: column;
  }

  .streak-label {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-secondary);
  }

  .streak-value {
    font-family: var(--serif);
    font-size: 32px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .streak-unit {
    font-size: 18px;
    font-weight: 400;
    color: var(--text-secondary);
  }

  .streak-secondary {
    text-align: right;
  }

  .streak-secondary-value {
    font-family: var(--serif);
    font-size: 24px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .charts-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 24px;
    margin-bottom: 24px;
  }

  @media (min-width: 1024px) {
    .charts-grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }

  .chart-card {
    padding: 24px;
  }

  .chart-wide {
    grid-column: 1 / -1;
  }

  .chart-title {
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 16px 0;
  }

  .chart-container {
    height: 256px;
    position: relative;
  }

  .chart-short {
    height: 192px;
  }

  .card-types-legend {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 16px;
    margin-top: 16px;
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .legend-color {
    width: 8px;
    height: 8px;
    border-radius: 2px;
    flex-shrink: 0;
  }

  .legend-text {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-secondary);
  }

  .retention-card {
    padding: 24px;
  }

  .retention-skeleton {
    display: flex;
    gap: 32px;
  }

  .retention-stats {
    display: flex;
    flex-wrap: wrap;
    gap: 32px;
  }

  .retention-item {
    text-align: center;
  }

  .retention-value {
    font-family: var(--serif);
    font-size: 32px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .retention-accent {
    color: var(--accent);
  }

  .retention-label {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-secondary);
  }

  .skeleton-line {
    background: var(--bg-subtle);
    border-radius: var(--radius-sm);
    animation: shimmer 1.4s infinite linear;
  }

  @keyframes shimmer {
    0% { background-position: -400px 0; }
    100% { background-position: 400px 0; }
  }

  .w-12 { width: 48px; }
  .w-16 { width: 64px; }
  .w-20 { width: 80px; }
  .w-24 { width: 96px; }
  .h-4 { height: 16px; }
  .h-8 { height: 32px; }
  .h-16 { height: 64px; }
  .h-48 { height: 192px; }
  .h-64 { height: 256px; }
  .mb-2 { margin-bottom: 8px; }
  .rounded { border-radius: var(--radius-sm); }
  .rounded-lg { border-radius: var(--radius-md); }
</style>
