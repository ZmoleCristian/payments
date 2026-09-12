# WCGW — what could go wrong

Domain hazards, not code hazards. Numbered for stable reference from tests and notes —
append only, never renumber. [d] = decided, [o] = open.

## Money

1. Money is never f32/f64 — 0.1 has no exact binary form; i64 scaled by 10_000 is the only representation. [d]
2. More than 4 decimal places (1.00005) is a partner error, not a rounding invitation — reject the row. [d]
3. A huge digit string overflows i64 before the number is finished — parse bails on the first digit that can't fit. [d]
4. Signed amounts (-5.0, +1.0) are malformed; a negative deposit is a withdrawal in costume. [d]
5. Zero-amount rows are legal noise — accept, record the tx (it can still be disputed later). [d]
6. Absurd sequences overflow i64 mid-run (deposit 9e14 twice) — checked math makes it a RowError, never wraparound. [d]
7. Disputing funds already spent pushes available negative — the liability stays visible and total = available + held still holds. [d]
8. A chargeback REVERSES its transaction: on a deposit, held and total decrease (funds vanish); on a withdrawal, held decreases and available/total increase (funds clawed back). Same math both ways hands a fraudster free money — 100 deposit, 30 withdraw, dispute, chargeback must end at total 100, not 40. [d]
9. Output can legitimately show negative available after 7 — not a bug, not clamped. Total never goes negative from a chargeback (held was bounded by the disputed amount), but stacked disputes on spent funds can push it there. [d]
10. Display math on scaled i64 uses div_euclid/rem_euclid — sign extraction via abs() trips on edge values, and integer division is lint-denied anyway. [d]

## Input

11. Header may have spaces ("type, client, tx, amount") — detect by trimmed fields, never by raw line. [d]
12. No header at all — first row is data unless its trimmed type field literally reads "type". [d]
13. CRLF files — strip trailing \r after \n or every last field is poisoned. [d]
14. UTF-8 BOM at file start — strip it or the header never matches. [d]
15. Blank/whitespace-only lines — skipped silently, not errors, no report line. [d]
16. Wrong field count for the row's type (deposit missing amount, dispute with 5 columns) — Malformed. [d]
17. ".5" and "5." are valid decimals (0.5000, 5.0000) — bare digits on one side of the dot is fine. [d]
18. Missing final newline on the last line — read_until returns data then 0; the row still counts. [d]
19. Non-UTF-8 bytes poison exactly one line, not the file — read raw with read_until(b'\n') into a Vec<u8>, then String::from_utf8 + match; a bad line is a Malformed report and the stream continues. read_line would kill a million rows over one byte. [d]
20. client 0 and tx 0 are valid ids — never sentinel, never special-cased. [d]
21. Leading zeros ("007") parse fine as numbers. [d]

## Protocol

22. tx ids are globally unique — a second deposit/withdrawal reusing any seen id is DuplicateTx, rejected. [d]
23. A rejected row leaves no tx record — disputing a failed withdrawal later is UnknownTx. [d]
24. Dispute/resolve/chargeback naming a tx owned by a different client — UnknownTx for this client. [d]
25. Double dispute on one tx — AlreadyDisputed, rejected. [d]
26. Resolve or chargeback on a tx not under dispute — NotDisputed, rejected. [d]
27. Re-dispute after resolve — allowed; resolve returns the tx to plain state and the machine is symmetric. [d]
28. Dispute/resolve/chargeback carrying a non-empty amount field — the field is ignored, spec leaves it blank. [d]
29. Type strings are exact lowercase — "Deposit" is UnknownType. [d]
30. Every row after an account locks is rejected (deposits included), each earning its own report line. [d]
31. A dispute naming a tx that appears later in the file is UnknownTx at that moment — chronology is the file's promise, not ours to repair. [d]

## Process

32. Missing or unreadable input file — FatalError before any processing, nonzero exit, message on stderr. [d]
33. Wrong arg count — usage on stderr, nonzero exit, nothing opened. [d]
34. Broken stdout pipe mid-render — write error becomes FatalError, nonzero exit; no silent truncation. [d]
35. Render builds the whole output in one buffer and writes once — a crash never emits half a CSV. [d]
36. Report-sink (stderr) failure is FatalError::Report and kills the run — a recovery nobody can see is a bug. [d]
37. Row errors never affect the exit code — 0 means the engine ran, not that every row was accepted. [d]

## Scale & Output

38. The transactions map grows O(#rows) and NOTHING is evictable — a dispute can arrive at any time (kept txs), and chargebacked ids stay as burned tombstones so reuse is still DuplicateTx (see 43). [d]
39. HashMap iteration order is random per run — sort clients before render so output is deterministic for diffing (spec permits any order; determinism is free at ≤65536 clients). [d]
40. A line with no newline inside the buffer cap must not grow memory forever — cap (config.rs), drain to the next newline, report Malformed, continue. [d]
41. Output header is always emitted, columns in spec order, locked as lowercase true/false, money at 4dp — spacing looseness is the grader's gift, not a license. [d]
42. TASK.md hints these CSVs may arrive over thousands of concurrent TCP streams — the engine therefore speaks only `impl BufRead`/`impl Write`, never File or TcpStream; the caller owns the transport and concurrency, the engine owns none of it. [d]
43. A chargebacked tx id is burned forever — the record stays as a tombstone (burned=true) so any later deposit/withdrawal reusing the id is DuplicateTx, and any dispute/resolve/chargeback naming it is UnknownTx. Evicting it would reopen the id to reuse exactly where fraud cleanup happened. [d]
44. Any well-formed row (one that parses) names its client — the account is created on sight, so a rejected duplicate deposit or a failed withdrawal still yields a zero row in the output. A malformed row names nobody. [d]
45. Locked is absolute: every later row for the account is AccountLocked, including resolve/chargeback on disputes still open at lock time — their held funds stay held forever, by design. A locked account may thus report held > 0 with no exit path. [d]
