use crate::csv::{next_record, Read};
use crate::errors::{FatalError, RowError};
use crate::parse::parse_header;
use crate::render::render;
use crate::structs::columns::Columns;
use crate::structs::ledger::Ledger;
use crate::structs::record::Record;
use std::io::{BufRead, Write};

pub fn run(mut input: impl BufRead, out: &mut impl Write, report: &mut impl Write) -> Result<(), FatalError> {
    let mut ledger = Ledger::default();
    let mut next_line: u64 = 1;
    let mut columns: Option<Columns> = None;
    loop {
        match next_record(&mut input, next_line)? {
            Read::Done => break,
            Read::Malformed { first_line, last_line, error } => {
                next_line = last_line + 1;
                report_span(report, first_line, last_line, error)?;
            }
            Read::Row(record) => {
                next_line = record.last_line + 1;
                handle(&mut ledger, &mut columns, &record, report)?;
            }
        }
    }
    render(&ledger, out)?;
    Ok(())
}

fn handle(ledger: &mut Ledger, columns: &mut Option<Columns>, record: &Record, report: &mut impl Write) -> Result<(), FatalError> {
    if record.fields.len() == 1 && field_blank(&record.fields) {
        return Ok(());
    }
    match columns {
        Some(cols) => apply_row(ledger, cols, record, report),
        None => {
            let cols = parse_header(&record.fields).map_err(bad_header)?;
            *columns = Some(cols);
            Ok(())
        }
    }
}

fn apply_row(ledger: &mut Ledger, cols: &Columns, record: &Record, report: &mut impl Write) -> Result<(), FatalError> {
    let row = match cols.parse_row(&record.fields) {
        Ok(row) => row,
        Err(e) => {
            report_span(report, record.first_line, record.last_line, e)?;
            return Ok(());
        }
    };
    match ledger.apply(&row) {
        Ok(()) => {}
        Err(e) => {
            report_span(report, record.first_line, record.last_line, e)?;
            return Ok(());
        }
    }
    Ok(())
}

fn field_blank(fields: &[Vec<u8>]) -> bool {
    let mut blank = true;
    for bytes in fields {
        for b in bytes {
            if !b.is_ascii_whitespace() {
                blank = false;
            }
        }
    }
    blank
}

fn bad_header(bad: RowError) -> FatalError {
    match bad {
        RowError::Malformed => FatalError::Header,
        other => FatalError::Corrupt(other),
    }
}

fn report_span(report: &mut impl Write, first_line: u64, last_line: u64, e: RowError) -> Result<(), FatalError> {
    if first_line == last_line {
        writeln!(report, "line {first_line}: {e}").map_err(FatalError::Report)
    } else {
        writeln!(report, "line {first_line}-{last_line}: {e}").map_err(FatalError::Report)
    }
}
