# Architecture

## The one decision everything else follows from

The obvious way to build this is to point an agentic CLI at `~/Downloads` and let it
move files. Menlo does not do that. It is slow, non-deterministic, unauditable, and one
bad tool call from destroying a folder.

Instead:

```
scan folder  →  MANIFEST (json)  →  CLI returns PLAN (json)  →  validate
                                                                   ↓
        journal  ←  Rust executes moves  ←  user approves  ←  plan review
```

**The LLM never touches the filesystem.** It is a pure classifier: text in, text out, no
tools, no shell, no write access. Everything that mutates a byte on disk lives in
`execute.rs`, behind `safety.rs`.

## Routing

Most of a Downloads folder is trivially classifiable, and sending all of it to a model
is slow and wasteful. So a file is routed in four steps, and only what is left after
them would ever reach a model (`rules::route`):

1. **Exclude.** A folder's "Skip …" rule the file matches, or an "Only …" rule it
   fails, takes that folder and its subfolders out of the running.
2. **Named rules.** A plain-language rule that matches the file's *name or type* routes
   it directly, at any depth — so "lease or agreement" sends a PDF to
   Documents/Contracts although the extension map says Documents.
3. **Matchers.** User matchers, then learned rules, then the built-in extension map. The
   built-in map only fires for destinations whose key is a known category
   (`documents`, `images`, …) — which is why first-run seeding uses those keys.
4. **Refine.** From a top-level folder, a file may drop one level into a subfolder: by
   that subfolder's rule, or — if nobody wrote one — by carrying every word of its name.

A rule with only a year, size or age can act in step 4 alone. That is what stops
"Anything dated 2026" on Documents/Invoices 2026 from pulling in every 2026 photo.

### Plain-language rules

Rules are sentences the user wrote about a folder, stored as written (`prose.rs`).
Without a model, Menlo reads what ordinary matching can honour — words in the name,
file types, a year, a size, an age, "screenshot" — and reports honestly when a sentence
has none of those and needs a model. It never half-applies one. The UI shows the
reading next to every rule.

**Learned** rules remain the product loop: an approved model decision is crystallised
into a deterministic rule, so the app gets faster on every run.

## Modules

| file | responsibility |
|---|---|
| `scan.rs` | Folders → one `Manifest`. Owns the `ScanItem`/`ManifestEntry` split. |
| `rules.rs` | Routing. Matchers, the built-in extension map, rule persistence, learning. |
| `prose.rs` | Plain-language rules and their no-model reading. |
| `dupes.rs` | Duplicates by name, size and SHA-256. Reads, never writes. |
| `rename.rs` | Clearer names for tool-named files, only when the new name says more. |
| `view.rs` | The shapes the window reads: app state, runs, folder suggestions. |
| `plan.rs` | The §4.2 contract, fence-stripping, and §4.3 validation. |
| `safety.rs` | The §7 rails. Every rule has a test. |
| `execute.rs` | The only code that mutates the filesystem. |
| `journal.rs` | Append-only JSONL, hash-verified revert. |
| `config.rs` | Persisted config, atomic writes, the app directory. |
| `error.rs` | Hand-rolled error type; serialises to a typed shape for the UI. |
| `lib.rs` | Tauri commands. None accepts a path for a plan's destination; the two that take a path at all are the user adding a folder through the picker. |

## Two type-level guarantees

Both of these are enforced by the shape of the types rather than by remembering to check:

**Paths cannot leak into a prompt.** `ManifestEntry` is exactly the §4.1 contract and has
no path field. The real path lives on `ScanItem`, which is never serialised as a whole.
To send a path to a CLI you would have to add a field, not merely forget a filter.

**Paths cannot arrive from a CLI.** A plan carries a `destination_key`, and the app maps
key → path from the user's own config. There is no string a model can emit that becomes
a filesystem path.

## State that crosses the IPC boundary

The frontend never sees a real path for a scanned file. `AppState` holds the current
`Scan` in Rust; the review screen refers to files by manifest id alone. When the user
approves, the edited plan comes back and is re-validated against the stored scan with
the same §4.3 rules applied to the CLI — the frontend is untrusted input too.

## Execution

- Same volume → `rename(2)`, one atomic call.
- Across volumes → copy, verify SHA-256, then delete the source. The source is removed
  only after the copy is proven byte-identical, so an interrupted copy costs disk space,
  never data.
- Collisions never overwrite: ` (2)`, ` (3)`, … before the extension.
- **Duplicates.** A file byte-identical to one already in its destination can be kept
  where it is or sent to the Trash; a file that only shares a *name* is moved alongside
  and never offered for deletion. The frontend's duplicate findings are discarded and
  recomputed on apply, and both copies are re-hashed immediately before a Trash move.
  A Trash move is journalled as a move (`Strategy::Trash`), so revert brings it back
  with no special case.
- **Stop** is checked between files; the file in flight finishes and is journalled.
- One file failing does not abort the batch. This is distinct from §4.3, where a
  malformed *plan* is rejected wholesale before anything moves.
- Every completed move is journalled and fsynced before the next begins, so an
  interrupted run is still fully revertible.

## Revert

Each batch is one append-only JSONL file. Reverts append rather than rewrite, so the
file is a complete audit trail and a crash mid-revert leaves a readable record.

Before restoring a file, its SHA-256 is compared against what was recorded at move time.
A file edited since it was filed is **reported and left alone**, never clobbered — the
newer work wins over the undo.

## What is deliberately absent

- **No network code in the Rust core.** CI greps `src-tauri/Cargo.toml` for HTTP clients
  and fails the build. Nothing leaves the device.
- **No recursion into subfolders.** A user who has already organised something into a
  folder does not want it re-organised. Subfolders are *destinations* — a destination
  with a `parent` key, one level deep — never scanned.
- **No socket for a model.** When Gemma arrives it is reached through the `ollama` CLI,
  not an HTTP client, so the check above keeps holding.
- **No Windows or Linux branches.** macOS only for v1; speculative cross-platform code
  rots.
