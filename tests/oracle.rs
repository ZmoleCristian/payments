use proptest::prelude::{any, prop_oneof, Just, Strategy};
use proptest::{prop_assert_eq, proptest};
use std::collections::HashMap;

const I64_MAX: i128 = 9_223_372_036_854_775_807;
const I64_MIN: i128 = -9_223_372_036_854_775_808;

#[derive(Clone, Debug)]
enum Op {
    Deposit { client: u8, tx: u16, amount: i128 },
    Withdrawal { client: u8, tx: u16, amount: i128 },
    Dispute { client: u8, tx: u16 },
    Resolve { client: u8, tx: u16 },
    Chargeback { client: u8, tx: u16 },
}

#[derive(Default, Clone, Copy, PartialEq, Debug)]
struct Account {
    available: i128,
    held: i128,
    locked: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Deposit,
    Withdrawal,
}

struct Record {
    client: u8,
    kind: Kind,
    amount: i128,
    disputed: bool,
    burned: bool,
}

fn amount_strategy() -> impl Strategy<Value = i128> {
    prop_oneof![
        4 => 0i128..1_000_000i128,
        2 => 0i128..I64_MAX,
        1 => Just(0i128),
        1 => Just(I64_MAX),
        1 => Just(I64_MAX - 1),
    ]
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        3 => (client_id(), tx_id(), amount_strategy()).prop_map(|(client, tx, amount)| Op::Deposit { client, tx, amount }),
        2 => (client_id(), tx_id(), amount_strategy()).prop_map(|(client, tx, amount)| Op::Withdrawal { client, tx, amount }),
        2 => (client_id(), tx_id()).prop_map(|(client, tx)| Op::Dispute { client, tx }),
        1 => (client_id(), tx_id()).prop_map(|(client, tx)| Op::Resolve { client, tx }),
        1 => (client_id(), tx_id()).prop_map(|(client, tx)| Op::Chargeback { client, tx }),
    ]
}

fn client_id() -> impl Strategy<Value = u8> {
    prop_oneof![
        4 => 0u8..4u8,
        1 => any::<u8>(),
    ]
}

fn tx_id() -> impl Strategy<Value = u16> {
    prop_oneof![
        4 => 0u16..8u16,
        1 => any::<u16>(),
    ]
}

fn render_csv(ops: &[Op]) -> String {
    let mut csv = String::from("type,client,tx,amount\n");
    for op in ops {
        match op {
            Op::Deposit { client, tx, amount } => csv.push_str(&format!("deposit,{client},{tx},{}\n", money_text(*amount))),
            Op::Withdrawal { client, tx, amount } => csv.push_str(&format!("withdrawal,{client},{tx},{}\n", money_text(*amount))),
            Op::Dispute { client, tx } => csv.push_str(&format!("dispute,{client},{tx},\n")),
            Op::Resolve { client, tx } => csv.push_str(&format!("resolve,{client},{tx},\n")),
            Op::Chargeback { client, tx } => csv.push_str(&format!("chargeback,{client},{tx},\n")),
        }
    }
    csv
}

fn money_text(scaled: i128) -> String {
    let whole = scaled.div_euclid(10_000);
    let frac = scaled.rem_euclid(10_000);
    format!("{whole}.{frac:04}")
}

fn client_of(op: &Op) -> u8 {
    match op {
        Op::Deposit { client, .. } => *client,
        Op::Withdrawal { client, .. } => *client,
        Op::Dispute { client, .. } => *client,
        Op::Resolve { client, .. } => *client,
        Op::Chargeback { client, .. } => *client,
    }
}

fn fits(value: i128) -> bool {
    (I64_MIN..=I64_MAX).contains(&value)
}

fn model(ops: &[Op]) -> HashMap<u8, Account> {
    let mut accounts: HashMap<u8, Account> = HashMap::new();
    let mut txs: HashMap<u16, Record> = HashMap::new();
    for op in ops {
        let client = client_of(op);
        let entry = accounts.entry(client).or_default();
        if entry.locked {
            continue;
        }
        match op {
            Op::Deposit { tx, amount, .. } => open(&mut accounts, &mut txs, client, *tx, *amount, Kind::Deposit),
            Op::Withdrawal { tx, amount, .. } => open(&mut accounts, &mut txs, client, *tx, *amount, Kind::Withdrawal),
            Op::Dispute { tx, .. } => dispute(&mut accounts, &mut txs, client, *tx),
            Op::Resolve { tx, .. } => settle(&mut accounts, &mut txs, client, *tx, false),
            Op::Chargeback { tx, .. } => settle(&mut accounts, &mut txs, client, *tx, true),
        }
    }
    accounts
}

fn open(accounts: &mut HashMap<u8, Account>, txs: &mut HashMap<u16, Record>, client: u8, tx: u16, amount: i128, kind: Kind) {
    if txs.contains_key(&tx) {
        return;
    }
    let account = accounts.entry(client).or_default();
    match kind {
        Kind::Deposit => {
            let available = account.available + amount;
            if !fits(available) || !fits(available + account.held) {
                return;
            }
            account.available = available;
        }
        Kind::Withdrawal => {
            if account.available < amount {
                return;
            }
            let available = account.available - amount;
            if !fits(available) {
                return;
            }
            account.available = available;
        }
    }
    txs.insert(
        tx,
        Record {
            client,
            kind,
            amount,
            disputed: false,
            burned: false,
        },
    );
}

fn dispute(accounts: &mut HashMap<u8, Account>, txs: &mut HashMap<u16, Record>, client: u8, tx: u16) {
    let Some(record) = txs.get_mut(&tx) else {
        return;
    };
    if record.client != client || record.burned || record.kind == Kind::Withdrawal || record.disputed || record.amount == 0 {
        return;
    }
    let Some(account) = accounts.get_mut(&client) else {
        return;
    };
    let available = account.available - record.amount;
    let held = account.held + record.amount;
    if !fits(available) || !fits(held) || !fits(available + held) {
        return;
    }
    account.available = available;
    account.held = held;
    record.disputed = true;
}

fn settle(accounts: &mut HashMap<u8, Account>, txs: &mut HashMap<u16, Record>, client: u8, tx: u16, back: bool) {
    let Some(record) = txs.get_mut(&tx) else {
        return;
    };
    if record.client != client || record.burned || record.kind == Kind::Withdrawal || !record.disputed {
        return;
    }
    let Some(account) = accounts.get_mut(&client) else {
        return;
    };
    let held = account.held - record.amount;
    if !fits(held) {
        return;
    }
    if back {
        account.held = held;
        account.locked = true;
        record.disputed = false;
        record.burned = true;
        return;
    }
    let available = account.available + record.amount;
    if !fits(available) || !fits(available + held) {
        return;
    }
    account.held = held;
    account.available = available;
    record.disputed = false;
}

fn engine(csv: &str) -> Result<HashMap<u8, Account>, String> {
    let mut out: Vec<u8> = Vec::new();
    let mut report: Vec<u8> = Vec::new();
    match payments::run::run(std::io::BufReader::new(csv.as_bytes()), &mut out, &mut report) {
        Ok(()) => {}
        Err(e) => return Err(format!("fatal: {e}")),
    }
    let text = match String::from_utf8(out) {
        Ok(text) => text,
        Err(e) => return Err(format!("non utf-8 output: {e}")),
    };
    let mut rows: HashMap<u8, Account> = HashMap::new();
    let mut lines = text.lines();
    match lines.next() {
        Some("client,available,held,total,locked") => {}
        Some(other) => return Err(format!("bad header: {other}")),
        None => return Err(String::from("no header")),
    }
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        let [client, available, held, total, locked] = fields.as_slice() else {
            return Err(format!("bad row: {line}"));
        };
        let client = match client.parse::<u8>() {
            Ok(id) => id,
            Err(e) => return Err(format!("bad client {client}: {e}")),
        };
        let available = unscale(available)?;
        let held = unscale(held)?;
        let total = unscale(total)?;
        if total != available + held {
            return Err(format!("total != available + held: {line}"));
        }
        rows.insert(
            client,
            Account {
                available,
                held,
                locked: *locked == "true",
            },
        );
    }
    Ok(rows)
}

fn unscale(text: &str) -> Result<i128, String> {
    let (sign, digits) = match text.strip_prefix('-') {
        Some(rest) => (-1i128, rest),
        None => (1i128, text),
    };
    let mut parts = digits.split('.');
    let Some(whole_text) = parts.next() else {
        return Err(format!("no whole part: {text}"));
    };
    let Some(frac_text) = parts.next() else {
        return Err(format!("money must be 4dp: {text}"));
    };
    if frac_text.len() != 4 {
        return Err(format!("money must be 4dp: {text}"));
    }
    let whole = match whole_text.parse::<i128>() {
        Ok(value) => value,
        Err(e) => return Err(format!("bad whole {whole_text}: {e}")),
    };
    let frac = match frac_text.parse::<i128>() {
        Ok(value) => value,
        Err(e) => return Err(format!("bad frac {frac_text}: {e}")),
    };
    Ok(sign * (whole * 10_000 + frac))
}

proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(4096))]

    #[test]
    fn engine_matches_independent_model(ops in proptest::collection::vec(op_strategy(), 0..60)) {
        let csv = render_csv(&ops);
        let actual = engine(&csv).expect("engine run");
        let expected = model(&ops);
        prop_assert_eq!(actual.len(), expected.len(), "client count for\n{}", csv);
        for (client, account) in &expected {
            let found = actual.get(client).expect("client present");
            prop_assert_eq!(found, account, "client {} for\n{}", client, csv);
        }
    }
}
