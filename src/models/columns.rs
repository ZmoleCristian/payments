use crate::errors::RowError;
use crate::structs::columns::{Columns, Slot};
use crate::structs::ids::{ClientId, TxId};
use crate::structs::money::Money;
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
    pub fn parse_row(&self, line: &str) -> Result<Row, RowError> {
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != self.width {
            return Err(RowError::Malformed);
        }
        let kind_text = field_at(&fields, self.kind)?;
        let client_text = field_at(&fields, self.client)?;
        let tx_text = field_at(&fields, self.tx)?;
        let amount_text = field_at(&fields, self.amount)?;
        let kind = match kind_text.to_ascii_lowercase().as_str() {
            "deposit" => RowKind::Deposit { amount: Money::parse(amount_text)? },
            "withdrawal" => RowKind::Withdrawal { amount: Money::parse(amount_text)? },
            "dispute" | "resolve" | "chargeback" => no_amount_kind(kind_text, amount_text)?,
            other => reject_type(other)?,
        };
        let client = parse_client(client_text)?;
        let tx = parse_tx(tx_text)?;
        Ok(Row { kind, client, tx })
    }
}

fn field_at<'a>(fields: &'a [&str], index: usize) -> Result<&'a str, RowError> {
    match fields.get(index) {
        Some(text) => Ok(text.trim()),
        None => Err(RowError::Malformed),
    }
}

fn no_amount_kind(kind_text: &str, amount_text: &str) -> Result<RowKind, RowError> {
    if !amount_text.is_empty() {
        return Err(RowError::Malformed);
    }
    match kind_text.to_ascii_lowercase().as_str() {
        "dispute" => Ok(RowKind::Dispute),
        "resolve" => Ok(RowKind::Resolve),
        "chargeback" => Ok(RowKind::Chargeback),
        other => reject_type(other),
    }
}

fn reject_type(other: &str) -> Result<RowKind, RowError> {
    if other.is_empty() {
        return Err(RowError::UnknownType);
    }
    Err(RowError::UnknownType)
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
