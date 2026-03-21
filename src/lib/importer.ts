import { invoke } from "@tauri-apps/api/core";
import { open, confirm, save } from "@tauri-apps/plugin-dialog";

export type ImportLog = {
  notes_added: number;
  notes_updated: number;
  notes_skipped: number;
  decks_added: string[];
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

export async function pickAndImportText(
  options: TextImportOptions
): Promise<ImportLog> {
  const file = await open({
    filters: [
      { name: "Text Files", extensions: ["txt", "csv"] },
      { name: "All Files", extensions: ["*"] },
    ],
    multiple: false,
  });

  if (!file) {
    throw new ImportError("Import cancelled", true);
  }

  return await invoke<ImportLog>("import_text_file", { path: file, options });
}

// Export functions

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
