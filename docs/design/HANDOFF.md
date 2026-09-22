# Handoff: Menlo — glass file-tidying flow

## Overview
Menlo is a desktop app that clears files out of source folders (Downloads, Desktop, Inbox) into
destination folders (Documents, Pictures, Developer, Movies, Music, Archives) in a single pass.
The user picks a saved *folder set*, shapes the destination folders and the plain-language rules
attached to them, watches the files move, and can restore any past run at any time.

The whole product is one window: a home screen, a four-stage flow inside it, and five full-window
pages (How it works, Rules memory, Runs, Folder sets, Settings) that open over it.

## About the Design Files
The file in this bundle is a **design reference written in HTML** — a working prototype of the
intended look and behaviour, not production code to lift. The task is to **recreate these designs
in the target codebase's own environment** (Electron + React, SwiftUI, Tauri, whatever the app
uses) with its established patterns, component library and state management. If no environment
exists yet, pick the framework that best fits a desktop file-management app and build the designs
there. The prototype's own runtime (a small template/logic component format) is scaffolding for
the preview and should not be ported.

## Fidelity
**High fidelity.** Colours, type, spacing, radii, blur values, easing curves and copy are final and
specified below. The layout is fixed at a 1040×720 window; treat that as the design size for a
resizable desktop window (the two-column lists and card grids are the parts that should flex).

---

## Design tokens

### Colour
| Token | Value | Use |
| --- | --- | --- |
| Window base | `linear-gradient(180deg,#0C0E13 0%,#07080B 100%)` | app window fill |
| Page behind window | `#060709` | desk background |
| Ink primary | `#F7F8FA` | headlines, active labels |
| Ink body | `#EDEEF1` | default text / inputs |
| Ink 86% | `rgba(237,238,241,0.86)` | list rows |
| Ink 70% | `rgba(237,238,241,0.70)` | eyebrows, secondary |
| Ink 66% | `rgba(237,238,241,0.66)` | meta, counts |
| Ink 60% | `rgba(237,238,241,0.60)` | hints, disabled |
| Solid button | `#F7F8FA` bg / `#0B0D11` text, hover `#FFFFFF` | primary CTA |
| Glass fill | `rgba(255,255,255,0.04–0.07)` | cards, bars, panels |
| Glass fill selected | `rgba(255,255,255,0.13–0.16)` | selected rows, chips |
| Hairline | `rgba(255,255,255,0.09–0.16)` | borders |
| Rim highlight | `inset 0 1px 0 rgba(255,255,255,0.14–0.26)` | top edge of glass |
| Sheet fill | `rgba(18,20,26,0.94)` | modals |
| Scrim | `rgba(6,7,9,0.60–0.72)` + `blur(6–8px)` | behind modals/pages |

Ambient light pools behind the content (three animated radial gradients, `pointer-events:none`):
blue `rgba(96,132,214,0.30)`, magenta `rgba(196,120,196,0.20)`, teal `rgba(88,176,168,0.18)`,
animating on 11s / 14s / 16s ease-in-out loops. A `inset 0 0 90px rgba(0,0,0,0.55)` vignette sits
over them.

### Type
Instrument Sans (400/500/600), fallback `SF Pro Text, -apple-system, BlinkMacSystemFont, sans-serif`.
Tabular numerals on (`font-feature-settings:'tnum' 1`).

| Role | Size / weight / tracking |
| --- | --- |
| Home greeting | 52px / 400 / -0.04em |
| Move counter | 56px / 400 / -0.04em |
| Receipt headline | 44px / 400 / -0.035em |
| Stage headline | 27–30px / 500 / -0.022em |
| Page title (overlays) | 20–21px / 500 / -0.02em |
| Section title | 17px / 500 / -0.015em |
| Card title | 13–14px / 500 / -0.01em |
| Body / rows | 12.5–13px / 400 |
| Meta, rules, hints | 11–11.5px / 400, line-height 1.45–1.6 |
| Eyebrow | 10.5px / 400 / 0.22em / uppercase |
| Wordmark | 15px / 500 / 0.34em / uppercase |

### Geometry & motion
- Window radius 18px; sheets 20–22px; cards 13–18px; rows 10–13px; pills/chips 11–17px; circles for icon buttons.
- Glass: `backdrop-filter: blur(26–48px) saturate(140–200%)` plus a 1px hairline and an inset top highlight.
- Standard easing `cubic-bezier(.32,.72,.16,1)`; 200–340ms for UI transitions, 900ms for the stage track, 420ms for row selection, springy `cubic-bezier(.34,1.4,.5,1)` for checkmark dots.
- Keyframes used: `menlo-rise` (6px + fade, entrances), `menlo-fly` (file flight, 1.05s), `menlo-drift` (background glyphs, 7.5–12.5s), `menlo-orb-a/b/c` (light pools), `menlo-sort` (sorting/scanning cards), `menlo-glint` (CTA specular, 20s), `menlo-fade-in`.

---

## Window chrome

**Top bar** — 56px, glass `rgba(255,255,255,0.06)` + `blur(34px) saturate(180%)`, bottom hairline.
Left: `MENLO` wordmark and a "How it works?" text link (opens the docs page). Right: step counter
(`Home` or `02 / 04`) and a 24px circular avatar button "M" that opens the profile menu.

**Profile menu** — 260px glass popover at `right:20px; top:62px`, `rgba(255,255,255,0.07)` +
`blur(48px) saturate(200%)`, rows label-left / value-right:
Rules memory (N saved) · Runs (N kept) · Folder sets (N saved) · How it works (5 stages) ·
Settings (current working model, first word). A transparent full-window layer behind it closes it.

**Bottom bar** — 74px, same glass as the top bar. Left: context note (e.g. "35 files in view",
"Stopped · 12 of 31 moved"). Centre: the primary CTA on home, the stage progress dots in the flow
(2px pills, active one 26px wide). Right: Back / Stop / Restore / Continue / Move N / Done.

**Primary CTA — "Let's tidy up"** — 46px pill, gradient
`linear-gradient(168deg, rgba(255,255,255,0.13), rgba(255,255,255,0.05) 52%, rgba(255,255,255,0.09))`,
`blur(30px) saturate(170%)`, 1px `rgba(255,255,255,0.22)` rim, inset top highlight 0.30, drop shadow
`0 12px 30px rgba(0,0,0,0.45)`; broom icon 22px (inverted PNG) + label 14.5px/300. A diagonal
specular band (58% wide, rotated 38°, blurred 2px) sweeps corner to corner once every 20s
(`menlo-glint`). Hover lifts 1px and brightens the rim to 0.42. Hovering also reveals a 280px
glass tooltip above it explaining that nothing is renamed or deleted.

---

## Screens

### 1. Home
Centred column: greeting "Good evening, {first name}." (from Settings), a one-line summary
("N new files have gathered since yesterday. Menlo can put them away in about a minute."), a
three-cell glass stat bar (Files · Source folders · Last scan, each 160px wide, value 26px /
label 10.5px uppercase), then "SOURCE FOLDERS" and a wrapping row of folder-set pills.

Each pill: 44px tall, 14px radius, glass; a stacked folder glyph (1–3 mini folders that fan out
from 7px to 15px spacing and rotate ±4° on hover), the set name, a count ("3 folders"), a ✎ edit
button, and an ✦ badge for AI-built sets. Hovering a multi-folder set drops a glass list of its
folders below it (animated max-height + fade). A dashed "More folder sets (N more)" pill opens the
Folder sets sheet. Seven file-type glyphs (PDF, image, film, zip, audio, code, spreadsheet) drift
behind the content at 24–48% opacity.

### 2. Flow (four stages on a vertical track)
The four stages are 100%-height panes stacked in a `400%` tall column that translates
`-25% × step` over 900ms. Each stage has an eyebrow naming it.

**Stage 1 — CHOOSE · "Clear what's piled up."**
Two columns (`1fr 1fr`, 26px gap): FROM and TO. Each column has a search field (34px, 11px radius)
and a scrollable glass list. Rows: folder glyph, name, meta (file count for sources, existing
subfolder for destinations), and a 16px selection dot that scales 0.82→1 and fills `#F7F8FA` with a
✓ when picked. Empty search shows "No folder by that name". Footer CTA: **Continue**.

**Stage 2 — SHAPE · "Pick the folders N files land in."**
A two-column grid of destination cards. Each card: the folder name (clickable, with a `›` when it
has subfolders), a subfolder count ("3 subfolders"), and its rules as removable tags plus a dashed
"Add rule" chip. **No file counts here** — which file goes where is deliberately not revealed until
the move. A "Suggest folders" button sits top-right. Footer note: "Menlo decides which file goes
where when it moves. You will see every landing in the next stage." Footer CTA: **Move N**.

*Folder drill-down*: clicking a folder replaces the grid with the same layout one level deeper —
a ← button next to the eyebrow, the full path as the headline (`~/Documents`), the folder's own
rules as tags under it, and one card per subfolder (name + its own rules). Each subfolder is seeded
with a default rule derived from its name ("Anything dated 2026 lands in Documents/Invoices 2026",
"Files that read like contracts land in Documents/Contracts"). The button becomes "Suggest
subfolders"; Back steps out of the folder before leaving the stage.

**Stage 3 — MOVE**
Eyebrow MOVE, tagline "Files are travelling to their folders in one pass.", a 56px `moved / total`
counter and a 220px progress hairline. Below: source names on the left, destination names with live
"3 of 9" counters on the right, and glass file cards flying left→right between them, one per file,
staggered by `moveSpeedMs` (default 110ms, exposed as a tweak), each landing on its destination's
lane (58px lane spacing). Footer shows **Stop**; after stopping, **↩ Restore N files** if anything
moved, otherwise Back.

**Stage 4 — FILES PROCESSED**
"N files filed", "N left in place · reversible any time from Runs", a **Done** button that returns
to home, and a receipt list of `Folder/Subfolder → N files`.

### 3. Sorting popup (between Shape and the move)
440px sheet. Eyebrow SORTING, title "Working out where N files belong". A 128px framed field with
six glass file cards shuffling on `menlo-sort` loops (2.4–3.3s, staggered). Four steps tick off in
sequence at 640ms intervals: reading saved rules → grouping by type and date → matching each file to
a subfolder → checking duplicates and clashes. Then it hands off to the move preview.

### 4. Move preview (confirmation)
620px sheet, capped to the window height. Title "N files have a place", sub-copy explaining nothing
is renamed or deleted. Body is a **scrollable block list** (max-height 352px — must be a block
container, not a flex column, or rows compress instead of scrolling): one expandable row per
destination showing "6 files · 3 subfolders"; expanding reveals its subfolders; expanding a
subfolder reveals the actual filenames and sizes. Chevrons rotate 90°. Footer: Cancel / **Move N**.

### 5. Suggest folders
Scan popup first (420px, same language as Sorting, with a ✕ to cancel): "Reading N files in your
sources" and three steps. Then a 480px picker titled **Select any folder**, sub-copy "Folders
outside the ones you picked, drafted from the files sitting in your sources." Rows are dashed-glyph
folders with a reason ("Your sources hold video files with nowhere to land") and a selection dot;
picking one adds it to the destinations. Inside a folder drill-down the same flow suggests
subfolders for that folder instead.

### 6. Rules memory (full-window page)
Left rail: every destination with its subfolders indented beneath it, each showing its rule count.
Right panel: the folder path, "N rules saved for this folder", the rules as ×-removable tags, an
input ("Add a rule — plain words, as long as you like", Enter to add), and dashed suggestion chips
("Create a subfolder per year inside Documents and file invoices by their issue date"). Rules are
free text of any length and are keyed by folder path (`~/Documents`, `~/Documents/Finance`).

### 7. Runs (full-window page)
Left rail: runs newest first (date + file count). Right panel: "{set} · {when}", "N files moved into
M folders", a **Restore this run** button, per-destination rows each with their own **Restore**, and
an alert line when a restore is partial: "2 of 18 files were renamed or moved on since this run, so
they stayed put. The other 16 went back to their source folders." Restored rows dim and their button
reads "Restored". Completed live runs are prepended to the list.

### 8. Settings (full-window page)
Left rail: Profile · Working model · Running · About (scrolls the right panel).
- **Profile name** — text field; drives the home greeting.
- **Working model** — three selectable cards: *Gemma + CLI Agent* ("Gemma reads the folder and writes the plan; the CLI agent carries out the moves."), *CLI Agent* ("Runs through Codex CLI, Claude Code CLI, or any other local CLI agent harness."), *Fully Local* ("Works by calling the normal terminal — plain text syntax matching and ordinary logic, no model.").
- **Running** — segmented controls for Schedule (Manually / Daily, 9pm / On login) and Duplicates (Keep both / Skip / Replace older); toggles for Ask before moving, Include hidden files, Notify when a run finishes, Keep run history (42×24 track, 18px knob, 280ms ease).
- **About** — "Menlo 1.0 — Created by Mayank Sagar…".

### 9. How it works (full-window page)
Left rail of five numbered sections; right panel scrolls. Sections: 01 Scan, 02 Choose, 03 Shape,
04 Move, 05 Files processed. Each has a body paragraph and a small chip flow with arrows
(e.g. Downloads → Desktop → 35 new files). The inactive sections sit at 55% opacity.

### 10. Folder sets sheet and set editor
**Folder sets** (460px): every saved set with its source list, an ✦ badge for AI sets, a ✎ edit
button, and a pin toggle ("On home" / "Add" / "Home full" at 9 pinned). A dashed "Create folder set"
row makes a new set and opens the editor. Footer: "N sets · M of 9 on home" + Done.
**Editor** (420px): editable set-name field, source-folder search, a checkable folder list, and a
footer with the folder count, a **Delete set** action, **← All sets**, and **Done**.

---

## Interactions & behaviour
- Home CTA and any folder-set pill enter the flow; "Custom" starts on stage 1 with nothing preset.
- Stage track moves with a 900ms `cubic-bezier(.32,.72,.16,1)` translate; progress dots are clickable.
- Back: steps out of a folder drill-down → Shape returns to Choose → Choose returns home (home uses the fade-in entrance, never the track scroll). Back is hidden while files are moving.
- Move: Move N → sorting popup (~3s) → move preview → Move N starts the pass. Cancel at any point.
- Stop halts the counter and freezes the in-flight cards at what has already landed.
- Restore (after stop) returns to Shape with the count cleared; Restore in Runs is per run or per destination.
- Rules can be added from the Shape card, the folder drill-down, the rule popup, or the Rules memory page — all write to the same store.
- Every list row, chip and toggle has a hover state (background lifts to `rgba(255,255,255,0.08–0.18)`).

## State
`home`, `step (0–3)`, `preset`, `pinned[]`, `userSets[]`, `deleted[]`, `presetSrc{}`, `presetName{}`,
`src{}`, `dst{}`, `subs{}`, `mode{}`, `prompts{}` (folder path → string[]), `shapeFolder`,
`thinking/thinkStep`, `suggesting/suggestStep/suggestOpen`, `confirmOpen`, `expDest{}/expSub{}`,
`moved`, `stopped`, `runs[]`, `restored{}`, `runActive`, `rulesActive`, `profileName`,
`workingModel`, `schedule`, `dupes`, `flags{}`, plus open/closed flags for each page and sheet.

Real implementation will need: a folder scanner, a rules engine (plain-language rules resolved by
the selected working model), a move executor with per-file progress events, and a run log durable
enough to support restore.

## Assets
- `uploads/imgi_1_broom-icon-8385327-512.png` — broom glyph on the CTA, rendered with `filter: invert(1)`. Replace with the codebase's icon set.
- All other glyphs (folders, file types, chevrons, checkmarks) are CSS shapes and text characters — reimplement with the app's icon library.
- Font: Instrument Sans via Google Fonts.

## Files
- `Menlo Glass Flow v3.dc.html` — the complete prototype (all screens, pages and states).
