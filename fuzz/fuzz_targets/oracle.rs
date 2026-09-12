#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use payments::run::run;
use std::collections::HashMap;
use std::io::BufReader;

#[derive(Arbitrary, Debug)]
enum Op {
    Deposit { client: u8, tx: u16, cents: u32 },
    Withdrawal { client: u8, tx: u16, cents: u32 },
    Dispute { client: u8, tx: u16 },
    Resolve { client: u8, tx: u16 },
    Chargeback { client: u8, tx: u16 },
}

#[derive(Default, Clone, Copy)]
struct Account {
    available: i64,
    held: i64,
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
    amount: i64,
    disputed: bool,
    burned: bool,
}

fuzz_target!(|ops: Vec<Op>| {
    let mut csv = String::from("type,client,tx,amount\n");
    for op in &ops {
        render_op(&mut csv, op);
    }
    let mut out: Vec<u8> = Vec::new();
    let mut report: Vec<u8> = Vec::new();
    match run(BufReader::new(csv.as_bytes()), &mut out, &mut report) {
        Ok(()) => {}
        Err(e) => panic!("generated csv must never be fatal: {e}\n{csv}"),
    }
    let text = match std::str::from_utf8(&out) {
        Ok(text) => text,
        Err(e) => panic!("output not utf-8: {e}"),
    };
    let expected = model(&ops);
    compare(text, &expected, &csv);
});

fn render_op(csv: &mut String, op: &Op) {
    match op {
        Op::Deposit { client, tx, cents } => csv.push_str(&format!("deposit,{client},{tx},{}\n", money_text(*cents))),
        Op::Withdrawal { client, tx, cents } => csv.push_str(&format!("withdrawal,{client},{tx},{}\n", money_text(*cents))),
        Op::Dispute { client, tx } => csv.push_str(&format!("dispute,{client},{tx},\n")),
        Op::Resolve { client, tx } => csv.push_str(&format!("resolve,{client},{tx},\n")),
        Op::Chargeback { client, tx } => csv.push_str(&format!("chargeback,{client},{tx},\n")),
    }
}

fn money_text(cents: u32) -> String {
    let whole = cents / 10_000;
    let frac = cents % 10_000;
    format!("{whole}.{frac:04}")
}

fn scaled(cents: u32) -> i64 {
    i64::from(cents)
}

fn model(ops: &[Op]) -> HashMap<u8, Account> {
    let mut accounts: HashMap<u8, Account> = HashMap::new();
    let mut txs: HashMap<u16, Record> = HashMap::new();
    for op in ops {
        let client = client_of(op);
        accounts.entry(client).or_default();
        let locked = match accounts.get(&client) {
            Some(account) => account.locked,
            None => false,
        };
        if locked {
            continue;
        }
        apply(op, &mut accounts, &mut txs);
    }
    accounts
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

fn apply(op: &Op, accounts: &mut HashMap<u8, Account>, txs: &mut HashMap<u16, Record>) {
    match op {
        Op::Deposit { client, tx, cents } => open(accounts, txs, *client, *tx, scaled(*cents), Kind::Deposit),
        Op::Withdrawal { client, tx, cents } => open(accounts, txs, *client, *tx, scaled(*cents), Kind::Withdrawal),
        Op::Dispute { client, tx } => dispute(accounts, txs, *client, *tx),
        Op::Resolve { client, tx } => settle(accounts, txs, *client, *tx, false),
        Op::Chargeback { client, tx } => settle(accounts, txs, *client, *tx, true),
    }
}

fn open(accounts: &mut HashMap<u8, Account>, txs: &mut HashMap<u16, Record>, client: u8, tx: u16, amount: i64, kind: Kind) {
    if txs.contains_key(&tx) {
        return;
    }
    let account = accounts.entry(client).or_default();
    match kind {
        Kind::Deposit => {
            let Some(available) = account.available.checked_add(amount) else {
                return;
            };
            if available.checked_add(account.held).is_none() {
                return;
            }
            account.available = available;
        }
        Kind::Withdrawal => {
            if account.available < amount {
                return;
            }
            let Some(available) = account.available.checked_sub(amount) else {
                return;
            };
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
    let Some(available) = account.available.checked_sub(record.amount) else {
        return;
    };
    let Some(held) = account.held.checked_add(record.amount) else {
        return;
    };
    if available.checked_add(held).is_none() {
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
    let Some(held) = account.held.checked_sub(record.amount) else {
        return;
    };
    if back {
        account.held = held;
        account.locked = true;
        record.disputed = false;
        record.burned = true;
        return;
    }
    let Some(available) = account.available.checked_add(record.amount) else {
        return;
    };
    if available.checked_add(held).is_none() {
        return;
    }
    account.held = held;
    account.available = available;
    record.disputed = false;
}

fn compare(text: &str, expected: &HashMap<u8, Account>, csv: &str) {
    let mut seen: HashMap<u64, (i64, i64, bool)> = HashMap::new();
    let mut lines = text.lines();
    match lines.next() {
        Some(header) => assert_eq!(header, "client,available,held,total,locked", "header"),
        None => panic!("no header"),
    }
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 5, "columns in {line:?}");
        let client = field(&fields, 0).parse::<u64>().expect("client id");
        let available = unscale(field(&fields, 1));
        let held = unscale(field(&fields, 2));
        let total = unscale(field(&fields, 3));
        let locked = field(&fields, 4) == "true";
        assert_eq!(total, available + held, "total != available + held in {line:?}");
        seen.insert(client, (available, held, locked));
    }
    assert_eq!(seen.len(), expected.len(), "client count\nmodel {} engine {}\n{csv}", expected.len(), seen.len());
    for (client, account) in expected {
        let key = u64::from(*client);
        let Some((available, held, locked)) = seen.get(&key) else {
            panic!("engine dropped client {client}\n{csv}");
        };
        assert_eq!(*available, account.available, "client {client} available\n{csv}");
        assert_eq!(*held, account.held, "client {client} held\n{csv}");
        assert_eq!(*locked, account.locked, "client {client} locked\n{csv}");
    }
}

fn field<'a>(fields: &[&'a str], index: usize) -> &'a str {
    match fields.get(index) {
        Some(text) => text,
        None => panic!("missing column {index}"),
    }
}

fn unscale(text: &str) -> i64 {
    let (sign, digits) = match text.strip_prefix('-') {
        Some(rest) => (-1i64, rest),
        None => (1i64, text),
    };
    let mut parts = digits.split('.');
    let whole = parts.next().expect("whole").parse::<i64>().expect("whole number");
    let frac = parts.next().expect("frac");
    assert_eq!(frac.len(), 4, "4dp in {text:?}");
    sign * (whole * 10_000 + frac.parse::<i64>().expect("frac number"))
}
