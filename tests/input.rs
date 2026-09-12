mod common;
use common::{drive, row_for};

#[test]
fn wcgw11_spaced_header_detected() {
    let o = drive("type, client, tx, amount\ndeposit, 1, 1, 1.0\n").expect("run");
    assert_eq!(row_for(&o.stdout, 1), "1,1.0000,0.0000,1.0000,false");
    assert!(o.stderr.is_empty(), "header must not be reported: {}", o.stderr);
}

#[test]
fn wcgw12_headerless_file_first_row_is_data() {
    let o = drive("deposit, 7, 9, 3.25\n").expect("run");
    assert_eq!(row_for(&o.stdout, 7), "7,3.2500,0.0000,3.2500,false");
}

#[test]
fn wcgw13_crlf_line_endings() {
    let o = drive("type, client, tx, amount\r\ndeposit, 1, 1, 2.0\r\n").expect("run");
    assert_eq!(row_for(&o.stdout, 1), "1,2.0000,0.0000,2.0000,false");
    assert!(o.stderr.is_empty(), "crlf must parse clean: {}", o.stderr);
}

#[test]
fn wcgw14_bom_stripped() {
    let o = drive("\u{feff}type, client, tx, amount\ndeposit, 1, 1, 2.0\n").expect("run");
    assert_eq!(row_for(&o.stdout, 1), "1,2.0000,0.0000,2.0000,false");
}

#[test]
fn wcgw15_blank_lines_skipped_silently() {
    let o = drive("type,client,tx,amount\n\n   \n\ndeposit,1,1,1.0\n\n").expect("run");
    assert_eq!(row_for(&o.stdout, 1), "1,1.0000,0.0000,1.0000,false");
    assert!(o.stderr.is_empty(), "blank lines must not be reported: {}", o.stderr);
}

#[test]
fn wcgw16_wrong_field_count_is_malformed() {
    let o = drive("type,client,tx,amount\ndeposit,1,1\n").expect("run");
    assert!(o.stderr.contains("line 2: malformed row"), "stderr: {}", o.stderr);
}

#[test]
fn wcgw17_bare_decimal_both_sides() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,.5\ndeposit,1,2,5.\n").expect("run");
    assert_eq!(row_for(&o.stdout, 1), "1,5.5000,0.0000,5.5000,false");
}

#[test]
fn wcgw18_no_final_newline_last_row_counts() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,1.0").expect("run");
    assert_eq!(row_for(&o.stdout, 1), "1,1.0000,0.0000,1.0000,false");
}

#[test]
fn wcgw19_bad_utf8_poisons_one_line_only() {
    let mut bytes: Vec<u8> = b"type,client,tx,amount\ndeposit,1,1,1.0\n".to_vec();
    bytes.extend_from_slice(b"deposit,2,2,\xff\xfe\n");
    bytes.extend_from_slice(b"deposit,3,3,3.0\n");
    let reader = std::io::BufReader::new(bytes.as_slice());
    let mut out: Vec<u8> = Vec::new();
    let mut report: Vec<u8> = Vec::new();
    payments::run::run(reader, &mut out, &mut report).expect("run");
    let stdout = String::from_utf8(out).expect("utf8");
    let stderr = String::from_utf8(report).expect("utf8");
    assert_eq!(row_for(&stdout, 1), "1,1.0000,0.0000,1.0000,false");
    assert_eq!(row_for(&stdout, 3), "3,3.0000,0.0000,3.0000,false");
    assert!(stderr.contains("line 3"), "poisoned line reported: {stderr}");
    assert!(!stdout.contains('2'), "poisoned row must not appear: {stdout}");
}

#[test]
fn wcgw20_zero_ids_are_valid() {
    let o = drive("type,client,tx,amount\ndeposit,0,0,1.0\n").expect("run");
    assert_eq!(row_for(&o.stdout, 0), "0,1.0000,0.0000,1.0000,false");
}

#[test]
fn wcgw21_leading_zeros_parse() {
    let o = drive("type,client,tx,amount\ndeposit,007,0042,1.0\n").expect("run");
    assert_eq!(row_for(&o.stdout, 7), "7,1.0000,0.0000,1.0000,false");
}
