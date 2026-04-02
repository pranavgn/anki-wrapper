import { invoke } from "@tauri-apps/api/core";
import { open, confirm, save } from "@tauri-apps/plugin-dialog";

export type ImportLog = {
  notes_added: number;
  notes_updated: number;
  notes_skipped: number;
  decks_added: string[];
  notetype_used?: string;
};

export type TextImportOptions = {
  deckId: number;
  notetypeName: string;
  delimiter: string;
  htmlEnabled: boolean;
  duplicatePolicy: "update" | "preserve" | "ignore";
};

export class ImportError extends Error {
  constructor(message: string, public readonly isCancelled = false) {
    super(message);
    this.name = "ImportError";
  }
}

// ────────────────────────── IMPORTS ──────────────────────────

export async function pickAndImportApkg(): Promise<ImportLog> {
  const file = await open({
    filters: [{ name: "Anki Package", extensions: ["apkg"] }],
    multiple: false,
  });

  if (!file) {
    throw new ImportError("Import cancelled", true);
  }

  return await invoke<ImportLog>("import_apkg", { path: file });
}

export async function pickAndImportColpkg(): Promise<string> {
  const confirmed = await confirm(
    "This will REPLACE your entire collection. Are you sure?",
    { title: "Replace Collection", kind: "warning" }
  );

  if (!confirmed) {
    throw new ImportError("Import cancelled", true);
  }

  const file = await open({
    filters: [{ name: "Anki Collection", extensions: ["colpkg"] }],
    multiple: false,
  });

  if (!file) {
    throw new ImportError("Import cancelled", true);
  }

  return await invoke<string>("import_colpkg", { path: file });
}

/**
 * Unified text import — handles .txt, .csv, AND .cloze files in one function.
 *
 * All three flow through the same `import_text_file` Tauri command which uses
 * anki-core's `get_csv_metadata` → `import_csv` pipeline. Cloze notetype
 * selection is done via the `options.notetypeName` field ("Cloze").
 *
 * The file picker shows all three extensions so users never need to pick a
 * separate "cloze import" path.
 */
export async function pickAndImportText(
  options: TextImportOptions
): Promise<ImportLog> {
  const file = await open({
    filters: [
      {
        name: "Text / CSV / Cloze",
        extensions: ["txt", "csv", "tsv", "cloze"],
      },
      { name: "All Files", extensions: ["*"] },
    ],
    multiple: false,
  });

  if (!file) {
    throw new ImportError("Import cancelled", true);
  }

  return await invoke<ImportLog>("import_text_file", { path: file, options });
}

// ────────────────────────── EXPORTS ──────────────────────────

export async function exportDeckApkg(
  deckId: number,
  includeScheduling: boolean
): Promise<string> {
  const file = await save({
    filters: [{ name: "Anki Package", extensions: ["apkg"] }],
    defaultPath: "deck.apkg",
  });

  if (!file) {
    throw new ImportError("Export cancelled", true);
  }

  return await invoke<string>("export_deck_apkg", {
    deckId,
    outPath: file,
    includeScheduling,
  });
}

export async function exportCollectionColpkg(): Promise<string> {
  const file = await save({
    filters: [{ name: "Anki Collection", extensions: ["colpkg"] }],
    defaultPath: "collection.colpkg",
  });

  if (!file) {
    throw new ImportError("Export cancelled", true);
  }

  return await invoke<string>("export_collection_colpkg", { outPath: file });
}
