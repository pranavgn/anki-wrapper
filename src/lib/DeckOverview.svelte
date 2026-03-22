<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { addToast } from "./toast";

  interface DeckStat {
    id: number;
    name: string;
    short_name: string;
    level: number;
    new_count: number;
    learn_count: number;
    review_count: number;
    card_count: number;
    is_filtered: boolean;
  }

  let { 
    deck,
    onStudy,
    onBrowse,
    onStats
  }: { 
    deck: DeckStat;
    onStudy: (deckId: number, deckName: string) => void;
    onBrowse: () => void;
    onStats: () => void;
  } = $props();

  // Mock data for charts (will be replaced with real data later)
  const weekData = [12, 8, 15, 10, 18, 6, 14];
  const forecastData = [5, 8, 12, 6, 10, 15, 9];
  const weekDays = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

  // Card type distribution (mock data)
  const cardTypes = [
    { name: 'New', count: deck.new_count, color: '#5B9BD5' },
    { name: 'Learning', count: deck.learn_count, color: '#C49A4F' },
    { name: 'Review', count: deck.review_count, color: '#C4714F' }
  ];

  const totalCards = deck.card_count;
  const streak = 7; // Mock streak data

  // Calculate progress percentage
  const progressPercent = totalCards > 0 
    ? Math.round(((deck.new_count + deck.learn_count + deck.review_count) / totalCards) * 100) 
    : 0;

  function handleStudy() {
    onStudy(deck.id, deck.name);
  }

  function handleBrowse() {
    onBrowse();
  }

  function handleStats() {
    onStats();
  }
</script>

<div class="max-w-[600px] mx-auto px-6 py-12" style="animation: fadeUp 0.4s ease-out;">
  <!-- Header -->
  <div class="text-center mb-10">
    <div class="text-6xl mb-4" style="animation: float 3s ease-in-out infinite;">📚</div>
    <h1 style="font-family: var(--serif); font-size: 32px; font-weight: 600; color: var(--text-primary); margin-bottom: 8px;">
      {deck.short_name || deck.name}
    </h1>
    <p style="font-family: var(--sans); font-size: 14px; color: var(--text-secondary);">
      {totalCards} cards · {streak} day streak 🔥
    </p>
  </div>

  <!-- Stat Pills -->
  <div class="flex justify-center gap-4 mb-10">
    <div 
      class="neu-raised text-center"
      style="
        background: var(--bg-card);
        box-shadow: var(--neu-up);
        border-radius: var(--radius-md);
        padding: 16px 24px;
      "
    >
      <div style="font-family: var(--serif); font-size: 28px; font-weight: 700; color: #5B9BD5;">
        {deck.new_count}
      </div>
      <div style="font-family: var(--sans); font-size: 11px; text-transform: uppercase; color: var(--text-muted); letter-spacing: 0.05em;">
        New
      </div>
    </div>
    <div 
      class="neu-raised text-center"
      style="
        background: var(--bg-card);
        box-shadow: var(--neu-up);
        border-radius: var(--radius-md);
        padding: 16px 24px;
      "
    >
      <div style="font-family: var(--serif); font-size: 28px; font-weight: 700; color: var(--warning);">
        {deck.learn_count}
      </div>
      <div style="font-family: var(--sans); font-size: 11px; text-transform: uppercase; color: var(--text-muted); letter-spacing: 0.05em;">
        Learning
      </div>
    </div>
    <div 
      class="neu-raised text-center"
      style="
        background: var(--bg-card);
        box-shadow: var(--neu-up);
        border-radius: var(--radius-md);
        padding: 16px 24px;
      "
    >
      <div style="font-family: var(--serif); font-size: 28px; font-weight: 700; color: var(--accent);">
        {deck.review_count}
      </div>
      <div style="font-family: var(--sans); font-size: 11px; text-transform: uppercase; color: var(--text-muted); letter-spacing: 0.05em;">
        Review
      </div>
    </div>
  </div>

  <!-- Progress Bar -->
  <div class="mb-10">
    <div 
      class="neu-pressed"
      style="
        background: var(--bg-deep);
        box-shadow: var(--neu-down);
        border-radius: 10px;
        height: 7px;
        overflow: hidden;
      "
    >
      <div 
        style="
          height: 100%;
          width: {progressPercent}%;
          background: linear-gradient(90deg, var(--success), var(--success) 80%, transparent);
          border-radius: 10px;
          transition: width 0.6s ease-out;
        "
      ></div>
    </div>
    <p class="text-center mt-2" style="font-family: var(--sans); font-size: 12px; color: var(--text-muted);">
      {progressPercent}% complete
    </p>
  </div>

  <!-- Mini Charts Section -->
  <div class="grid grid-cols-2 gap-4 mb-10">
    <!-- This Week Chart -->
    <div 
      class="neu-raised"
      style="
        background: var(--bg-card);
        box-shadow: var(--neu-up);
        border-radius: var(--radius-md);
        padding: 20px;
      "
    >
      <h3 style="font-family: var(--sans); font-size: 13px; font-weight: 600; color: var(--text-primary); margin-bottom: 16px;">
        This Week
      </h3>
      <svg viewBox="0 0 200 100" class="w-full h-24">
        {#each weekData as value, i}
          <rect
            x={i * 28 + 10}
            y={100 - (value / 20) * 80}
            width="20"
            height={(value / 20) * 80}
            fill="var(--accent)"
            rx="3"
            opacity="0.8"
          />
          <text
            x={i * 28 + 20}
            y="95"
            text-anchor="middle"
            fill="var(--text-muted)"
            font-size="8"
            font-family="var(--sans)"
          >
            {weekDays[i]}
          </text>
        {/each}
      </svg>
    </div>

    <!-- Forecast Chart -->
    <div 
      class="neu-raised"
      style="
        background: var(--bg-card);
        box-shadow: var(--neu-up);
        border-radius: var(--radius-md);
        padding: 20px;
      "
    >
      <h3 style="font-family: var(--sans); font-size: 13px; font-weight: 600; color: var(--text-primary); margin-bottom: 16px;">
        Forecast
      </h3>
      <svg viewBox="0 0 200 100" class="w-full h-24">
        {#each forecastData as value, i}
          <rect
            x={i * 28 + 10}
            y={100 - (value / 20) * 80}
            width="20"
            height={(value / 20) * 80}
            fill="var(--success)"
            rx="3"
            opacity="0.8"
          />
          <text
            x={i * 28 + 20}
            y="95"
            text-anchor="middle"
            fill="var(--text-muted)"
            font-size="8"
            font-family="var(--sans)"
          >
            {weekDays[i]}
          </text>
        {/each}
      </svg>
    </div>
  </div>

  <!-- Card Types Doughnut -->
  <div 
    class="neu-raised mb-10"
    style="
      background: var(--bg-card);
      box-shadow: var(--neu-up);
      border-radius: var(--radius-md);
      padding: 24px;
    "
  >
    <h3 style="font-family: var(--sans); font-size: 13px; font-weight: 600; color: var(--text-primary); margin-bottom: 20px;">
      Card Types
    </h3>
    <div class="flex items-center gap-6">
      <svg viewBox="0 0 100 100" class="w-24 h-24">
        <!-- Background circle -->
        <circle
          cx="50"
          cy="50"
          r="40"
          fill="none"
          stroke="var(--bg-deep)"
          stroke-width="12"
        />
        <!-- New cards segment -->
        <circle
          cx="50"
          cy="50"
          r="40"
          fill="none"
          stroke="#5B9BD5"
          stroke-width="12"
          stroke-dasharray="{(deck.new_count / totalCards) * 251.2} 251.2"
          stroke-dashoffset="0"
          transform="rotate(-90 50 50)"
        />
        <!-- Learning cards segment -->
        <circle
          cx="50"
          cy="50"
          r="40"
          fill="none"
          stroke="var(--warning)"
          stroke-width="12"
          stroke-dasharray="{(deck.learn_count / totalCards) * 251.2} 251.2"
          stroke-dashoffset="-{(deck.new_count / totalCards) * 251.2}"
          transform="rotate(-90 50 50)"
        />
        <!-- Review cards segment -->
        <circle
          cx="50"
          cy="50"
          r="40"
          fill="none"
          stroke="var(--accent)"
          stroke-width="12"
          stroke-dasharray="{(deck.review_count / totalCards) * 251.2} 251.2"
          stroke-dashoffset="-{((deck.new_count + deck.learn_count) / totalCards) * 251.2}"
          transform="rotate(-90 50 50)"
        />
      </svg>
      <div class="flex-1 grid grid-cols-1 gap-3">
        {#each cardTypes as type}
          <div class="flex items-center gap-3">
            <div 
              class="w-3 h-3 rounded-full"
              style="background: {type.color};"
            ></div>
            <span style="font-family: var(--sans); font-size: 13px; color: var(--text-secondary);">
              {type.name}
            </span>
            <span 
              class="ml-auto"
              style="font-family: var(--sans); font-size: 13px; font-weight: 600; color: var(--text-primary);"
            >
              {type.count}
            </span>
          </div>
        {/each}
      </div>
    </div>
  </div>

  <!-- Action Buttons -->
  <div class="space-y-3">
    <button
      onclick={handleStudy}
      class="w-full neu-raised cursor-pointer"
      style="
        background: linear-gradient(135deg, var(--accent), var(--accent) 80%, transparent);
        box-shadow: var(--neu-up);
        border-radius: var(--radius-md);
        padding: 18px 24px;
        border: none;
      "
    >
      <span style="font-family: var(--serif); font-size: 22px; font-weight: 600; color: white;">
        Study Now
      </span>
    </button>

    <button
      onclick={handleBrowse}
      class="w-full neu-subtle flex items-center justify-center gap-2 cursor-pointer"
      style="
        background: var(--bg-card);
        box-shadow: var(--neu-subtle);
        border-radius: var(--radius-md);
        padding: 14px 24px;
        border: none;
      "
    >
      <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h18M3 14h18m-9-4v8m-7 0h14a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
      </svg>
      <span style="font-family: var(--sans); font-size: 14px; color: var(--text-secondary);">
        Browse All Cards
      </span>
    </button>

    <button
      onclick={handleStats}
      class="w-full neu-subtle flex items-center justify-center gap-2 cursor-pointer"
      style="
        background: var(--bg-card);
        box-shadow: var(--neu-subtle);
        border-radius: var(--radius-md);
        padding: 14px 24px;
        border: none;
      "
    >
      <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
      </svg>
      <span style="font-family: var(--sans); font-size: 14px; color: var(--text-secondary);">
        View Statistics
      </span>
    </button>
  </div>
</div>

<style>
  @keyframes float {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(-4px); }
  }
</style>
