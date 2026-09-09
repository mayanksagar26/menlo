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

## Three-tier routing

Most of a Downloads folder is trivially classifiable, and sending all of it to a model
is slow and wasteful.

1. **Deterministic** (`rules.rs`) — extension map, user globs, download origin. Resolves
   the bulk of a typical folder with zero inference.
2. **Learned** (`rules.rs`) — every LLM decision the user *approves* is crystallised
   into a deterministic rule and persisted. The app gets faster on every run. This is
   the core product loop, not a cache.
3. **LLM** (`harness/`, Phase 2) — only the unresolved residue, batched into a single
   invocation.

Rule precedence is user → learned → built-in, and within a tier the most specific
matcher wins, so `glob:Invoice_*.pdf` beats `ext:pdf` regardless of insertion order.

## Modules

| file | responsibility |
|---|---|
| `scan.rs` | Folder → `Manifest`. Owns the `ScanItem`/`ManifestEntry` split. |
| `rules.rs` | Tiers 1 and 2. Matchers, the built-in extension map, rule persistence, learning. |
| `plan.rs` | The §4.2 contract, fence-stripping, and §4.3 validation. |
| `safety.rs` | The §7 rails. Every rule has a test. |
| `execute.rs` | The only code that mutates the filesystem. |
| `journal.rs` | Append-only JSONL, hash-verified revert. |
| `config.rs` | Persisted config, atomic writes, the app directory. |
| `error.rs` | Hand-rolled error type; serialises to a typed shape for the UI. |
| `lib.rs` | Tauri commands. Notably, none of them accepts a destination path. |

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
  folder does not want it re-organised.
- **No Windows or Linux branches.** macOS only for v1; speculative cross-platform code
  rots.
