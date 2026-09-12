mod common;
use common::{drive, row_for};

#[test]
fn wcgw22_duplicate_tx_id_rejected() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,10\ndeposit,2,1,99\n").expect("run");
    assert!(o.stderr.contains("line 3: duplicate transaction id"), "stderr: {}", o.stderr);
    assert!(!o.stdout.contains('2'), "second client must not exist: {}", o.stdout);
    assert_eq!(row_for(&o.stdout, 1), "1,10.0000,0.0000,10.0000,false");
}

#[test]
fn wcgw23_dispute_on_failed_withdrawal_is_unknown_tx() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,5\nwithdrawal,1,2,99\ndispute,1,2,\n").expect("run");
    assert!(o.stderr.contains("line 3: insufficient funds"), "stderr: {}", o.stderr);
    assert!(o.stderr.contains("line 4: unknown transaction id"), "stderr: {}", o.stderr);
    assert_eq!(row_for(&o.stdout, 1), "1,5.0000,0.0000,5.0000,false");
}

#[test]
fn wcgw24_cross_client_reference_is_unknown_tx() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,10\ndispute,2,1,\n").expect("run");
    assert!(o.stderr.contains("line 3: unknown transaction id"), "stderr: {}", o.stderr);
    assert_eq!(row_for(&o.stdout, 1), "1,10.0000,0.0000,10.0000,false");
}

#[test]
fn wcgw25_double_dispute_rejected() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,10\ndispute,1,1,\ndispute,1,1,\n").expect("run");
    assert!(o.stderr.contains("line 4: transaction already under dispute"), "stderr: {}", o.stderr);
    assert_eq!(row_for(&o.stdout, 1), "1,0.0000,10.0000,10.0000,false");
}

#[test]
fn wcgw26_resolve_without_dispute_rejected() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,10\nresolve,1,1,\n").expect("run");
    assert!(o.stderr.contains("line 3: transaction not under dispute"), "stderr: {}", o.stderr);
    assert_eq!(row_for(&o.stdout, 1), "1,10.0000,0.0000,10.0000,false");
}

#[test]
fn wcgw26_chargeback_without_dispute_rejected() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,10\nchargeback,1,1,\n").expect("run");
    assert!(o.stderr.contains("line 3: transaction not under dispute"), "stderr: {}", o.stderr);
    assert_eq!(row_for(&o.stdout, 1), "1,10.0000,0.0000,10.0000,false");
}

#[test]
fn wcgw27_redispute_after_resolve_allowed() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,10\ndispute,1,1,\nresolve,1,1,\ndispute,1,1,\n").expect("run");
    assert!(o.stderr.is_empty(), "re-dispute must be clean: {}", o.stderr);
    assert_eq!(row_for(&o.stdout, 1), "1,0.0000,10.0000,10.0000,false");
}

#[test]
fn wcgw28_amount_on_dispute_row_ignored() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,10\ndispute,1,1,999\n").expect("run");
    assert_eq!(row_for(&o.stdout, 1), "1,0.0000,10.0000,10.0000,false");
}

#[test]
fn wcgw29_type_case_sensitive() {
    let o = drive("type,client,tx,amount\nDeposit,1,1,10\n").expect("run");
    assert!(o.stderr.contains("line 2: unknown transaction type"), "stderr: {}", o.stderr);
}

#[test]
fn wcgw30_locked_account_rejects_everything_each_reported() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,10\ndispute,1,1,\nchargeback,1,1,\ndeposit,1,2,5\nwithdrawal,1,3,1\ndispute,1,2,\n").expect("run");
    assert!(o.stderr.contains("line 5: account locked"), "stderr: {}", o.stderr);
    assert!(o.stderr.contains("line 6: account locked"), "stderr: {}", o.stderr);
    assert!(o.stderr.contains("line 7: account locked"), "stderr: {}", o.stderr);
    assert_eq!(row_for(&o.stdout, 1), "1,0.0000,0.0000,0.0000,true");
}

#[test]
fn wcgw31_forward_reference_is_unknown_tx_at_that_moment() {
    let o = drive("type,client,tx,amount\ndispute,1,5,\ndeposit,1,5,10\n").expect("run");
    assert!(o.stderr.contains("line 2: unknown transaction id"), "stderr: {}", o.stderr);
    assert_eq!(row_for(&o.stdout, 1), "1,10.0000,0.0000,10.0000,false");
}
