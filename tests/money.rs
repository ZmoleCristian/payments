mod common;
use common::{drive, row_for};

#[test]
fn wcgw2_nonzero_past_four_decimals_rejected_zero_padding_accepted() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,1.00005\ndeposit,2,2,1.50000\ndeposit,3,3,2.00000001\n").expect("run");
    assert!(o.stderr.contains("line 2: invalid amount"), "real precision loss: {}", o.stderr);
    assert!(o.stderr.contains("line 4: invalid amount"), "nonzero 8th decimal: {}", o.stderr);
    assert_eq!(row_for(&o.stdout, 2), "2,1.5000,0.0000,1.5000,false", "padded zeros parse");
}

#[test]
fn wcgw3_rejects_overflowing_amount() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,99999999999999999999\n").expect("run");
    assert!(o.stderr.contains("line 2: invalid amount"), "stderr: {}", o.stderr);
}

#[test]
fn wcgw3_fourteen_whole_digits_parse_fifteen_overflow() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,99999999999999.9999\ndeposit,2,2,999999999999999.9999\n").expect("run");
    assert_eq!(row_for(&o.stdout, 1), "1,99999999999999.9999,0.0000,99999999999999.9999,false", "14 digits fit in scaled i64");
    assert!(o.stderr.contains("line 3: invalid amount"), "15 nines overflow scaled i64: {}", o.stderr);
    assert!(!o.stdout.lines().any(|l| l.starts_with("2,")), "unrepresentable amount is malformed-class: no client row");
}

#[test]
fn wcgw4_rejects_signed_amount() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,-5.0\n").expect("run");
    assert!(o.stderr.contains("line 2: invalid amount"), "stderr: {}", o.stderr);
}

#[test]
fn wcgw5_zero_amount_tx_recorded_but_not_disputable() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,0\ndispute,1,1,\n").expect("run");
    assert!(o.stderr.contains("line 3: invalid amount"), "disputing zero is noise: {}", o.stderr);
    assert_eq!(row_for(&o.stdout, 1), "1,0.0000,0.0000,0.0000,false");
}

#[test]
fn wcgw6_midrun_overflow_is_row_error_not_wrap() {
    let input = "type,client,tx,amount\ndeposit,1,1,800000000000000.0000\ndeposit,1,2,800000000000000.0000\n";
    let o = drive(input).expect("run");
    assert!(o.stderr.contains("line 3: invalid amount"), "stderr: {}", o.stderr);
    assert_eq!(row_for(&o.stdout, 1), "1,800000000000000.0000,0.0000,800000000000000.0000,false");
}

#[test]
fn wcgw7_dispute_on_spent_funds_shows_negative_available() {
    let o = drive("type,client,tx,amount\ndeposit,2,2,5.0\nwithdrawal,2,3,4.0\ndispute,2,2,\n").expect("run");
    assert_eq!(row_for(&o.stdout, 2), "2,-4.0000,5.0000,1.0000,false");
}

#[test]
fn wcgw8_withdrawal_chargeback_claws_back_to_pre_withdrawal_total() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,100\nwithdrawal,1,2,30\ndispute,1,2,\nchargeback,1,2,\n").expect("run");
    assert_eq!(row_for(&o.stdout, 1), "1,100.0000,0.0000,100.0000,true");
}

#[test]
fn wcgw8_deposit_chargeback_removes_funds_and_locks() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,100\ndeposit,1,2,50\ndispute,1,2,\nchargeback,1,2,\n").expect("run");
    assert_eq!(row_for(&o.stdout, 1), "1,100.0000,0.0000,100.0000,true");
}

#[test]
fn wcgw9_negative_fractional_available_prints_truncated_sign() {
    let o = drive("type,client,tx,amount\ndeposit,2,2,5.0\nwithdrawal,2,3,1.5\ndispute,2,2,\n").expect("run");
    assert_eq!(row_for(&o.stdout, 2), "2,-1.5000,5.0000,3.5000,false");
}

#[test]
fn wcgw10_display_min_value_edge() {
    let o = drive("type,client,tx,amount\ndeposit,2,2,5.0\nwithdrawal,2,3,4.9999\ndispute,2,2,\n").expect("run");
    assert_eq!(row_for(&o.stdout, 2), "2,-4.9999,5.0000,0.0001,false");
}
