# fuzz

Dev-only, nightly-only. `libfuzzer-sys` and `arbitrary` live here and in no
other manifest — the shipped binary stays std-only.

The oracle and the allocation proof are NOT here: they run in `cargo test`
(`tests/oracle.rs`, `tests/allocs.rs`) so they gate every change. These targets
are the coverage-guided pass on top of that, for the byte-level input space
proptest does not reach.

    cargo install cargo-fuzz
    cargo +nightly fuzz run pipeline -j 6 -- -max_total_time=900 -max_len=16384

Seed the corpus from the test fixtures before a long run — starting from valid
CSV beats making the fuzzer rediscover the header from noise:

    mkdir -p fuzz/corpus/pipeline
    cp tests/fixtures/*.csv fuzz/corpus/pipeline/

## targets

`pipeline` — arbitrary bytes through `run()`, asserting on the rendered CSV:
UTF-8, exact header, 5 columns, money at 4dp, `locked` a real bool,
`total == available + held`, client ids strictly ascending (#39), no stdout on
a fatal run (#35), and never `FatalError::Io` from an in-memory reader.

`oracle` — typed op sequences via `arbitrary`, rendered to CSV, compared
against an independent model. Coverage feedback learns to reuse tx ids, which
is what makes the dispute/resolve/chargeback state machine reachable at all.

`money` — round trip: anything `Money::parse` accepts must `Display` back to a
string that reparses to the same scaled value, and display must be stable.

## a crash

Artifacts land in `fuzz/artifacts/<target>/`. The corpus and artifacts are
gitignored, so a real find gets `git add -f`'d into `tests/fixtures/` as a
regression case with a WCGW number, not left sitting in the artifact dir.

    cargo +nightly fuzz run pipeline fuzz/artifacts/pipeline/crash-<hash>
