#![no_main]

use libfuzzer_sys::fuzz_target;
use payments::errors::FatalError;
use payments::run::run;
use std::io::BufReader;

fuzz_target!(|data: &[u8]| {
    let mut out: Vec<u8> = Vec::new();
    let mut report: Vec<u8> = Vec::new();
    let outcome = run(BufReader::new(data), &mut out, &mut report);
    match outcome {
        Err(FatalError::Io(_)) => panic!("io error from an in-memory reader"),
        Err(_) => {
            assert!(out.is_empty(), "a fatal run emitted stdout: {out:?}");
            return;
        }
        Ok(()) => {}
    }
    let text = match std::str::from_utf8(&out) {
        Ok(text) => text,
        Err(e) => panic!("output is not utf-8: {e}"),
    };
    check_output(text);
    match std::str::from_utf8(&report) {
        Ok(_) => {}
        Err(e) => panic!("report is not utf-8: {e}"),
    }
});

fn check_output(text: &str) {
    let mut lines = text.lines();
    match lines.next() {
        Some(header) => assert_eq!(header, "client,available,held,total,locked", "header"),
        None => panic!("output has no header"),
    }
    let mut previous: Option<u64> = None;
    for line in lines {
        let row = parse_row(line);
        let total = row.available.checked_add(row.held).expect("total fits i64");
        assert_eq!(total, row.total, "total != available + held in {line:?}");
        match previous {
            Some(last) => assert!(row.client > last, "clients out of order at {line:?}"),
            None => {}
        }
        previous = Some(row.client);
    }
}

struct Row {
    client: u64,
    available: i64,
    held: i64,
    total: i64,
}

fn parse_row(line: &str) -> Row {
    let fields: Vec<&str> = line.split(',').collect();
    assert_eq!(fields.len(), 5, "row must have 5 columns: {line:?}");
    let client = match fields.first() {
        Some(text) => text.parse::<u64>().expect("client id is a number"),
        None => panic!("missing client"),
    };
    let available = scaled(fields.get(1).expect("available column"));
    let held = scaled(fields.get(2).expect("held column"));
    let total = scaled(fields.get(3).expect("total column"));
    let locked = fields.get(4).expect("locked column");
    assert!(*locked == "true" || *locked == "false", "locked is a bool: {locked:?}");
    Row { client, available, held, total }
}

fn scaled(text: &str) -> i64 {
    let (sign, digits) = match text.strip_prefix('-') {
        Some(rest) => (-1i64, rest),
        None => (1i64, text),
    };
    let mut parts = digits.split('.');
    let whole = parts.next().expect("whole part").parse::<i64>().expect("whole is a number");
    let frac = parts.next().expect("money is printed at 4dp");
    assert_eq!(frac.len(), 4, "money must be 4dp: {text:?}");
    assert!(parts.next().is_none(), "money has one dot: {text:?}");
    let frac_value = frac.parse::<i64>().expect("frac is a number");
    let magnitude = whole.checked_mul(10_000).expect("whole fits scaled").checked_add(frac_value).expect("scaled fits i64");
    sign.checked_mul(magnitude).expect("signed fits i64")
}
