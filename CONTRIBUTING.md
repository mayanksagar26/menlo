# Contributing

Thanks for looking. Menlo is small and opinionated, and a few of its constraints are
load-bearing rather than stylistic.

## Getting set up

```bash
git clone https://github.com/mayanksagar26/menlo.git
cd menlo
npm install
npm run tauri dev
```

You need Rust (stable) and Node 22+. On macOS the Xcode Command Line Tools are enough —
you do not need full Xcode.

## Before you open a PR

```bash
cargo fmt   --manifest-path src-tauri/Cargo.toml --all
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test  --manifest-path src-tauri/Cargo.toml
npm run typecheck && npm test
npm run test:e2e
```

CI runs all of the above, a check that no HTTP client is in the Rust core, and a full
`tauri build`.

## Things that will get a PR turned down

These are not preferences; changing them changes what the app is.

1. **Giving a model filesystem access.** A model is a pure classifier: text in, text
   out. If a change would let a model's output become a path or a syscall, it is out of
   scope no matter how convenient.
2. **Network calls from the Rust core.** There is a CI check for this. Menlo's promise
   is that nothing leaves the device.
3. **Moving a file without an explicit approval.** Nothing moves until the user
   presses Move, and the preview is on by default.
4. **Deleting anything that is not a byte-identical duplicate,** or deleting anything
   outside the Trash and the journal.
5. **A batch that cannot be reverted.** Every move is journalled and hash-verified
   before it happens.
6. **Weakening a §7 safety rail** without a test showing why the new behaviour is safe.

## Dependencies

Keep the tree small, and say why in the PR body. The Rust core in particular should stay
close to what it has: `walkdir`, `sha2`, `xattr`, `trash`, `serde`, `chrono`, `uuid`,
`mime_guess`.

## Rule packs

The easiest useful contribution. A pack is a JSON file of rules for a particular kind of
person — photographer, student, freelancer, developer. See `/packs` (Phase 4).

## Style

- Conventional commits.
- Comments explain *why*, not *what*. If a line needs a comment to say what it does,
  rewrite the line.
- Every §7 safety rail gets a test that proves it refuses the thing it claims to refuse.
