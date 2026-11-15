<p align="center">
  <img src="docs/screenshots/icon.png" width="128" height="128" alt="Menlo">
</p>

<h1 align="center">Menlo</h1>

<p align="center">
  <strong>A calmer place for your files.</strong><br>
  Clears Downloads and Desktop into the folders you choose, by rules you write in plain words.<br>
  Local-first. No account, no cloud — nothing leaves your Mac.
</p>

<p align="center">
  <a href="https://github.com/mayanksagar26/menlo/releases/latest"><strong>Download for macOS</strong></a> ·
  <a href="#install">Install</a> ·
  <a href="#how-it-works">How it works</a> ·
  <a href="#writing-rules">Writing rules</a> ·
  <a href="#safety">Safety</a>
</p>

<p align="center">
  <a href="https://github.com/mayanksagar26/menlo/actions/workflows/ci.yml"><img src="https://github.com/mayanksagar26/menlo/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <img src="https://img.shields.io/badge/macOS-11%2B-black" alt="macOS 11+">
  <img src="https://img.shields.io/badge/license-MIT-black" alt="MIT">
</p>

![Home](docs/screenshots/home.png)

Menlo takes the folders things pile up in — Downloads, Desktop, anywhere — and files
what is in them into the folders you actually use. You pick the folders, write rules
like *"anything with the word lease or agreement goes here"*, and Menlo shows you every
landing before a single file moves. Every run can be undone.

It works with **no AI at all**. Rules are read with ordinary matching — names, file
types, dates, sizes — and Menlo tells you plainly when a rule needs a model to apply.
If you already use a coding CLI — Claude Code, Codex, Gemini CLI, opencode or any other —
Menlo can ask it about the files your rules can't place. That is optional, never a
requirement, and a local model (Gemma, through Ollama) is next.

It can also **tidy a folder in place**: point it at Documents → Documents and it files
the loose files into Documents' own subfolders, and suggests new subfolders for the
ones nothing fits yet. Every decision you approve teaches it a rule for next time.

---

## Contents

- [Install](#install) — download, with Claude Code, or from source
- [How it works](#how-it-works)
- [Tidying a folder in place](#tidying-a-folder-in-place)
- [Writing rules](#writing-rules)
- [How Menlo remembers](#how-menlo-remembers)
- [Duplicates and renaming](#duplicates-and-renaming)
- [Safety](#safety)
- [Working models](#working-models)
- [Development](#development)
- [Roadmap](#roadmap)

---

## Install

<p align="center">
  <img src="docs/screenshots/installer.png" width="560" alt="The Menlo installer: drag Menlo to Applications">
</p>

### Download

1. Download **`Menlo_0.2.0_aarch64.dmg`** from the
   [latest release](https://github.com/mayanksagar26/menlo/releases/latest).
2. Open it and drag **Menlo** onto **Applications**.
3. Open Menlo. The first time, macOS will stop it, because this build is signed ad-hoc
   rather than with a paid Apple Developer ID, so macOS does not recognise the developer:
   - **macOS 15 and later:** open **System Settings → Privacy & Security**, scroll to the
     message about Menlo, and click **Open Anyway**.
   - **macOS 14 and earlier:** right-click Menlo in Applications → **Open**, then
     **Open** again.

If macOS instead says the app *"is damaged"*, it has only been quarantined by the
download. Clear the quarantine with:

```bash
xattr -dr com.apple.quarantine /Applications/Menlo.app
```

The release build is for **Apple Silicon**. On an Intel Mac, build from source below.

### Let your coding CLI do it

If you have [Claude Code](https://claude.ai/code) or
[Codex CLI](https://github.com/openai/codex), open a terminal in the folder you want
the project to live in, start the CLI, and paste this:

```text
Clone https://github.com/mayanksagar26/menlo and set it up for me on macOS.

Do all of this:
1. Check I have the prerequisites: macOS 11+, the Xcode Command Line Tools,
   Node.js 22+, and Rust (stable, via rustup). If any are missing, install them
   and tell me what you installed.
2. git clone the repo and cd into it.
3. Run: npm install
4. Build the app: npm run tauri build
5. Copy Menlo.app from src-tauri/target/release/bundle/macos/ into /Applications
6. If macOS refuses to open it, clear the quarantine flag:
   xattr -dr com.apple.quarantine /Applications/Menlo.app
7. Launch it and tell me if it opened.

If any step fails, read the error, fix it, and continue. Report what you did.
```

The CLI asks before running each command, so you see exactly what it installs. The
steps are the same as building from source below. Menlo itself does not need the CLI
once it is installed.

### From source

You need **Rust** (stable), **Node 22+**, and the **Xcode Command Line Tools**
(`xcode-select --install`) — full Xcode is not required.

```bash
git clone https://github.com/mayanksagar26/menlo.git
cd menlo
npm install
npm run tauri build
open src-tauri/target/release/bundle/dmg/Menlo_0.2.0_*.dmg
```

To run it without building an app, use `npm run tauri dev`.

---

## How it works

A run is one pass through four stages. Nothing moves until the third.

### 1 · Choose

Pick the folders to clear and the folders things can land in. **Add folder** on either
side takes any folder on your Mac. On first launch Menlo sets itself up from the folders
you already have — Downloads and Desktop to clear; Documents, Pictures, Movies, Music and
Developer to file into, with their existing subfolders. A folder can be on both sides at
once — that tidies it in place (see [below](#tidying-a-folder-in-place)).

![Choose](docs/screenshots/choose.png)

### 2 · Shape

Give folders rules in plain words, and step into a folder to shape its subfolders. A
subfolder with no rule takes files named like it — `Invoices 2026` catches
`Invoice_Aug2026.pdf`. Each rule shows whether Menlo can apply it on its own
(filled dot) or needs a model (ring).

![Shape](docs/screenshots/shape-drilldown.png)

### 3 · Move

Press **Move** and Menlo works out where everything goes. Files already filed, byte for
byte, are raised first — keep them where they are, or send the extra copy to the Trash.

![Duplicates](docs/screenshots/duplicates.png)

Then the preview: every landing, every new name, before anything happens. When Menlo
thinks a group of files deserves a folder of its own, the preview offers it there — see
[New folders](#new-folders).

![Preview](docs/screenshots/preview.png)

Files travel in one pass, with a counter per folder. **Stop** stops after the file in
flight — what landed stays, and nothing else is touched.

### 4 · Files processed

A receipt of what actually landed, read back from the journal of the run.

![Receipt](docs/screenshots/receipt.png)

### Runs

Every run is kept. Send back a whole run, one folder of it, or just what it put in the
Trash. A file you have edited since is left alone rather than overwritten.

![Runs](docs/screenshots/runs.png)

---

## Tidying a folder in place

Add a folder as both a place to clear and a place to file into — Documents → Documents —
and Menlo sorts the files lying loose in it:

- **Into its own subfolders.** A loose `Contracts lease.pdf` goes into
  Documents/Contracts, by your rules or by name. Subfolders you made in Finder since
  adding the folder are picked up at the start of every run.
- **Or somewhere better.** A photo sitting in Documents can still go to Pictures.
- **Or nowhere.** A file nothing fits stays exactly where it is, marked *"Already in
  Documents; none of its folders fit yet."* A file is never "moved" into the folder it is
  already in.
- Files already inside a subfolder are never touched — only the loose ones.

### New folders

What nothing fits yet is grouped, and each group is offered as a new subfolder in the
preview, with **Create** and **Not now**:

- **A word the names share** — three invoice PDFs → *Documents / Invoice*.
- **A CLI agent's suggestion**, when you use one, for files it thinks belong together.
- **A kind of file out of place** — three or more images loose in Documents →
  *Documents / Images*.

**Create** makes the folder and files the group into it. Nothing is created until you
press it. A folder made from a shared word is also remembered as a rule, so next time
those files go there on their own.

---

## Writing rules

Rules are sentences, stored exactly as you write them and read afresh on every run.
Nothing is tied to particular wording. Without a model, Menlo understands:

| It reads | For example |
|---|---|
| **Words in the name** — after *named*, *called*, *containing*, *with … in the name*, *read like*, *keep … together*, or in quotes | "Anything named invoice" · "Files with receipt in the name" · `"boarding pass"` |
| **File types** | "Put PDFs here" · "Photos" · "Videos" · "Spreadsheets" |
| **Screenshots** | "Screenshots" |
| **A year** | "Photos from 2025" |
| **A size** | "Videos bigger than 1 GB" · "Skip anything under 2 KB" |
| **An age** | "Screenshots older than 30 days" · "Only files created this month" |
| **Only / Skip** | "Only files over 10 MB" turns other files away |

Anything else is marked **Needs a model** and moves nothing — Menlo never guesses. The
honest gap today: the most natural phrasing, like *"Invoices go here"* or *"Tax stuff"*,
has no cue word yet, so it waits for a model. Say *"anything named invoice"* instead,
until that lands (see [Roadmap](#roadmap)).

**Rules memory** lists every folder in your home folder — Documents, Pictures, Movies,
Music, Downloads, Desktop and the rest — with their subfolders inside. Main folders
fold open and closed; folders Menlo already files into start open. Each folder shows the
rules you wrote, what Menlo understood from each, and the rules it has learned. Writing
a rule for a folder Menlo does not use yet adds it. The list is read from disk only while
the page is open, and reading it changes nothing.

![Rules memory](docs/screenshots/rules.png)

How rules combine: a rule about names or types can send a file anywhere — *"lease or
agreement"* reaches Documents/Contracts even though the file is a PDF. A rule with only
a date, size or age can only choose *between the subfolders of the folder a file is
already going to*, so *"anything dated 2026"* on Documents/Invoices 2026 never pulls in
your 2026 holiday photos.

---

## How Menlo remembers

Every run you approve can teach Menlo a pattern rule — `the name matches *invoice*` →
Documents/Invoices — which then files those files with no model and no question. A
learned rule outranks the built-in type map, so a careless one would do real damage
(*"every PDF → Finance"*, learned from one invoice). So each one is checked before it is
kept:

- **Too broad is refused.** A bare common type (`.pdf`, `.jpg`) or a pattern with no
  distinctive word (`IMG_*`) is never learned.
- **Your latest decision wins.** The same pattern approved into a different folder moves
  the old rule rather than keeping two that disagree.
- **No repeats.** A pattern an existing rule already catches is not added.
- **Alike rules merge.** Three or more learned rules for one folder that share a word
  fold into one: `Invoice_Aug*`, `Invoice_Sep*` and `invoice-2026*` become `*invoice*`.
- **Shared words are noticed.** When a CLI agent places two or more files with the same
  distinctive word in one folder and you approve them, that word becomes the rule.
- Only moves you approved as proposed count; anything you leave unticked teaches nothing.

Learned rules appear in Rules memory under **Learned from what you approved**, each with
**Forget**.

---

## Duplicates and renaming

**Duplicates.** A file is a duplicate only if it is byte-for-byte the same (SHA-256) as
one already in its folder, or as another file in the same run. You choose, per file:

- **Keep** — leave it where it is; it is not moved.
- **Move to Trash** — remove the extra copy. It goes to your Trash and into the run's
  journal, so Runs can bring it back.

A file that only *shares a name* with one already there is a different file. It is
moved alongside as `report (2).pdf` and never offered for deletion. Settings sets the
default: ask each time, keep, or trash.

**Renaming.** Names a tool chose — `Screenshot 2026-09-02 at 10.14.33`, `IMG_4821`,
`download (3)` — get a clearer one, but only when it says more than the old name. Filed
into Documents/Tax 2026, a screenshot becomes `Tax 2026 2026-09-02 10.14.33.png`. Filed
into plain Pictures it keeps its name, because "Pictures 2026-09-02" would say less.
A name you chose is never touched. Every rename is shown in the preview, and undoing a
run restores the original name.

---

## Safety

- **Nothing moves without you pressing Move,** and by default you see every landing
  first.
- **Nothing is overwritten.** A name clash gets a ` (2)` suffix.
- **Every run is reversible.** An append-only journal records each move, and a restore
  verifies each file's SHA-256 first. A file you edited since is reported, not clobbered.
- **Only identical copies can be trashed,** and both copies are re-hashed immediately
  before, so a file that changed after the preview is never removed.
- **Cross-volume moves** copy, verify the hash, and only then delete the original.
- **It refuses to touch** `/`, `/System`, `/Library`, `/Applications`, `~/Library`,
  anything inside an app bundle, symlinks, files open in another app, and installers
  unless you opt in. Paths are checked after symlink resolution, and again right before
  each move.
- **No network code.** The Rust core makes no network calls; CI fails the build if an
  HTTP client is ever added to it. A CLI agent you choose talks to its own provider, in
  its own process — Menlo never does.
- **A model can never name a path.** When one is used, it picks a folder *key* from a
  list Menlo supplied; Menlo maps the key to a folder you added. Its whole reply is
  rejected if a single entry fails a check.
- **A CLI agent sees names, never contents.** It is sent the names, types, sizes and dates
  of the files your rules could not place — never file contents, never where they live.
  Claude Code is run with every tool switched off. Any other CLI runs inside a macOS
  sandbox (`sandbox-exec`) that blocks your home folder and other drives, apart from its
  own settings and install folders — so a file named to trick it into opening `~/.ssh`
  gets *Operation not permitted*.

---

## Working models

Chosen in Settings.

| | What decides | Status |
|---|---|---|
| **Fully Local** | Ordinary matching and the rules you write. No model, no network. | **Works today** |
| **Gemma** | A local Gemma reads each file and applies the rules plain matching cannot, through Ollama on this Mac. | Next |
| **CLI Agent** | Claude Code, Codex, Gemini CLI, opencode or any other CLI agent you already have, asked only about files the rules can't place. Claude Code runs with tools off; the others run in a macOS sandbox that keeps them out of your home folder. | Works today, optional |

![Settings](docs/screenshots/settings.png)

### Using a CLI agent

Choose **CLI Agent** in Settings and Menlo lists the CLIs it knows — Claude Code, Codex
CLI, Gemini CLI, opencode, Qwen Code and Cursor Agent — with where each was found on
this Mac. **Other command** takes any CLI that reads a prompt on standard input and
prints its answer (e.g. `qwen -p`); it is run directly, never through a shell.
**Check it works** sends a tiny test prompt with nothing about your files in it.

On a run, your rules go first. Only what they could not place is sent, in batches, and
the answers are folded into the preview:

- A confident answer is ticked; a guess (below 70% confidence) is shown but left
  unticked, so it stays put unless you tick it.
- The preview says what the CLI did — *"Claude Code placed 12 of the 15 files the rules
  couldn't"* — or why it could not run, in which case the plan is your rules' moves only.
- Menlo finds CLIs installed through Homebrew, npm, bun or `~/.local/bin` even when it is
  opened from the Dock, where macOS hides your terminal's `PATH`.

Gemini CLI, opencode, Qwen Code and Cursor Agent are supported but have so far been
tested less than Claude Code and Codex. If one needs a settings folder the sandbox does
not open, the preview shows its error and the run falls back to your rules.

---

## Development

```bash
npm install
npm run tauri dev          # the app, with hot reload

cargo test  --manifest-path src-tauri/Cargo.toml   # the Rust tests
npm run typecheck && npm test                      # TypeScript + unit tests
npm run test:e2e                                   # the whole flow in a browser
```

Worth knowing:

- **`src-tauri/tests/ipc.rs`** drives the real Tauri commands with the exact JSON the
  window sends, on real files in a sandboxed home.
- **`src-tauri/tests/duplicates.rs`** puts files in the Trash and brings them back,
  checking every file's contents by hash.
- **`e2e/`** runs the real window against a stubbed backend and asserts what it sends.
  `e2e/screens.spec.ts` also captures every screen into `test-results/screens/`.
- **Live CLI tests** are opt-in, because they spend a real CLI's quota. They sort sample
  files, and check the sandbox stops a CLI told to read a decoy in your home folder:
  `MENLO_LIVE_AGENT=codex cargo test --manifest-path src-tauri/Cargo.toml --lib live -- --ignored`
  (or `claude`).
- The app icon and installer artwork are drawn in `design/brand/` and rendered by
  `design/brand/build.sh`. The results are committed, so a normal build needs nothing
  extra.

More in [CONTRIBUTING.md](CONTRIBUTING.md), [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
and [docs/PROMPT_CONTRACT.md](docs/PROMPT_CONTRACT.md). The design this is built from is
in [docs/design/](docs/design/HANDOFF.md).

---

## Roadmap

- **Gemma through Ollama** — for rules plain matching cannot read, and to name
  screenshots after what is in them. Reached through the `ollama` command line, so the
  Rust core stays free of network code.
- **Natural phrasing without a model** — reading *"Invoices go here"* the way it reads
  *"anything named invoice"*.
- **A menu-bar agent** — scheduled runs and a notification when a run finishes. Both are
  in Settings already, switched off until this lands.
- **A decision model trained on your approvals** — Menlo already turns approvals into
  rules; every choice you confirm is also labelled data. A small decision model such as
  [Laya](https://laya.convaiinnovations.com/), fine-tuned on it, could replace the model
  step entirely.
- **A notarised build**, so no right-click is needed on first open.

---

## Licence

MIT © Mayank Sagar

<sub>Named after the compulsively organised office aide in <em>Recess</em>.</sub>
