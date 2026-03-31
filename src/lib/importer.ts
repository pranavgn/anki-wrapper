import { invoke } from "@tauri-apps/api/core";
import { open, confirm, save } from "@tauri-apps/plugin-dialog";
import { readTextFile } from "@tauri-apps/plugin-fs";

export type ImportLog = {
  notes_added: number;
  notes_updated: number;
  notes_skipped: number;
  decks_added: string[];
};

// Use snake_case to match the Rust struct TextImportOptions
// Rust fields: deck_id, notetype_name, delimiter, html_enabled, duplicate_policy
export type TextImportOptions = {
  deck_id: number;
  notetype_name: string;
  delimiter: string;
  html_enabled: boolean;
  duplicate_policy: "update" | "preserve" | "ignore";
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
      { name: "Text Files", extensions: ["txt", "csv", "tsv"] },
      { name: "All Files", extensions: ["*"] },
    ],
    multiple: false,
  });

  if (!file) {
    throw new ImportError("Import cancelled", true);
  }

  return await invoke<ImportLog>("import_text_file", { path: file, options });
}

// ────────────────────────────────────────────────────────
// Cloze / Markdown text importer
// Reads the file client-side, parses lines into cloze notes,
// and adds them via add_note (bypassing the broken CSV path)
// ────────────────────────────────────────────────────────

export interface ClozeImportOptions {
  deckId: number;
  tags: string[];
  /** If true, auto-wrap key terms in {{c1::…}} for lines that have no cloze syntax */
  autoCloze: boolean;
}

/**
 * Parse a text/markdown file into individual cloze note strings.
 * Strips markdown headers, horizontal rules, blank lines, and comment lines.
 * Each remaining non-empty paragraph becomes one note.
 */
export function parseClozeLines(content: string): string[] {
  const lines = content.split("\n");
  const notes: string[] = [];

  for (const raw of lines) {
    const line = raw.trim();

    // Skip empty lines
    if (!line) continue;

    // Skip markdown headers (# lines) — they are section labels, not content
    if (/^#{1,6}\s/.test(line)) continue;

    // Skip horizontal rules (--- or ***)
    if (/^[-*_]{3,}\s*$/.test(line)) continue;

    // Skip comment lines (lines starting with # without space, or HTML comments)
    if (/^<!--/.test(line) || /^-->/.test(line)) continue;

    // Skip lines that are purely metadata/instructions (starting with * and ending with *)
    if (/^\*[^*].*\*$/.test(line)) continue;

    // Keep this line as a note
    notes.push(line);
  }

  return notes;
}

/**
 * Auto-wrap key terms in cloze syntax if the line doesn't already contain cloze markers.
 * Strategy: look for capitalized proper nouns, dates, and quoted terms as cloze targets.
 */
function autoClozeWrap(text: string): string {
  // If already has cloze syntax, return as-is
  if (/\{\{c\d+::/.test(text)) return text;

  let clozeNum = 1;
  let result = text;

  // Pattern 1: Wrap year numbers (4-digit years)
  result = result.replace(
    /\b(\d{4})\b/,
    () => `{{c${clozeNum++}::$1}}`
  );

  // Pattern 2: Wrap the first quoted term
  result = result.replace(
    /"([^"]+)"/,
    (_, content) => `"{{c${clozeNum++}::${content}}}"`
  );

  // Pattern 3: Wrap proper nouns after "is" or "was" or "called" (first match only)
  if (clozeNum <= 2) {
    result = result.replace(
      /\b(?:is|was|are|called|named|known as|founded|originates? from)\s+([A-Z][a-zA-Z\s]+?)(?=[,.]|\s(?:and|or|in|of|the|a|from|by|with|who|which|that))/,
      (match, name) => match.replace(name, `{{c${clozeNum++}::${name.trim()}}}`)
    );
  }

  // If we only managed one or zero clozes, try wrapping the subject
  if (clozeNum <= 1) {
    // Wrap the first capitalized multi-word term at sentence start
    result = result.replace(
      /^([A-Z][a-zA-Z]+(?:\s[A-Z][a-zA-Z]+)*)/,
      `{{c${clozeNum++}::$1}}`
    );
  }

  return result;
}

/**
 * Pick a text/markdown file and import lines as cloze notes.
 * This bypasses import_text_file entirely and uses add_note directly.
 */
export async function pickAndImportClozeText(
  options: ClozeImportOptions
): Promise<ImportLog> {
  const filePath = await open({
    filters: [
      { name: "Text / Markdown", extensions: ["txt", "md", "csv", "tsv"] },
      { name: "All Files", extensions: ["*"] },
    ],
    multiple: false,
  });

  if (!filePath) {
    throw new ImportError("Import cancelled", true);
  }

  // Read the file content client-side
  const content = await readTextFile(filePath as string);

  // Parse into individual note lines
  let noteLines = parseClozeLines(content);

  if (noteLines.length === 0) {
    throw new ImportError("No valid note content found in the file.");
  }

  // Optionally auto-wrap cloze syntax
  if (options.autoCloze) {
    noteLines = noteLines.map(autoClozeWrap);
  }

  // Find or verify cloze notetype exists
  const notetypes = await invoke<Array<{ id: number; name: string; kind: string }>>(
    "get_all_notetypes"
  );
  const clozeType = notetypes.find((nt) => nt.kind === "cloze");
  if (!clozeType) {
    throw new ImportError(
      "No Cloze note type found in your collection. Please create one first."
    );
  }

  // Get notetype detail for field names
  const detail = await invoke<{ fields: Array<{ ord: number; name: string }> }>(
    "get_notetype_detail",
    { notetypeId: clozeType.id }
  );

  let added = 0;
  let skipped = 0;

  for (const line of noteLines) {
    // Build fields array: first field = cloze text, rest = empty
    const fields = new Array(detail.fields.length).fill("");
    fields[0] = line;

    try {
      await invoke("add_note", {
        deckId: options.deckId,
        notetypeId: clozeType.id,
        fields,
        tags: options.tags,
      });
      added++;
    } catch (e) {
      // Likely a duplicate — count as skipped
      skipped++;
    }
  }

  return {
    notes_added: added,
    notes_updated: 0,
    notes_skipped: skipped,
    decks_added: [],
  };
}

// ────────────────────────────────────────────────────────
// Export functions
// ────────────────────────────────────────────────────────

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
