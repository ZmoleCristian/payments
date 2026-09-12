use crate::errors::RowError;
use crate::structs::columns::{Columns, Slot};
use crate::structs::ids::{ClientId, TxId};
use crate::structs::money::Money;
use crate::structs::record::{FieldError, Record};
use crate::structs::row::{Row, RowKind};

impl Slot {
    pub fn empty() -> Slot {
        Slot { seen: false, index: 0 }
    }

    pub fn claim(&mut self, index: usize) -> Result<(), RowError> {
        if self.seen {
            return Err(RowError::Malformed);
        }
        self.seen = true;
        self.index = index;
        Ok(())
    }

    pub fn position(&self) -> Result<usize, RowError> {
        if !self.seen {
            return Err(RowError::Malformed);
        }
        Ok(self.index)
    }
}

impl Columns {
    pub fn parse_row(&self, record: &Record) -> Result<Row, RowError> {
        if record.width != self.width {
            return Err(RowError::Malformed);
        }
        let kind_text = field_at(record, self.kind)?;
        let client_text = field_at(record, self.client)?;
        let tx_text = field_at(record, self.tx)?;
        let amount_text = field_at(record, self.amount)?;
        let kind = kind_of(kind_text, amount_text)?;
        let client = parse_client(client_text)?;
        let tx = parse_tx(tx_text)?;
        Ok(Row { kind, client, tx })
    }
}

fn kind_of(kind_text: &str, amount_text: &str) -> Result<RowKind, RowError> {
    if kind_text.eq_ignore_ascii_case("deposit") {
        return Ok(RowKind::Deposit { amount: Money::parse(amount_text)? });
    }
    if kind_text.eq_ignore_ascii_case("withdrawal") {
        return Ok(RowKind::Withdrawal { amount: Money::parse(amount_text)? });
    }
    if kind_text.eq_ignore_ascii_case("dispute") {
        return no_amount_kind(RowKind::Dispute, amount_text);
    }
    if kind_text.eq_ignore_ascii_case("resolve") {
        return no_amount_kind(RowKind::Resolve, amount_text);
    }
    if kind_text.eq_ignore_ascii_case("chargeback") {
        return no_amount_kind(RowKind::Chargeback, amount_text);
    }
    Err(RowError::UnknownType)
}

fn field_at(record: &Record, index: usize) -> Result<&str, RowError> {
    let raw = match record.field(index) {
        Ok(bytes) => bytes,
        Err(FieldError::OutOfRange) => return Err(RowError::Malformed),
    };
    let text = match std::str::from_utf8(raw) {
        Ok(text) => text,
        Err(bad) => return Err(RowError::BadClient(format!("invalid utf-8 at byte {}", bad.valid_up_to()))),
    };
    Ok(text.trim())
}

fn no_amount_kind(kind: RowKind, amount_text: &str) -> Result<RowKind, RowError> {
    if !amount_text.is_empty() {
        return Err(RowError::Malformed);
    }
    Ok(kind)
}

fn parse_client(text: &str) -> Result<ClientId, RowError> {
    match text.parse::<u16>() {
        Ok(id) => Ok(ClientId(id)),
        Err(bad) => Err(RowError::BadClient(evidence(text, bad))),
    }
}

fn parse_tx(text: &str) -> Result<TxId, RowError> {
    match text.parse::<u32>() {
        Ok(id) => Ok(TxId(id)),
        Err(bad) => Err(RowError::BadTx(evidence(text, bad))),
    }
}

fn evidence(text: &str, bad: std::num::ParseIntError) -> String {
    format!("'{text}' ({bad})")
}
