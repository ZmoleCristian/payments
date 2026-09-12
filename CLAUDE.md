# Payments — toy payments engine

Read TASK.md for the spec. `cargo run -- transactions.csv > accounts.csv`.

## Verification — three gates, all must exit 0

1. `cargo fmt --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `lawkeeper`

The PostToolUse hook judges every edited `.rs` file automatically — a violation blocks the
edit, fix it immediately. Full constitution: `lawkeeper --rules`.

On top of the law: rustfmt.toml (max_width 220, tall params), clippy.toml (f32/f64 are
disallowed types — money is fixed point), and Cargo.toml `[lints]` deny unsafe_code, unwrap,
expect, panic, todo, unimplemented, dbg_macro, float arithmetic, integer division, `as`
casts, and indexing/slicing (walk iterators/bytes, never `buf[i]` or `line[..n]`).
Release profile keeps overflow-checks on.

## The law, briefly (28 rules, syntax-enforced)

- **DECOMPOSITION** — ≤800 LOC/file, ≤80 LOC/fn. lib.rs/mod.rs hold only `mod`/`use`/attrs.
- **LAYER** — imports follow the `[layers]` DAG in law.toml. No inline `crate::`/`self::`/`super::`
  paths outside `use`. No callable values in `static`s.
- **ERROR** — no unwrap/expect/ok/unwrap_or/if-let-on-Option/let-else-on-Result. Only `?`,
  `match` with bound arms, `.ok_or(...)?`. Err arms must propagate, crash, or recover through a
  blessed trace symbol. Never fabricate values from an Err arm. No fallible work in `fn drop`.
  No `.flatten()`/`.filter_map()` on fallibles; no `for` over Result/Option iterators.
- **TYPE** — named error enums only (no String/Box<dyn>/anyhow). No `Result<T>` aliases, spell
  the error at every signature. No `Option` in pub fn args/returns. No `as` casts —
  `u32::try_from` etc. No wrapping_/saturating_/lossy — `checked_*` + `.ok_or(...)?`.
- **DEAD** — zero comments in src/ (names and types carry the meaning). No #[allow] on
  dead_code/unused*. No todo!/unimplemented!/unreachable!. No `use x::*`.
- **TRUTH** — defaults (absence resolving to a value) live only in config.rs, the sanctum.
  `std::env`/argv only in law.toml `env_files` (main.rs).
- **LOG** — println!/eprint! only in bin roots. Libraries take `&mut impl Write` sinks.
  No io::stdout()/stderr() handles outside bin roots.
- **TESTS** — no #[test]/#[cfg(test)] under src/. Tests live in tests/ against the public API.

## Hooks (they will block you; work with them)

- **lawkeeper-edit** (PostToolUse Edit/Write) — judges every edited src/*.rs file; exit 2 = fix now.
- **ban-rust-bash** (PreToolUse Bash) — never read/write .rs via shell: no cat/grep/sed on .rs
  targets, no `> file.rs`. Use Read/Grep/Glob/Edit/Write. Symbol questions go to rustgraph MCP.
- **ban-heredoc-write** — no authoring any file via cat/tee + heredoc or shell redirect. Use Write.
- **ban-inplace-edit** — no `sed -i`, no awk writing files. Pipes that only filter stdout are fine.
- **ban-inline-python** — python only as a shebanged .py file, never `python3 -c`.
- **ban-killbyname** — no pkill/killall; `kill <PID>` only.

## Project rules (from the owner, override habit)

- **std only.** No runtime crates. Every dependency needs an argued case and explicit approval.
- **Layout**: `src/structs/` = declarations only (no impls). `src/models/` = impls only.
  Top-level src/*.rs = free fns. `src/errors.rs` = both enums (declarations AND their impls —
  the one exception to the structs/models split).
- **Two error enums**: `RowError` (rejected row → reported, processing continues) and
  `FatalError` (stops the program → nonzero exit).
- **Everything plain `pub`.** No pub(crate), no pub(super).
- **Diagnostics via sink.** Rejected rows are reported as `line <n>: <error>` to a
  `&mut impl Write` that main hands down (stderr in prod, buffer in tests). A write failure on
  that sink is `FatalError::Report` — it propagates and kills the run, never swallowed.
- **Money** is i64 scaled by 10_000. No floats anywhere. All arithmetic via checked ops.
- **Streaming**: read the input line by line into one reused buffer (BufRead::read_line loop,
  not `.lines()` — for_loops_over_fallibles deputy). Keep only the ledger maps in memory.

## Git

- **Never commit unless the owner says "commit". Never push unless the owner says "push".**
  No proactive commits, no "saving progress", no committing on your own initiative ever.
- Initial scaffolding commits may be large. Everything after that: atomic — one logical change
  per commit, nothing unrelated riding along.

## Domain decisions (documented in README too)

- Disputes apply to deposits AND withdrawals. Available may go negative — a dispute on funds
  already spent is a visible liability, chargeback realizes the loss and locks the account.
- Duplicate tx id on deposit/withdrawal → reject row (spec guarantees global uniqueness).
- A locked account rejects every subsequent transaction.
