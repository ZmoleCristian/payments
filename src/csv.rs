use crate::config::MAX_RECORD_BYTES;
use crate::errors::{FatalError, RowError};
use crate::structs::record::Record;
use std::io::BufRead;

pub enum Read {
    Row(Record),
    Malformed { first_line: u64, last_line: u64, error: RowError },
    Done,
}

enum Step {
    More,
    Ended,
    Bad,
}

pub fn next_record(input: &mut impl BufRead, first_line: u64) -> Result<Read, FatalError> {
    let mut st = State {
        fields: Vec::new(),
        field: Vec::new(),
        in_quotes: false,
        quoted: false,
        field_start: true,
        just_closed: false,
        saw_any: false,
        bad: false,
        quoted_any: false,
        line: first_line,
    };
    loop {
        let buf = input.fill_buf()?;
        if buf.is_empty() {
            return eof(&mut st, first_line);
        }
        let (consumed, step) = consume(&mut st, buf);
        let len = buf.len();
        input.consume(match step {
            Step::More => len,
            _ => consumed,
        });
        match step {
            Step::Bad => {
                skip_to_eol(input, &mut st)?;
                return Ok(malformed(first_line, st.line - 1));
            }
            Step::Ended => {
                if st.line - 1 != first_line && !st.quoted_any {
                    st.bad = true;
                }
                if st.bad || over_cap(&st) {
                    return Ok(malformed(first_line, st.line - 1));
                }
                return Ok(finish(st, first_line));
            }
            Step::More => {
                if over_cap(&st) {
                    skip_to_eol(input, &mut st)?;
                    return Ok(malformed(first_line, st.line - 1));
                }
            }
        }
    }
}

struct State {
    fields: Vec<Vec<u8>>,
    field: Vec<u8>,
    in_quotes: bool,
    quoted: bool,
    field_start: bool,
    just_closed: bool,
    saw_any: bool,
    bad: bool,
    quoted_any: bool,
    line: u64,
}

fn eof(st: &mut State, first_line: u64) -> Result<Read, FatalError> {
    if st.in_quotes {
        return Err(FatalError::UnterminatedQuote);
    }
    if st.saw_any {
        if st.bad {
            return Ok(malformed(first_line, st.line - 1));
        }
        flush_field(st);
        return Ok(Read::Row(Record {
            fields: std::mem::take(&mut st.fields),
            first_line,
            last_line: st.line - 1,
        }));
    }
    Ok(Read::Done)
}

fn skip_to_eol(input: &mut impl BufRead, st: &mut State) -> Result<(), FatalError> {
    loop {
        let buf = input.fill_buf()?;
        if buf.is_empty() {
            if st.in_quotes {
                return Err(FatalError::UnterminatedQuote);
            }
            return Ok(());
        }
        let mut consumed = 0;
        let mut hit = false;
        for &b in buf {
            consumed += 1;
            if st.in_quotes {
                match b {
                    b'"' => st.in_quotes = false,
                    b'\n' => st.line += 1,
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
            st.line += 1;
            return Ok(());
        }
    }
}

fn consume(st: &mut State, buf: &[u8]) -> (usize, Step) {
    let mut consumed = 0;
    for &b in buf {
        consumed += 1;
        st.saw_any = true;
        let step = if st.in_quotes {
            step_quoted(st, b)
        } else if st.just_closed {
            step_closed(st, b)
        } else {
            step_bare(st, b)
        };
        match step {
            Step::More => {}
            done => return (consumed, done),
        }
    }
    (consumed, Step::More)
}

fn newline(st: &mut State) -> Step {
    if st.in_quotes {
        st.field.push(b'\n');
        st.line += 1;
        Step::More
    } else {
        flush_field(st);
        st.line += 1;
        Step::Ended
    }
}

fn step_quoted(st: &mut State, b: u8) -> Step {
    st.quoted_any = true;
    match b {
        b'"' => {
            st.in_quotes = false;
            st.just_closed = true;
            Step::More
        }
        b'\n' => newline(st),
        _ => {
            st.field.push(b);
            Step::More
        }
    }
}

fn step_closed(st: &mut State, b: u8) -> Step {
    match b {
        b'"' => {
            st.field.push(b'"');
            st.just_closed = false;
            st.in_quotes = true;
            Step::More
        }
        b',' => {
            flush_field(st);
            st.just_closed = false;
            st.field_start = true;
            st.quoted = false;
            Step::More
        }
        b'\n' => {
            flush_field(st);
            st.line += 1;
            Step::Ended
        }
        b'\r' | b' ' => Step::More,
        _ => Step::Bad,
    }
}

fn step_bare(st: &mut State, b: u8) -> Step {
    match b {
        b'"' if st.field_start => {
            st.in_quotes = true;
            st.quoted = true;
            st.field_start = false;
            Step::More
        }
        b'"' => Step::Bad,
        b',' => {
            flush_field(st);
            st.field_start = true;
            st.quoted = false;
            Step::More
        }
        b'\n' => newline(st),
        b'\r' => Step::More,
        b' ' if st.field_start => Step::More,
        _ => {
            st.field.push(b);
            st.field_start = false;
            Step::More
        }
    }
}

fn flush_field(st: &mut State) {
    let bytes = std::mem::take(&mut st.field);
    st.fields.push(bytes);
}

fn finish(st: State, first_line: u64) -> Read {
    Read::Row(Record {
        fields: st.fields,
        first_line,
        last_line: st.line - 1,
    })
}

fn malformed(first_line: u64, last_line: u64) -> Read {
    Read::Malformed {
        first_line,
        last_line,
        error: RowError::Malformed,
    }
}

fn over_cap(st: &State) -> bool {
    let mut total = st.field.len();
    for f in &st.fields {
        total += f.len();
        if total > MAX_RECORD_BYTES {
            return true;
        }
    }
    total > MAX_RECORD_BYTES
}
