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

export const MODELS: { value: WorkingModel; title: string; body: string; ready: boolean }[] = [
  {
    value: "fully_local",
    title: "Fully Local",
    body: "Plain matching and ordinary logic — names, types, dates and sizes, and the rules you write. No model, no network, nothing leaves this Mac.",
    ready: true,
  },
  {
    value: "gemma",
    title: "Gemma",
    body: "A local Gemma reads each file and applies the rules plain matching cannot. Runs on this Mac through Ollama; nothing is sent anywhere.",
    ready: false,
  },
  {
    value: "cli_agent",
    title: "CLI Agent",
    body: "Uses Claude Code, Codex or another local CLI agent you already have, as a classifier only. Optional — Menlo never needs one.",
    ready: false,
  },
];

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

