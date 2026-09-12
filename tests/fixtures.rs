use std::path::PathBuf;
use std::process::Command;

type Io<T> = Result<T, std::io::Error>;

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run_fixture(name: &str) -> Io<Run> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures");
    path.push(name);
    run_path(&path)
}

fn run_path(path: &PathBuf) -> Io<Run> {
    let output = Command::new(binary_path()?).arg(path).output()?;
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8(output.stdout).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let stderr = String::from_utf8(output.stderr).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(Run { code, stdout, stderr })
}

fn binary_path() -> Io<PathBuf> {
    let mut path = std::env::current_exe()?;
    path.pop();
    path.pop();
    path.push("payments");
    Ok(path)
}

impl Run {
    fn ok(&self) -> &Run {
        assert_eq!(self.code, 0, "exit code, stderr: {}", self.stderr);
        self
    }

    fn fatal(&self) -> &Run {
        assert_eq!(self.code, 1, "fatal exit code, stderr: {}", self.stderr);
        assert!(self.stdout.is_empty(), "fatal runs emit no stdout: {}", self.stdout);
        assert!(self.stderr.contains("missing or invalid header"), "fatal names the header: {}", self.stderr);
        self
    }

    fn fatal_unterminated(&self) -> &Run {
        assert_eq!(self.code, 1, "fatal exit code, stderr: {}", self.stderr);
        assert!(self.stdout.is_empty(), "fatal runs emit no stdout: {}", self.stdout);
        assert!(self.stderr.contains("quoted field opened but never closed"), "fatal names the unterminated quote: {}", self.stderr);
        self
    }

    fn row(&self, client: u16, expect: &str) -> &Run {
        let needle = format!("{client},");
        let mut found = "";
        for line in self.stdout.lines() {
            if line.starts_with(&needle) {
                found = line;
            }
        }
        assert!(!found.is_empty(), "no output row for client {client} in:\n{}", self.stdout);
        assert_eq!(found, expect, "client {client}");
        self
    }

    fn no_row(&self, client: u16) -> &Run {
        let needle = format!("{client},");
        assert!(!self.stdout.lines().any(|l| l.starts_with(&needle)), "client {client} must not appear:\n{}", self.stdout);
        self
    }

    fn report(&self, line: u32, fragment: &str) -> &Run {
        let needle = format!("line {line}: {fragment}");
        assert!(self.stderr.contains(&needle), "expected report '{needle}' in:\n{}", self.stderr);
        self
    }

    fn report_span(&self, first: u32, last: u32, fragment: &str) -> &Run {
        let needle = format!("line {first}-{last}: {fragment}");
        assert!(self.stderr.contains(&needle), "expected report '{needle}' in:\n{}", self.stderr);
        self
    }

    fn no_report(&self, line: u32) -> &Run {
        let needle = format!("line {line}:");
        assert!(!self.stderr.contains(&needle), "no report expected for '{needle}' in:\n{}", self.stderr);
        self
    }

    fn header_only(&self) -> &Run {
        assert_eq!(self.stdout, "client,available,held,total,locked\n", "stdout: {}", self.stdout);
        self
    }

    fn clean(&self) -> &Run {
        assert!(self.stderr.is_empty(), "no reports expected: {}", self.stderr);
        self
    }
}

#[test]
fn bad_header_is_fatal() {
    run_fixture("bad_header.csv").expect("run").fatal();
}

#[test]
fn bom_header_parses_clean() {
    run_fixture("bom_header.csv").expect("run").ok().clean().row(1, "1,10.0000,0.0000,10.0000,false");
}

#[test]
fn crlf_carriage_returns_stripped() {
    run_fixture("crlf.csv").expect("run").ok().report(3, "unknown transaction type").row(1, "1,12.0000,0.0000,12.0000,false");
}

#[test]
fn cross_client_dispute_is_unknown_tx_but_names_client() {
    run_fixture("cross_client_dispute.csv")
        .expect("run")
        .ok()
        .report(3, "unknown transaction id")
        .row(1, "1,10.0000,0.0000,10.0000,false")
        .row(2, "2,0.0000,0.0000,0.0000,false");
}

#[test]
fn dispute_with_stated_amount_is_malformed() {
    run_fixture("dispute_amount.csv").expect("run").ok().report(3, "malformed row").row(1, "1,10.0000,0.0000,10.0000,false");
}

#[test]
fn quoted_fields_parse_as_real_csv() {
    run_fixture("dispute_with_amount_and_quoted_rows.csv")
        .expect("run")
        .ok()
        .report(3, "malformed row")
        .no_report(5)
        .row(1, "1,10.0000,0.0000,10.0000,false")
        .row(2, "2,1.0000,0.0000,1.0000,false")
        .row(3, "3,2.0000,0.0000,2.0000,false");
}

#[test]
fn quote_must_wrap_the_whole_field_wcgw52() {
    run_fixture("quote_grammar.csv")
        .expect("run")
        .ok()
        .report(3, "invalid amount")
        .report(6, "malformed row")
        .no_report(4)
        .no_report(5)
        .no_row(2)
        .no_row(6)
        .row(1, "1,10.0000,0.0000,10.0000,false")
        .row(3, "3,2.0000,0.0000,2.0000,false")
        .row(4, "4,1.0000,0.0000,1.0000,false")
        .row(5, "5,7.0000,0.0000,7.0000,false");
}

#[test]
fn record_cap_inside_open_quote_drains_whole_record_wcgw40() {
    run_fixture("cap_inside_quote.csv")
        .expect("run")
        .ok()
        .report_span(2, 902, "malformed row")
        .no_row(1)
        .no_row(100)
        .no_row(500)
        .row(7, "7,5.0000,0.0000,5.0000,false");
}

#[test]
fn dispute_on_withdrawal_rejected_wcgw47() {
    run_fixture("dispute_withdrawal.csv")
        .expect("run")
        .ok()
        .report(4, "withdrawals cannot be disputed")
        .row(1, "1,7.0000,0.0000,7.0000,false");
}

#[test]
fn dispute_missing_trailing_comma_is_malformed() {
    run_fixture("dispute_without_trailing_comma.csv")
        .expect("run")
        .ok()
        .report(3, "unknown transaction type")
        .report(4, "malformed row")
        .report(6, "insufficient funds")
        .row(1, "1,0.0000,10.0000,10.0000,false");
}

#[test]
fn duplicate_header_column_is_fatal() {
    run_fixture("dup_column.csv").expect("run").fatal();
}

#[test]
fn duplicate_tx_ids_rejected_globally() {
    run_fixture("duplicate_tx.csv")
        .expect("run")
        .ok()
        .report(3, "duplicate transaction id")
        .report(4, "duplicate transaction id")
        .report(5, "duplicate transaction id")
        .row(1, "1,10.0000,0.0000,10.0000,false")
        .row(2, "2,0.0000,0.0000,0.0000,false");
}

#[test]
fn empty_file_is_zero_clients() {
    run_fixture("empty.csv").expect("run").ok().clean().header_only();
}

#[test]
fn unknown_header_column_is_fatal() {
    run_fixture("extra_column.csv").expect("run").fatal();
}

#[test]
fn blank_line_skipped_ghost_clients_appear_zeroed() {
    run_fixture("ghost_accounts_and_blank_line.csv")
        .expect("run")
        .ok()
        .report(3, "unknown transaction type")
        .report(4, "unknown transaction id")
        .report(5, "insufficient funds")
        .no_report(2)
        .row(7, "7,0.0000,0.0000,0.0000,false")
        .row(8, "8,0.0000,0.0000,0.0000,false");
}

#[test]
fn bad_amount_never_names_a_client() {
    run_fixture("ghost_accounts.csv")
        .expect("run")
        .ok()
        .report(3, "insufficient funds")
        .report(4, "unknown transaction id")
        .report(5, "invalid amount")
        .row(1, "1,10.0000,0.0000,10.0000,false")
        .row(7, "7,0.0000,0.0000,0.0000,false")
        .row(8, "8,0.0000,0.0000,0.0000,false")
        .no_row(9);
}

#[test]
fn grammar_edges_accept_bare_decimals_reject_junk() {
    run_fixture("grammar_edges.csv")
        .expect("run")
        .ok()
        .report(4, "invalid amount")
        .report(5, "invalid amount")
        .report(6, "invalid amount")
        .report(8, "invalid amount")
        .row(1, "1,0.5000,0.0000,0.5000,false")
        .row(2, "2,1.0000,0.0000,1.0000,false")
        .no_row(3)
        .no_row(4)
        .no_row(5)
        .row(6, "6,7.0000,0.0000,7.0000,false")
        .no_row(7);
}

#[test]
fn dispute_held_overflow_rejected_row_survives() {
    run_fixture("half_applied_dispute_i64_14digit.csv")
        .expect("run")
        .ok()
        .report(20, "invalid amount")
        .row(1, "1,0.0000,899999999999999.9991,899999999999999.9991,false");
}

#[test]
fn rejected_dispute_moves_no_money() {
    run_fixture("half_applied_hold_i64.csv")
        .expect("run")
        .ok()
        .report(6, "invalid amount")
        .row(1, "1,-922337203685477.5806,922337203685477.5807,0.0001,false");
}

#[test]
fn header_case_is_irrelevant() {
    run_fixture("header_case.csv").expect("run").ok().clean().row(1, "1,10.0000,0.0000,10.0000,false");
}

#[test]
fn header_only_yields_bare_output() {
    run_fixture("header_only.csv").expect("run").ok().clean().header_only();
}

#[test]
fn padded_header_fields_trim() {
    run_fixture("header_padding.csv").expect("run").ok().clean().row(1, "1,10.0000,0.0000,10.0000,false");
}

#[test]
fn rejected_withdrawal_frees_its_id() {
    run_fixture("id_reuse_after_reject.csv")
        .expect("run")
        .ok()
        .report(2, "insufficient funds")
        .no_report(3)
        .row(1, "1,5.0000,0.0000,5.0000,false");
}

#[test]
fn chargebacked_id_stays_burned_forever() {
    run_fixture("id_reuse_after_withdrawal_and_chargeback.csv")
        .expect("run")
        .ok()
        .report(4, "duplicate transaction id")
        .report(8, "duplicate transaction id")
        .row(1, "1,9.0000,0.0000,9.0000,false")
        .row(3, "3,0.0000,0.0000,0.0000,true")
        .row(4, "4,0.0000,0.0000,0.0000,false");
}

#[test]
fn full_dispute_lifecycle_then_locked() {
    run_fixture("lifecycle.csv")
        .expect("run")
        .ok()
        .report(3, "transaction not under dispute")
        .report(4, "transaction not under dispute")
        .report(6, "transaction already under dispute")
        .report(8, "transaction not under dispute")
        .report(11, "account locked")
        .report(12, "account locked")
        .row(1, "1,0.0000,0.0000,0.0000,true");
}

#[test]
fn locked_account_rejects_everything_after() {
    run_fixture("locked_account.csv")
        .expect("run")
        .ok()
        .report(6, "account locked")
        .report(7, "account locked")
        .report(8, "account locked")
        .row(1, "1,5.0000,0.0000,5.0000,true");
}

#[test]
fn amounts_beyond_i64_max_rejected() {
    run_fixture("max_amount.csv")
        .expect("run")
        .ok()
        .report(3, "invalid amount")
        .report(4, "invalid amount")
        .row(1, "1,922337203685477.5807,0.0000,922337203685477.5807,false")
        .no_row(2)
        .no_row(3);
}

#[test]
fn chargeback_can_drive_total_negative_unclamped() {
    run_fixture("negative_after_dispute.csv").expect("run").ok().clean().row(1, "1,-100.0000,0.0000,-100.0000,true");
}

#[test]
fn headerless_data_is_fatal() {
    run_fixture("no_header.csv").expect("run").fatal();
}

#[test]
fn multiline_quote_span_reports_span_wcgw49() {
    run_fixture("quoted_injection.csv")
        .expect("run")
        .ok()
        .report_span(2, 4, "invalid amount")
        .no_report(2)
        .no_row(1)
        .no_row(2)
        .row(3, "3,7.0000,0.0000,7.0000,false");
}

#[test]
fn redispute_after_resolve_is_legal() {
    run_fixture("redispute.csv").expect("run").ok().clean().row(1, "1,0.0000,10.0000,10.0000,false");
}

#[test]
fn reordered_header_remaps_columns() {
    run_fixture("reordered_header_type_first.csv").expect("run").ok().clean().row(1, "1,10.0000,0.0000,10.0000,false");
}

#[test]
fn spec_sample_overdraft_rejected() {
    run_fixture("spec_sample.csv")
        .expect("run")
        .ok()
        .report(6, "insufficient funds")
        .row(1, "1,1.5000,0.0000,1.5000,false")
        .row(2, "2,2.0000,0.0000,2.0000,false");
}

#[test]
fn huge_amounts_rejected_other_clients_unaffected() {
    run_fixture("sum_overflow_i128.csv")
        .expect("run")
        .ok()
        .report(2, "invalid amount")
        .report(3, "unknown transaction id")
        .report(4, "invalid amount")
        .row(1, "1,0.0000,0.0000,0.0000,false")
        .row(2, "2,5.0000,0.0000,5.0000,false");
}

#[test]
fn stacked_disputes_cannot_exceed_representable_total_wcgw51() {
    run_fixture("sum_overflow_i64_14digit.csv")
        .expect("run")
        .ok()
        .report(28, "invalid amount")
        .row(1, "1,0.0000,899999999999999.9991,899999999999999.9991,false")
        .row(2, "2,5.0000,0.0000,5.0000,false");
}

#[test]
fn dispute_at_i64_max_rejects_unrepresentable_total_wcgw51() {
    run_fixture("sum_overflow_i64.csv")
        .expect("run")
        .ok()
        .report(4, "invalid amount")
        .row(1, "1,0.0000,922337203685477.5807,922337203685477.5807,false")
        .row(2, "2,5.0000,0.0000,5.0000,false");
}

#[test]
fn trailing_zero_padding_accepted_real_precision_rejected() {
    run_fixture("trailing_zeros.csv")
        .expect("run")
        .ok()
        .report(3, "invalid amount")
        .row(1, "1,1.5000,0.0000,1.5000,false")
        .no_row(2)
        .row(3, "3,2.5000,0.0000,2.5000,false");
}

#[test]
fn type_casing_and_padding_tolerated_but_synonyms_rejected() {
    run_fixture("type_case.csv").expect("run").ok().report(4, "unknown transaction type").row(1, "1,12.0000,0.0000,12.0000,false");
}

#[test]
fn unterminated_quote_at_eof_is_broken_file_wcgw50() {
    run_fixture("unterminated_quote.csv").expect("run").fatal_unterminated();
}

#[test]
fn exact_withdrawal_drains_then_overdraw_rejected() {
    run_fixture("withdrawal_exact_and_over.csv")
        .expect("run")
        .ok()
        .report(4, "insufficient funds")
        .row(1, "1,0.0000,0.0000,0.0000,false");
}

#[test]
fn zero_amounts_recorded_but_zero_dispute_rejected() {
    run_fixture("zero_amount.csv")
        .expect("run")
        .ok()
        .report(5, "invalid amount")
        .report(6, "transaction not under dispute")
        .row(1, "1,0.0000,0.0000,0.0000,false")
        .row(2, "2,5.0000,0.0000,5.0000,false");
}
