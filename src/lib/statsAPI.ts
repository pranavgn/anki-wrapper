// src/lib/statsAPI.ts
import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "@tauri-apps/api/core";

async function requireTauri() {
  if (!(await isTauri())) throw new Error("Stats API requires Tauri runtime");
}

const statsAPI = {
  getReviewStats: async (deckId: number | null = null) => {
    await requireTauri();
    return deckId === null
      ? invoke("get_review_stats", { deckId: null })
      : invoke("get_deck_specific_stats", { deckId });
  },
  getTodayStats: async () => { await requireTauri(); return invoke("get_today_stats"); },
  getHeatmap: async (days = 365, deckId: number | null = null) => {
    await requireTauri(); return invoke("get_review_heatmap", { days, deckId });
  },
  getDifficultCards: async (deckId: number | null = null, limit = 50) => {
    await requireTauri(); return invoke("get_difficult_cards", { deckId, limit });
  },
  resetStats: async () => { await requireTauri(); return invoke("reset_review_stats"); },
  exportCSV: async (outPath: string) => { await requireTauri(); return invoke("export_review_stats_csv", { outPath }); },
  importCSV: async (path: string) => { await requireTauri(); return invoke("import_review_stats_csv", { path }); },
  version: "1.0.0",
};

if (typeof window !== "undefined") (window as any).__ankiStats = statsAPI;
export default statsAPI;
