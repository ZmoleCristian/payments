use crate::config::MAX_RECORD_BYTES;
use crate::errors::{FatalError, RowError};
use crate::structs::record::Record;
use crate::structs::scan::Scan;
use std::io::BufRead;

pub enum Read {
    Row,
    Malformed { first_line: u64, last_line: u64, error: RowError },
    Done,
}

enum Step {
    More,
    Ended,
    Bad,
}

pub fn next_record(input: &mut impl BufRead, record: &mut Record, scan: &mut Scan, first_line: u64) -> Result<Read, FatalError> {
    record.reset(first_line);
    scan.reset(first_line);
    loop {
        let buf = input.fill_buf()?;
        if buf.is_empty() {
            return eof(record, scan, first_line);
        }
        let (consumed, step) = consume(record, scan, buf);
        let len = buf.len();
        input.consume(match step {
            Step::More => len,
            _ => consumed,
        });
        match step {
            Step::Bad => {
                skip_to_eol(input, scan)?;
                return Ok(malformed(first_line, scan.line - 1));
            }
            Step::Ended => {
                if scan.line - 1 != first_line && !scan.quoted_any {
                    scan.bad = true;
                }
                if scan.bad || over_cap(record) {
                    return Ok(malformed(first_line, scan.line - 1));
                }
                record.last_line = scan.line - 1;
                return Ok(Read::Row);
            }
            Step::More => {
                if over_cap(record) {
                    skip_to_eol(input, scan)?;
                    return Ok(malformed(first_line, scan.line - 1));
                }
            }
        }
    }
}

fn eof(record: &mut Record, scan: &mut Scan, first_line: u64) -> Result<Read, FatalError> {
    if scan.in_quotes {
        return Err(FatalError::UnterminatedQuote);
    }
    if !scan.saw_any {
        return Ok(Read::Done);
    }
    if scan.bad {
        return Ok(malformed(first_line, scan.line - 1));
    }
    record.close_field();
    record.last_line = scan.line - 1;
    Ok(Read::Row)
}

fn skip_to_eol(input: &mut impl BufRead, scan: &mut Scan) -> Result<(), FatalError> {
    loop {
        let buf = input.fill_buf()?;
        if buf.is_empty() {
            if scan.in_quotes {
                return Err(FatalError::UnterminatedQuote);
            }
            return Ok(());
        }
        let mut consumed = 0;
        let mut hit = false;
        for &b in buf {
            consumed += 1;
            if scan.in_quotes {
                match b {
                    b'"' => scan.in_quotes = false,
                    b'\n' => scan.line += 1,
                    _ => {}
                }
                continue;
            }
            if b == b'\n' {
                hit = true;
                break;
            }
        }
        input.consume(consumed);
        if hit {
            scan.line += 1;
            return Ok(());
        }
    }
}

fn consume(record: &mut Record, scan: &mut Scan, buf: &[u8]) -> (usize, Step) {
    let mut consumed = 0;
    for &b in buf {
        consumed += 1;
        scan.saw_any = true;
        let step = if scan.in_quotes {
            step_quoted(record, scan, b)
        } else if scan.just_closed {
            step_closed(record, scan, b)
        } else {
            step_bare(record, scan, b)
        };
        match step {
            Step::More => {}
            done => return (consumed, done),
        }
    }
    (consumed, Step::More)
}

fn newline(record: &mut Record, scan: &mut Scan) -> Step {
    if scan.in_quotes {
        record.bytes.push(b'\n');
        scan.line += 1;
        Step::More
    } else {
        record.close_field();
        scan.line += 1;
        Step::Ended
    }
}

fn step_quoted(record: &mut Record, scan: &mut Scan, b: u8) -> Step {
    scan.quoted_any = true;
    match b {
        b'"' => {
            scan.in_quotes = false;
            scan.just_closed = true;
            Step::More
        }
        b'\n' => newline(record, scan),
        _ => {
            record.bytes.push(b);
            Step::More
        }
    }
}

fn step_closed(record: &mut Record, scan: &mut Scan, b: u8) -> Step {
    match b {
        b'"' => {
            record.bytes.push(b'"');
            scan.just_closed = false;
            scan.in_quotes = true;
            Step::More
        }
        b',' => {
            record.close_field();
            scan.just_closed = false;
            scan.field_start = true;
            Step::More
        }
        b'\n' => {
            record.close_field();
            scan.line += 1;
            Step::Ended
        }
        b'\r' | b' ' => Step::More,
        _ => Step::Bad,
    }
}

fn step_bare(record: &mut Record, scan: &mut Scan, b: u8) -> Step {
    match b {
        b'"' if scan.field_start => {
            scan.in_quotes = true;
            scan.field_start = false;
            Step::More
        }
        b'"' => Step::Bad,
        b',' => {
            record.close_field();
            scan.field_start = true;
            Step::More
        }
        b'\n' => newline(record, scan),
        b'\r' => Step::More,
        b' ' if scan.field_start => Step::More,
        _ => {
            record.bytes.push(b);
            scan.field_start = false;
            Step::More
        }
    }
}

fn malformed(first_line: u64, last_line: u64) -> Read {
    Read::Malformed {
        first_line,
        last_line,
        error: RowError::Malformed,
    }
}

fn over_cap(record: &Record) -> bool {
    record.bytes.len() > MAX_RECORD_BYTES
}
