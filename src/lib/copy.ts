/**
 * Fixed copy: the How it works page, the working models, the rule ideas.
 *
 * Kept apart from components so the words can be edited without touching layout, and
 * so every claim the app makes about itself lives in one place to check against what
 * the code actually does.
 */

import type { WorkingModel } from "./types";

export const DOCS = [
  {
    num: "01",
    title: "Scan",
    body: "Menlo looks at the folders in your set — only the top level, never inside a folder you have already organised. Hidden files, half-finished downloads and apps are left out. Nothing moves yet.",
    flow: ["Downloads", "Desktop", "Files in view"],
  },
  {
    num: "02",
    title: "Choose",
    body: "A folder set pairs the places you clear from with the places things belong. Pick a saved set, or build one. Add any folder on your Mac with Add folder.",
    flow: ["Source folders", "Destinations", "Saved set"],
  },
  {
    num: "03",
    title: "Shape",
    body: "Write rules in plain words — “anything with the word lease goes here”. Menlo reads names, file types, years, sizes and ages without any AI, and tells you when a rule needs a model to apply. A subfolder with no rule takes files named like it.",
    flow: ["Documents", "“lease or agreement”", "Contracts"],
  },
  {
    num: "04",
    title: "Move",
    body: "You see every landing before anything happens. Files already sitting in their destination, byte for byte, are caught first — keep them where they are, or send the extra copy to the Trash. Stop at any point; what has landed stays, and nothing else is touched.",
    flow: ["Preview", "Duplicates", "In flight"],
  },
  {
    num: "05",
    title: "Files processed",
    body: "Every run is kept. Open Runs to send back a whole run, one folder of it, or the copies it put in the Trash. A file you have edited since is left alone rather than overwritten.",
    flow: ["Receipt", "Kept in Runs", "Restore any time"],
  },
];

/**
 * Who decides where a file goes once the rules have had their turn.
 *
 * `selectable` marks the ones Settings offers. Laya is on the list because the
 * How it works page explains where this is going, not because it is a setting.
 */
export const MODELS: {
  value: WorkingModel | "laya";
  title: string;
  when: string;
  ready: boolean;
  selectable: boolean;
  body: string;
  /** What it adds over the row above it. */
  gains: string;
  link?: { label: string; href: string };
}[] = [
  {
    value: "fully_local",
    title: "Fully Local",
    when: "Works today",
    ready: true,
    selectable: true,
    body: "Plain matching and ordinary logic — names, types, dates and sizes, and the rules you write. No model, no network, nothing leaves this Mac.",
    gains: "Instant, predictable, and it works with nothing installed. It cannot read a rule like “anything about my flat”, and it cannot tell you what is inside a screenshot.",
  },
  {
    value: "gemma",
    title: "Gemma",
    when: "Next",
    ready: false,
    selectable: true,
    body: "A local Gemma reads each file and applies the rules plain matching cannot. It runs on this Mac through Ollama, reached by running the ollama command — Menlo itself still makes no network calls.",
    gains: "Rules in any wording, and names for screenshots based on what is actually in them. Costs a few seconds per run and about 5 GB on disk.",
  },
  {
    value: "cli_agent",
    title: "CLI Agent",
    when: "Later, optional",
    ready: false,
    selectable: true,
    body: "Claude Code, Codex or another CLI agent you already have, driven in headless mode as a classifier only: text in, text out, no tools and no access to your files.",
    gains: "Nothing extra to install if you already code with one. Menlo never needs it — it is an alternative to Gemma, not a requirement.",
  },
  {
    value: "laya",
    title: "Laya",
    when: "The direction",
    ready: false,
    selectable: false,
    body: "Every decision you confirm is a labelled example: this file, these rules, that folder. Enough of them and a small decision model can be fine-tuned on your own choices to answer in milliseconds, with a calibrated confidence rather than a sentence.",
    gains: "The model step stops being a wait. Menlo already records what you approve, which is the part that makes this possible.",
    link: { label: "Laya, by Convai Innovations", href: "https://laya.convaiinnovations.com/" },
  },
];

/** The models Settings offers, narrowed to the setting they write. */
export const SELECTABLE_MODELS = MODELS.filter((m) => m.selectable) as (Omit<
  (typeof MODELS)[number],
  "value"
> & { value: WorkingModel })[];

/** Dashed suggestion chips on the rule popup and Rules memory, by folder label. */
const IDEAS: Record<string, string[]> = {
  Documents: [
    "Anything with the word lease or agreement goes here",
    "Keep tax forms and receipts together",
    "PDFs from 2026",
  ],
  Pictures: ["Screenshots go here", "Only images over 1 MB"],
  Movies: ["Recordings over 500 MB", "Skip anything under 1 MB"],
  Music: ["Podcasts and audio files"],
  Developer: ["Code files", "Skip anything under 2 KB"],
};

// Every idea here is one `prose::read` understands without a model, so a chip the user
// taps is a rule that works in Fully Local mode.
const BASE_IDEAS = ["PDFs go here", "Only files over 10 MB", "Skip anything under 2 KB"];
const SUB_IDEAS = ["Anything dated 2026", "Only files created this month", "Screenshots older than 30 days"];

export function ideasFor(label: string, isSubfolder: boolean): string[] {
  if (isSubfolder) return SUB_IDEAS;
  return IDEAS[label] ?? BASE_IDEAS;
}

/**
 * The built-in extension categories (`rules::BUILTIN_CATEGORIES`) and what each files
 * without a rule. A destination whose key is one of these gets that routing for free;
 * any other folder gets nothing until it has a rule — so the UI must only claim
 * by-type filing for these keys.
 */
export const BUILTIN_KINDS: Record<string, string> = {
  documents: "PDFs and documents",
  spreadsheets: "Spreadsheets",
  presentations: "Presentations",
  images: "Photos and images",
  video: "Videos",
  audio: "Audio files",
  archives: "Archives",
  code: "Code files",
  design: "Design files",
  ebooks: "Ebooks",
  fonts: "Fonts",
  installers: "Installers",
};

