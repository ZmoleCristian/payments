use crate::config::{MAX_LINE_BYTES, MAX_LINE_BYTES_U64};
use crate::errors::{FatalError, RowError};
use crate::parse::parse_header;
use crate::render::render;
use crate::structs::columns::Columns;
use crate::structs::ledger::Ledger;
use std::io::{BufRead, Read, Write};

pub fn run(mut input: impl BufRead, out: &mut impl Write, report: &mut impl Write) -> Result<(), FatalError> {
    let mut ledger = Ledger::default();
    let mut line_no: u64 = 0;
    let mut columns: Option<Columns> = None;
    loop {
        let mut raw: Vec<u8> = Vec::new();
        let read = {
            let mut capped = input.by_ref().take(MAX_LINE_BYTES_U64 + 1);
            capped.read_until(b'\n', &mut raw)?
        };
        if read == 0 {
            break;
        }
        line_no += 1;
        if raw.len() > MAX_LINE_BYTES {
            drain_line(&mut input)?;
            report_line(report, line_no, RowError::Malformed)?;
            continue;
        }
        let text = match String::from_utf8(raw) {
            Ok(text) => text,
            Err(bad) => {
                report_bad_bytes(report, line_no, &bad)?;
                continue;
            }
        };
        let line = text.trim_end_matches(['\n', '\r']);
        if line.trim().is_empty() {
            continue;
        }
        let active = match columns {
            Some(ref cols) => cols,
            None => {
                columns = Some(parse_header(line).map_err(bad_header)?);
                continue;
            }
        };
        match active.parse_row(line) {
            Ok(row) => match ledger.apply(&row) {
                Ok(()) => {}
                Err(e) => report_line(report, line_no, e)?,
            },
            Err(e) => report_line(report, line_no, e)?,
        }
    }
    render(&ledger, out)?;
    Ok(())
}

fn bad_header(bad: RowError) -> FatalError {
    match bad {
        RowError::Malformed => FatalError::Header,
        other => FatalError::Corrupt(other),
    }
}

fn report_bad_bytes(report: &mut impl Write, line_no: u64, bad: &std::string::FromUtf8Error) -> Result<(), FatalError> {
    let valid_up_to = bad.utf8_error().valid_up_to();
    writeln!(report, "line {line_no}: invalid utf-8 at byte {valid_up_to}").map_err(FatalError::Report)
}

fn drain_line(input: &mut impl BufRead) -> Result<(), FatalError> {
    let mut sink: Vec<u8> = Vec::new();
    loop {
        sink.clear();
        let n = input.read_until(b'\n', &mut sink)?;
        if n == 0 {
            return Ok(());
        }
        let Some(last) = sink.last() else {
            return Ok(());
        };
        if *last == b'\n' {
            return Ok(());
        }
    }
}

fn report_line(report: &mut impl Write, line_no: u64, e: RowError) -> Result<(), FatalError> {
    writeln!(report, "line {line_no}: {e}").map_err(FatalError::Report)
}
