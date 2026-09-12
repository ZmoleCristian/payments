use crate::errors::RowError;
use crate::structs::ids::{ClientId, TxId};
use crate::structs::money::Money;
use crate::structs::row::{Row, RowKind};

pub fn parse_row(line: &str) -> Result<Row, RowError> {
    let mut fields = line.split(',');
    let kind_text = next_field(&mut fields)?;
    let client_text = next_field(&mut fields)?;
    let tx_text = next_field(&mut fields)?;
    let amount_text = fields.next();
    let kind = match kind_text {
        "deposit" => RowKind::Deposit { amount: required_amount(amount_text)? },
        "withdrawal" => RowKind::Withdrawal { amount: required_amount(amount_text)? },
        "dispute" | "resolve" | "chargeback" => no_amount_kind(kind_text)?,
        other => reject_type(other)?,
    };
    let client = parse_client(client_text)?;
    let tx = parse_tx(tx_text)?;
    Ok(Row { kind, client, tx })
}

pub fn is_header(line: &str) -> bool {
    let first = line.split(',').next();
    let Some(text) = first else {
        return false;
    };
    text.trim() == "type"
}

fn no_amount_kind(kind_text: &str) -> Result<RowKind, RowError> {
    match kind_text {
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

fn required_amount(field: Option<&str>) -> Result<Money, RowError> {
    match field {
        Some(text) => Money::parse(text.trim()),
        None => Err(RowError::Malformed),
    }
}

fn next_field<'a>(fields: &mut impl Iterator<Item = &'a str>) -> Result<&'a str, RowError> {
    match fields.next() {
        Some(field) => Ok(field.trim()),
        None => Err(RowError::Malformed),
    }
}

fn parse_client(text: &str) -> Result<ClientId, RowError> {
    let id = text.trim().parse::<u16>().map_err(reject_client)?;
    Ok(ClientId(id))
}

fn reject_client(bad: std::num::ParseIntError) -> RowError {
    if bad.to_string().is_empty() {
        return RowError::BadClient;
    }
    RowError::BadClient
}

fn parse_tx(text: &str) -> Result<TxId, RowError> {
    let id = text.trim().parse::<u32>().map_err(reject_tx)?;
    Ok(TxId(id))
}

fn reject_tx(bad: std::num::ParseIntError) -> RowError {
    if bad.to_string().is_empty() {
        return RowError::BadTx;
    }
    RowError::BadTx
}
