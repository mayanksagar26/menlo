# Prompt contract

The exact interface between Menlo and whichever agentic CLI is driving it.

> **Status.** The JSON Schemas and the safety rules below are implemented and enforced
> today (`src-tauri/src/scan.rs`, `src-tauri/src/plan.rs`). The prompt template in §4 is
> the Phase 2 target and is not yet wired to a CLI — `harness/` does not exist yet. The
> schemas will not change when it lands; that is the point of writing them down first.

---

## 1. The shape of the exchange

```
scan folder  →  MANIFEST (json)  →  CLI  →  PLAN (json)  →  validate  →  approve  →  move
```

The CLI is invoked in headless/print mode as a **pure classifier**. It receives text and
returns text. It gets no tools, no shell, and no write access. Every filesystem mutation
happens in Rust, behind validation. This is not a defence-in-depth nicety — it is the
reason the app can be trusted with a folder at all.

---

## 2. Manifest entry — app → CLI

One JSON array of these is embedded in the prompt.

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

### Schema

| field | type | required | notes |
|---|---|---|---|
| `id` | string | yes | `f_NNN`, assigned by the scan. Stable for a given folder listing. |
| `name` | string | yes | **Basename only.** Never a path. |
| `ext` | string | yes | Lowercased, no leading dot. Empty string if the file has none. |
| `size_bytes` | integer | yes | |
| `created_at` | string | yes | RFC 3339, UTC. Epoch if the filesystem does not record it. |
| `modified_at` | string | yes | RFC 3339, UTC. |
| `mime` | string | yes | Guessed from the extension. |
| `origin` | string | no | Omitted when absent. See below. |
| `excerpt` | string | no | Capped at 400 characters. Omitted when absent. |

### Hard rules

- **Never send file bytes.** Only the excerpt, and only up to 400 characters.
- **Never send full paths.** The prompt cannot be allowed to leak directory structure.
  This is enforced structurally: `ManifestEntry` has no path field, and the real path
  lives on `ScanItem`, which is never serialised. There is a test asserting no field but
  `mime` contains a `/`.

### `origin`

`origin` is read from the macOS extended attribute
`com.apple.metadata:kMDItemWhereFroms`, a binary plist array whose first element is the
source URL.

This is the single strongest classification signal available, and essentially nothing
else uses it. A file that came from a bank's domain is a bank document regardless of
what it is called — `Untitled (3).pdf` from `hdfcbank.com` is unambiguous to Menlo and
opaque to every filename-based organiser.

---

## 3. Plan entry — CLI → app

The CLI must return a JSON array of these, and nothing else.

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

### Schema

| field | type | required | notes |
|---|---|---|---|
| `id` | string | yes | Must be one of the supplied ids, used exactly once. |
| `action` | enum | yes | `move` \| `skip` \| `needs_review` |
| `destination_key` | string \| null | when `move` | Must be one of the supplied keys. |
| `rename_to` | string \| null | no | Basename only. |
| `confidence` | number | yes | `[0, 1]` inclusive. |
| `reason` | string | yes | One sentence, shown verbatim to the user. |
| `suggest_rule` | object \| null | no | `{ match, destination_key }` |

### `destination_key`, and why there is no path here

**The CLI never returns a filesystem path.** It returns a key from a set the app
supplied, and the app maps that key to a path it already trusts.

This makes path injection structurally impossible rather than merely filtered. There is
no string a model can emit that becomes a path, because the only thing it can influence
is a lookup into a table the user built. A hallucinated `"destination_key": "/etc"`
fails a set-membership check and rejects the batch; it never reaches the filesystem.

### `suggest_rule`

The deterministic rule this decision should crystallise into (§1.2 tier 2). Supported
matcher prefixes:

| prefix | example | meaning |
|---|---|---|
| `ext:` | `ext:pdf` | Extension, case-insensitive. |
| `glob:` | `glob:Invoice_*.pdf` | Shell glob over the basename: `*`, `?`, `[a-z]`, `[!0-9]`. Case-insensitive. |
| `origin:` | `origin:billing.example.com` | Substring of the download origin. |

`regex:` is reserved and currently **rejected** — it needs the `regex` crate, which is
not yet in the approved dependency set. A `regex:` rule fails to parse loudly rather
than silently never matching.

A suggestion is persisted only when the user approves the entry **and** does not
override the destination. An overridden destination means the suggestion was wrong, and
learning it would bake in the mistake.

---

## 4. The prompt template *(Phase 2 — not yet implemented)*

Version this string. Log which version produced each plan; `Plan.prompt_version` exists
for exactly this and is `null` until the harness lands.

```
### menlo/v1

You are a file classifier. You will be given a list of files and a list of
destination folders. For each file, decide which destination it belongs in.

Rules:
- Reply with a JSON array and nothing else. No prose, no markdown fences.
- One object per input file, using the file's `id`. Do not invent ids.
- `destination_key` must be exactly one of the keys listed below. Never write a
  file path.
- If a file does not clearly belong anywhere, use "needs_review" rather than
  guessing. A wrong confident answer is worse than an honest shrug.
- `confidence` is your genuine estimate between 0 and 1.
- `reason` is one short sentence a non-technical person would understand.
- `suggest_rule` is optional: include it only when a simple, durable pattern
  would catch this file and others like it in future.

Destinations:
{{#each destinations}}
- {{key}} — {{label}}: {{brief}}
{{/each}}

Files:
{{manifest_json}}

Example of a well-formed reply:
[
  {
    "id": "f_000",
    "action": "move",
    "destination_key": "finance",
    "rename_to": null,
    "confidence": 0.93,
    "reason": "An invoice from a billing domain, which matches the Finance brief.",
    "suggest_rule": { "match": "glob:Invoice_*.pdf", "destination_key": "finance" }
  }
]
```

The `origin` field deserves explicit mention in the framing when present — it is the
signal most likely to be under-weighted by a model that has been trained to look at
filenames.

---

## 5. Validation — §4.3

Implemented in `plan::validate`. **The entire batch is rejected if any check fails.** A
malformed plan is never partially applied and a correction is never guessed.

| check | rejection reason |
|---|---|
| id not in the manifest | `unknown id` |
| id appears twice | `duplicate id` |
| `move` with no `destination_key` | `action 'move' without a destination_key` |
| `destination_key` not in the supplied set | `destination_key '…' was not supplied` |
| `skip`/`needs_review` carrying a `destination_key` | `non-move action carries a destination_key` |
| `rename_to` contains `/` | `rename_to contains '/'` |
| `rename_to` contains `..` | `rename_to contains '..'` |
| `rename_to` starts with `.` | `rename_to starts with '.'` |
| `rename_to` contains NUL, is empty, or exceeds 255 bytes | as stated |
| `confidence` outside `[0,1]`, or NaN | `confidence … is outside [0,1]` |
| `suggest_rule` targets an unsupplied key | `suggest_rule targets unsupplied destination_key` |
| body does not parse as JSON after fence-stripping | `malformed JSON` |

All problems are collected and reported together, not just the first — a user debugging
a flaky CLI deserves the whole list.

On rejection the UI shows the raw CLI output in a collapsible panel and offers **retry
once**. Two failures in a row fall back to deterministic-only mode for that run and say
so plainly.

### Parsing leniency, and its limits

Menlo is lenient about *packaging* and strict about *content*:

- Markdown fences are stripped, with or without a language tag.
- Surrounding prose is discarded by taking the outermost `[…]` or `{…}` span.
- A top-level object with an `entries`, `plan`, `files`, or `results` array is unwrapped.

It is not lenient about anything in the table above. Being forgiving about how four
different CLIs wrap their output is pragmatism; being forgiving about a destination key
that does not exist would be negligence.

---

## 6. Invocation *(Phase 2 — not yet implemented)*

Starting points to **verify at runtime**, not to hardcode. These CLIs change often, so
the probe is the contract: run `--help`/`--version` for each binary found on `PATH` and
record a capability record.

| CLI | candidate invocation | parse |
|---|---|---|
| `claude` | `claude -p "<prompt>" --output-format json` | `.result` |
| `codex` | `codex exec "<prompt>"` | stdout |
| `gemini` | `gemini -p "<prompt>"` | stdout |
| `opencode` | `opencode run "<prompt>"` | stdout |
| `ollama` | `ollama run <model> "<prompt>"` | stdout |

Every invocation must:

- time out after 60s,
- inherit no TTY,
- run with `cwd` set to an empty temp directory,
- pass an explicitly empty tool allowlist wherever the CLI supports one,
- **stream the prompt over stdin** — the manifest is never interpolated into a shell
  string.
