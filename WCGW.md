# WCGW — what could go wrong

Domain hazards, not code hazards. Numbered for stable reference from tests and notes —
append only, never renumber. [d] = decided, [o] = open.

## Money

1. Money is never f32/f64 — 0.1 has no exact binary form; i64 scaled by 10_000 is the only representation. [d]
2. More than 4 decimal places is fine IFF every digit past the 4th is zero (1.50000 = 1.5, partner padding); any nonzero digit past the 4th is unrepresentable — reject the row, never round. [d]
3. A huge digit string overflows i64 before the number is finished — parse bails on the first digit that can't fit. [d]
4. Signed amounts (-5.0, +1.0) are malformed; a negative deposit is a withdrawal in costume. [d]
5. Zero-amount rows are legal noise — accept, record the tx. But DISPUTING a zero tx is rejected (BadAmount): holding nothing is nonsense, and a chargeback of nothing would lock an account over 0.0000. [d]
6. Absurd sequences overflow i64 mid-run (deposit 9e14 twice) — checked math makes it a RowError, never wraparound. [d]
7. Disputing funds already spent pushes available negative — the liability stays visible and total = available + held still holds. [d]
8. A chargeback REVERSES its transaction: on a deposit, held and total decrease (funds vanish); on a withdrawal, held decreases and available/total increase (funds clawed back). Same math both ways hands a fraudster free money — 100 deposit, 30 withdraw, dispute, chargeback must end at total 100, not 40. [d]
9. Output can legitimately show negative available after 7 — not a bug, not clamped. Total never goes negative from a chargeback (held was bounded by the disputed amount), but stacked disputes on spent funds can push it there. [d]
10. Display math on scaled i64 uses div_euclid/rem_euclid — sign extraction via abs() trips on edge values, and integer division is lint-denied anyway. [d]

## Input

11. Header may have spaces ("type, client, tx, amount") — fields are matched trimmed, never raw. [d]
12. No header, no file — a non-empty input whose first row isn't a valid header is FatalError::Header, exit 1. An EMPTY file is zero clients, not a broken one: bare header out, exit 0. Headers are MANDATORY for data and DRIVE column mapping (see 46). [d]
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
28. Dispute/resolve/chargeback must carry an EMPTY amount field — the tx record is the only source of truth for the amount. A row that states one (chargeback of 1000 on a 100 deposit) is a lie on its face: Malformed, rejected, the genuine dispute untouched. [d]
29. Type strings are case-insensitive — "Deposit" is a deposit, same partner-sloppiness class as header casing (46). [d]
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
46. The header names the columns and is honored in ANY order, case-insensitively — TYPE,tx,Client,Amount is the same header. Unknown, duplicate, or missing columns are FatalError::Header. A positional parser would silently misbook funds on reordered partners; this is the fraud-adjacent sloppiness the engine exists to catch. [d]
47. Withdrawals are NOT disputable — only deposits are. A dispute/resolve/chargeback naming a withdrawal tx is rejected with a dedicated RowError (WithdrawalDispute), no state change. Chargeback of a withdrawal is meaningless to the bank: the funds already left. [d]
48. Input is real RFC-4180 CSV — a field wrapped in double quotes may contain commas, escaped quotes (""), and newlines. A quoted field with an embedded newline makes ONE record span several physical lines. [d]
49. Line numbers in reports stay PHYSICAL. A record spanning lines N..M reports its span as "line N-M: <error>"; a single-line record reports "line N: <error>". [d]
50. A quote opened but never closed by end-of-file is a broken file — FatalError, nonzero exit, no partial output. The record is unterminated; guessing where it ends is a lie. [d]
51. Render-time total (available+held) overflow is NOT fatal — a stacked-dispute account can legitimately overflow i64 in total while available and held each fit. Such a row reports its total as the literal string "overflow" and processing continues, exit 0. A run dying at render over one hot account is #6 escaping as #2/#3. [d]
52. A quote is structural ONLY when it opens a field; it must then wrap the WHOLE field. `"3"` is a quoted 3, but `, "3"` (space first) and `"4" ,` (trailing junk after the close) are broken records — Malformed, not a lenient re-read as the literal text `"3"`. Inside a quoted field `""` is one literal quote, so `"a""b"` is the 3-byte field `a"b` and is rejected on its own merits (invalid amount), never as malformed. One rule for the whole file beats a per-field guess: a parser that quietly strips stray quotes is guessing at a partner's intent in exactly the place #46 says never to guess. [d]
53. Cap-recovery is quote-aware: when a record blows the size cap while a quoted field is still OPEN, the drain must skip to the end of the RECORD (tracking quote state), not to the next newline. Resyncing at a newline inside the quote restarts the parser mid-field, so the field's own closing quote is read as an OPENING one — and a file with perfectly balanced quotes dies as #50 UnterminatedQuote, taking every later row with it. The cap is a row-level reject (#40); it must never escalate into a file-level fatal. [d]
