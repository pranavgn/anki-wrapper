import { invoke } from "@tauri-apps/api/core";

const SEED_KEY = "mnemora_seed_v1";

type NotetypeInfo = { id: number; name: string; kind: string; field_count: number };

const SEED_DECKS: Array<{
  name: string;
  cards: Array<[string, string]>;
}> = [
  {
    name: "Japanese Vocabulary",
    cards: [
      ["こんにちは", "Hello"],
      ["ありがとう", "Thank you"],
      ["すみません", "Excuse me / Sorry"],
      ["はい", "Yes"],
      ["いいえ", "No"],
      ["おはようございます", "Good morning"],
      ["おやすみなさい", "Good night"],
      ["水 (みず)", "Water"],
      ["食べる (たべる)", "To eat"],
      ["行く (いく)", "To go"],
    ],
  },
  {
    name: "Science Basics",
    cards: [
      ["What is osmosis?", "The movement of water across a semipermeable membrane from a region of low solute concentration to high."],
      ["What is Newton's first law?", "An object at rest stays at rest and an object in motion stays in motion unless acted upon by an external force."],
      ["What is the speed of light?", "Approximately 299,792,458 metres per second (c) in a vacuum."],
      ["What is the chemical formula for water?", "H₂O — two hydrogen atoms bonded to one oxygen atom."],
      ["What is photosynthesis?", "The process by which plants use sunlight, water, and CO₂ to produce glucose and oxygen."],
      ["What is the atomic number of carbon?", "6 — carbon has 6 protons in its nucleus."],
    ],
  },
  {
    name: "Mathematics",
    cards: [
      ["What is the Pythagorean theorem?", "a² + b² = c², where c is the hypotenuse of a right triangle."],
      ["What is the derivative of sin(x)?", "cos(x)"],
      ["What is Euler's number e to 5 decimal places?", "2.71828"],
      ["What is the area of a circle?", "A = πr², where r is the radius."],
      ["What is a prime number?", "A natural number greater than 1 with no positive divisors other than 1 and itself."],
      ["What is the quadratic formula?", "x = (−b ± √(b²−4ac)) / 2a"],
    ],
  },
];

export async function seedTestData(): Promise<void> {
  if (!import.meta.env.DEV) return;
  if (localStorage.getItem(SEED_KEY)) return;

  try {
    // Only seed if the collection is empty
    const existing = await invoke<any[]>("get_all_decks");
    if (existing.length > 0) {
      localStorage.setItem(SEED_KEY, "1");
      return;
    }

    // Find the Basic notetype (2 fields: Front + Back)
    const notetypes = await invoke<NotetypeInfo[]>("get_all_notetypes");
    const basic = notetypes.find(n => n.name === "Basic" && n.field_count === 2)
      ?? notetypes.find(n => n.kind === "standard" && n.field_count === 2);

    if (!basic) {
      console.warn("[seed] Could not find a 2-field standard notetype — skipping seed.");
      return;
    }

    for (const { name, cards } of SEED_DECKS) {
      const deckId = await invoke<number>("create_deck", { name });
      for (const [front, back] of cards) {
        await invoke("add_note", {
          deckId,
          notetypeId: basic.id,
          fields: [front, back],
          tags: ["demo"],
        });
      }
    }

    localStorage.setItem(SEED_KEY, "1");
    window.dispatchEvent(new CustomEvent("refresh-decks"));
    console.info("[seed] Demo decks created.");
  } catch (err) {
    console.warn("[seed] Failed to seed test data:", err);
  }
}
