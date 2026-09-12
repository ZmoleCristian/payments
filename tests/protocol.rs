mod common;
use common::{drive, row_for};

#[test]
fn wcgw22_duplicate_tx_id_rejected() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,10\ndeposit,2,1,99\n").expect("run");
    assert!(o.stderr.contains("line 3: duplicate transaction id"), "stderr: {}", o.stderr);
    assert_eq!(row_for(&o.stdout, 1), "1,10.0000,0.0000,10.0000,false");
    assert_eq!(row_for(&o.stdout, 2), "2,0.0000,0.0000,0.0000,false", "well-formed row names its client even when rejected");
}

#[test]
fn wcgw22_burned_id_stays_burned_after_chargeback() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,10\ndispute,1,1,\nchargeback,1,1,\ndeposit,2,1,99\n").expect("run");
    assert!(o.stderr.contains("line 5: duplicate transaction id"), "burned id must reject reuse: {}", o.stderr);
    assert_eq!(row_for(&o.stdout, 2), "2,0.0000,0.0000,0.0000,false");
}

#[test]
fn wcgw23_failed_withdrawal_names_its_client() {
    let o = drive("type,client,tx,amount\nwithdrawal,3,9,5\n").expect("run");
    assert!(o.stderr.contains("line 2: insufficient funds"), "stderr: {}", o.stderr);
    assert_eq!(row_for(&o.stdout, 3), "3,0.0000,0.0000,0.0000,false", "rejected but well-formed: client appears");
}

#[test]
fn wcgw30_locked_account_held_funds_stay_held_forever() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,10\ndeposit,1,2,20\ndispute,1,1,\ndispute,1,2,\nchargeback,1,1,\nresolve,1,2,\nchargeback,1,2,\n").expect("run");
    assert!(o.stderr.contains("line 7: account locked"), "stderr: {}", o.stderr);
    assert!(o.stderr.contains("line 8: account locked"), "stderr: {}", o.stderr);
    assert_eq!(row_for(&o.stdout, 1), "1,0.0000,20.0000,20.0000,true", "locked is absolute: held has no exit");
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
