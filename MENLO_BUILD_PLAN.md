# Menlo — Build Plan

> Instructions for Claude Code. Read this file top to bottom before writing any code.
> Execute phases in order. Do not start a phase until the previous phase's acceptance
> criteria pass. Commit at the end of every numbered task.

---

## 0. What we are building

**Menlo** is a local-first macOS app that files the contents of a cluttered folder
(usually `~/Downloads`) into destination folders, using natural-language rules the user
writes per destination. It runs on whatever agentic CLI is already installed on the
machine — Claude Code, Codex, Gemini CLI, opencode — so there is **no API key and nothing
leaves the device**.

Named after Menlo, the compulsively organised office aide from *Recess*. Sibling project
to Third Street Bookmarks. Keep the joke in the repo's soul and out of its documentation:
the README describes a file organiser, not a cartoon.

**One-liner for the repo description:**
`Local-first, AI-powered file organiser for macOS. Runs on the CLI you already have. No API key, no cloud.`

---

## 1. Non-negotiable architecture decisions

These are settled. Do not redesign them.

**1.1 — The LLM never touches the filesystem.**
The obvious implementation is "point the CLI at ~/Downloads and let it move files." We are
not doing that. It is slow, non-deterministic, unauditable, and one bad tool call from
destroying a folder. Instead:

```
scan folder  →  build MANIFEST (json)  →  CLI returns PLAN (json)  →  validate PLAN  →  user approves  →  Rust executes moves  →  journal
```

The CLI is invoked in headless/print mode as a **pure classifier**. It receives text, it
returns text. It gets no tools, no shell, no write access. All filesystem mutation happens
in Rust, behind validation.

**1.2 — Three-tier routing.**
Most of a Downloads folder is trivially classifiable. Sending all of it to an LLM is slow
and wasteful.

1. **Deterministic pass** — extension, glob, regex, and download-origin rules. Should
   resolve 70–80% of a typical folder with zero inference.
2. **Learned rules** — every LLM decision the user *approves* is crystallised into a
   deterministic rule and persisted. The app gets faster and cheaper on every run. This is
   the core product loop; treat it as a first-class feature, not a cache.
3. **LLM pass** — only the unresolved residue, batched into a single CLI invocation.

**1.3 — Dry run is the default and the hero screen.**
Nothing moves without an explicit approval click in v1. The plan-review screen is the most
important surface in the app — design it first, not last.

**1.4 — Every batch is reversible.**
Append-only journal, one-click revert, hash-verified.

**1.5 — macOS only for v1.**
Tauri gives us cross-platform nearly free, but the extended-attribute reads and the visual
polish are Mac-specific. Ship one platform properly. Do not add Windows/Linux branches
"just in case" — they rot.

---

## 2. Stack

| Layer | Choice | Notes |
|---|---|---|
| Shell | Tauri v2 | Small binary, native feel, Rust core |
| Backend | Rust | `walkdir`, `notify`, `sha2`, `serde`, `xattr`, `trash` |
| Frontend | React 19 + TypeScript + Vite | |
| Styling | Tailwind v4, CSS-first tokens | Tokens in §5 |
| Motion | Framer Motion | Respect `prefers-reduced-motion` |
| State | Zustand | Keep it small |
| Tests | `cargo test` + Vitest + Playwright for the plan-review flow | |

Pin exact versions in the lockfiles. Do not add a dependency without noting why in the PR
body.

---

## 3. Repo structure

```
menlo/
├── README.md
├── LICENSE                      # MIT
├── CONTRIBUTING.md
├── .github/workflows/ci.yml     # fmt, clippy, cargo test, vitest, tauri build
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── scan.rs              # folder → Manifest
│   │   ├── extract.rs           # pdf text, exif, id3, kMDItemWhereFroms
│   │   ├── rules.rs             # deterministic + learned rule engine
│   │   ├── harness/
│   │   │   ├── mod.rs           # detection, selection, invocation
│   │   │   ├── adapter.rs       # trait CliAdapter
│   │   │   ├── claude.rs
│   │   │   ├── codex.rs
│   │   │   ├── gemini.rs
│   │   │   ├── opencode.rs
│   │   │   └── ollama.rs        # fully-offline fallback
│   │   ├── plan.rs              # Plan type + validation
│   │   ├── execute.rs           # moves, collisions, cross-volume
│   │   ├── journal.rs           # undo
│   │   └── config.rs
│   └── tauri.conf.json
├── src/
│   ├── screens/{Setup,PlanReview,History,Settings}.tsx
│   ├── components/
│   ├── styles/tokens.css
│   └── store/
└── docs/
    ├── ARCHITECTURE.md
    └── PROMPT_CONTRACT.md       # the exact prompt sent to the CLI
```

---

## 4. Data contracts

These are the spine of the app. Define them in Rust with `serde`, mirror them in
TypeScript, and write the JSON Schema into `docs/PROMPT_CONTRACT.md`.

### 4.1 Manifest entry (app → CLI)

```json
{
  "id": "f_007",
  "name": "Invoice_Aug2026.pdf",
  "ext": "pdf",
  "size_bytes": 184320,
  "created_at": "2026-08-14T10:22:03Z",
  "modified_at": "2026-08-14T10:22:03Z",
  "mime": "application/pdf",
  "origin": "https://billing.example.com/invoices/8821",
  "excerpt": "Tax Invoice — Aug 2026 — GSTIN 29AA..."
}
```

`origin` comes from the macOS extended attribute
`com.apple.metadata:kMDItemWhereFroms` (a binary plist array). **This is the single
strongest classification signal we have and nobody else uses it** — a file that came from
a bank domain is a bank document regardless of its filename. Implement it in Phase 2 and
make it prominent in the README.

`excerpt` is capped at 400 characters. Never send file bytes. Never send full paths — only
the basename, so the prompt cannot leak directory structure.

### 4.2 Plan entry (CLI → app)

```json
{
  "id": "f_007",
  "action": "move",
  "destination_key": "finance",
  "rename_to": null,
  "confidence": 0.93,
  "reason": "Invoice from a billing domain; matches the Finance brief.",
  "suggest_rule": { "match": "glob:Invoice_*.pdf", "destination_key": "finance" }
}
```

`action` ∈ `move` | `skip` | `needs_review`.
`destination_key` must be one of the keys the app supplied — **the CLI never returns a
filesystem path.** The app maps key → path. This makes path injection structurally
impossible.

### 4.3 Validation (`plan.rs`)

Reject the **entire batch** if any of these fail. Never partially apply a malformed plan,
never guess a correction:

- unknown `id`, or duplicate `id`
- `destination_key` not in the supplied set
- `rename_to` containing `/`, `..`, or a leading `.`
- `confidence` outside `[0,1]`
- response is not parseable JSON after stripping markdown fences

On rejection: surface the raw CLI output in a collapsible panel and offer "retry once".
Two failures in a row → fall back to deterministic-only mode for that run and tell the user.

---

## 5. Design system — "Manila"

Light, warm, paper-like. Deliberately not another dark developer tool. Typography and
restraint carry it; there are no gradients and no glassmorphism.

```css
:root {
  --paper:       #F5F0E6;  /* app ground */
  --manila:      #EDE4D3;  /* cards, folder tiles */
  --manila-deep: #E2D5BC;  /* hover, tab backs */
  --ink:         #1C2333;  /* primary text */
  --ink-soft:    #4A5265;  /* secondary text */
  --mute:        #8A8175;  /* metadata, timestamps */
  --oxblood:     #8C3A2B;  /* single accent: primary action, active state */
  --rule:        rgba(28, 35, 51, 0.12);
  --ok:          #4F6F52;
  --warn:        #B07A2B;
}
```

**Type.** Geist Sans for UI (this is the deliberate bridge to Third Street Bookmarks and
mayanksagar.tech — different theme, same hand). **Menlo** for every filename, path,
extension and confidence figure. The app is named after a monospace font; use it with
intent, not decoration.

**Form.** 6px radii. 1px hairline borders in `--rule`, never box-shadows for structure.
Generous whitespace. Manila folder tiles get a small offset tab on the top-left edge —
that one detail carries the entire metaphor, so get it right and then stop.

**Motion.** On apply, each file card travels along a bezier from the source column to its
destination tile: 320ms, `ease-out-quart`, 40ms stagger, max 12 in flight. Under
`prefers-reduced-motion`, cross-fade instead. The animation exists so the user can *audit*
the plan, not to look busy — it must be interruptible.

**Dark mode.** Phase 3, and it is a distinct palette rather than an inversion: asphalt
`#14120F` ground, chalk `#EDE7DB` ink, manila-amber `#D9A441` accent. Call it "Lights Out".

---

## 6. Phases

### Phase 1 — Skeleton that actually moves files (no AI)

1. Tauri v2 + React + TS scaffold. CI green on `main`.
2. `scan.rs`: folder → `Vec<ManifestEntry>` (name, ext, size, dates, mime). Ignore
   dotfiles, `.DS_Store`, `.crdownload`/`.part`, symlinks, and directories in v1.
3. Setup screen: pick one source folder, add N destinations, each with `{ key, label,
   path, brief }` where `brief` is the natural-language rule textarea.
4. `rules.rs` deterministic engine: extension map + user globs. Produces a Plan.
5. Plan-review screen with the manila design system. Per-row: file, proposed destination
   (editable dropdown), reason, confidence, include/exclude toggle.
6. `execute.rs`: `rename()` on same volume; copy → verify sha256 → delete across volumes.
   Collision policy: never overwrite, append ` (2)` before the extension.
7. `journal.rs`: JSONL per batch at
   `~/Library/Application Support/menlo/journal/<batch_id>.jsonl`. History screen with
   one-click revert that verifies hashes before restoring.

**Acceptance:** organise a 200-file synthetic Downloads folder by extension, review, apply,
and fully revert with zero file loss. Write an integration test that asserts this over a
tempdir.

### Phase 2 — The harness

8. `harness/mod.rs`: probe `PATH` for `claude`, `codex`, `gemini`, `opencode`, `ollama`.
   For each found binary, run its `--help` / `--version` **at runtime** and record which
   headless flags it actually supports. Do not hardcode my assumptions below — verify them
   and store a capability record. These CLIs change often; the probe is the contract.

   Starting points to verify:
   - `claude -p "<prompt>" --output-format json` → parse `.result`
   - `codex exec "<prompt>"`
   - `gemini -p "<prompt>"`
   - `opencode run "<prompt>"`
   - `ollama run <model> "<prompt>"`

   Invoke with a 60s timeout, no inherited TTY, `cwd` set to a temp dir, and an explicitly
   empty tool allowlist wherever the CLI supports one. Stream stdin, never interpolate the
   manifest into a shell string.

9. Settings: show detected CLIs with version and a "Test" button; let the user pick the
   default. If none found, the app still works in deterministic-only mode and says so
   plainly — it must never be dead on arrival.
10. Prompt builder (`docs/PROMPT_CONTRACT.md`): system framing + destination briefs +
    manifest + strict JSON output instruction + one worked example. Version this string;
    log which version produced each plan.
11. `extract.rs`: PDF first-page text, image EXIF, audio ID3, and `kMDItemWhereFroms`.
12. Learned rules: on approval, persist `suggest_rule` entries the user didn't override.
    Settings screen to view, edit, and delete them. Show a "resolved without AI" counter on
    the plan screen — it makes the compounding value visible.

**Acceptance:** on a folder of 300 mixed real-world files, tier 1+2 resolve ≥70% and the
LLM pass handles the rest in a single invocation under 30 seconds.

### Phase 3 — Ambient

13. Menu-bar mode with a `notify` watcher on the source folder, debounced 30s, sweeping on
    idle.
14. **Quarantine window:** never touch a file younger than 24h (configurable). A file you
    downloaded eight minutes ago is a file you are still using. This single rule prevents
    most of the ways this app could become infuriating.
15. Voice rules: mic button on each destination brief. macOS on-device `SFSpeechRecognizer`
    via a Swift sidecar; `whisper.cpp` as the bundled fallback. No cloud STT, ever — it
    would break the core promise.
16. "Lights Out" dark palette.
17. Duplicate detection by sha256 → offer move-to-Trash (via the `trash` crate, never
    `unlink`).

### Phase 4 — Community

18. **Rule packs:** exportable/importable JSON presets (Photographer, Student, Freelancer,
    Developer). Ship four in `/packs`. This is the contribution surface that makes the repo
    worth starring.
19. Consider extracting `harness/` into its own crate + repo. A clean "detect and drive any
    local agent CLI" library will plausibly outlive and out-star this app, and every future
    project in this line needs it. Do not do this until the interface has survived Phase 3.

---

## 7. Safety rails (hard requirements, test each one)

- Destination allowlist: every resolved path must be inside a user-added destination root.
  Canonicalise and re-check *after* symlink resolution.
- Refuse to operate on `/`, `/System`, `/Library`, `/Applications`, `~/Library`, or any
  path containing `.app`.
- Never move a file open by another process (`lsof` check, or fail the move gracefully).
- Never move `.dmg`, `.pkg`, `.app` unless the user explicitly enables it.
- Free-space check before cross-volume copies.
- Hard cap: 500 files per batch in v1.
- No network calls from the Rust core. Add a CI check that greps for HTTP clients in
  `src-tauri` and fails the build.

---

## 8. Git & GitHub

Run at the end of Phase 1, not before — the first public commit should already build.

```bash
gh auth status                      # verify first; stop and ask if not authed
git init -b main
# .gitignore: /target, node_modules, dist, .DS_Store, /src-tauri/target
git add -A
git commit -m "feat: menlo scaffold — scan, deterministic rules, plan review, undo"
gh repo create menlo --public --source=. --remote=origin --push \
  --description "Local-first, AI-powered file organiser for macOS. Runs on the CLI you already have. No API key, no cloud."
```

Then: add topics `macos`, `tauri`, `rust`, `local-first`, `file-organizer`, `claude-code`,
`cli`; enable Issues and Discussions; push the CI workflow; add MIT `LICENSE`.

Branch per phase (`phase-2-harness`), PR into `main`, squash merge. Conventional commits.

**README must contain,** in this order: a screenshot of the plan-review screen (this is the
product), the one-line pitch, "no API key — it uses the CLI you already have" as its own
callout, a 60-second quickstart, the safety model in five bullets, and the supported-CLI
table. Put the *Recess* note at the very bottom under "Why Menlo?" — one sentence, no
explanation.

---

## 9. Ground rules for you, Claude Code

- Ask before adding any dependency not listed in §2.
- Do not write the LLM integration before Phase 1's acceptance test passes. A file mover
  that cannot revert is not a product, it is a liability.
- If a CLI flag in §8 of the harness does not work as described, **fix the plan file and
  say so** — do not silently work around it.
- Every phase ends with: tests pass, `cargo clippy -- -D warnings` clean, commit, and a
  three-line summary of what changed and what you were unsure about.
