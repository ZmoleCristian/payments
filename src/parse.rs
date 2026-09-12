use crate::errors::RowError;
use crate::structs::columns::{Columns, Slot};
use crate::structs::record::{FieldError, Record};

pub fn parse_header(record: &Record) -> Result<Columns, RowError> {
    let mut kind = Slot::empty();
    let mut client = Slot::empty();
    let mut tx = Slot::empty();
    let mut amount = Slot::empty();
    let width = record.width;
    for index in 0..width {
        let raw = match record.field(index) {
            Ok(bytes) => bytes,
            Err(FieldError::OutOfRange) => return Err(RowError::Malformed),
        };
        let text = match std::str::from_utf8(raw) {
            Ok(text) => text,
            Err(bad) => return Err(RowError::BadClient(format!("invalid utf-8 at byte {}", bad.valid_up_to()))),
        };
        let name = text.trim_start_matches('\u{feff}').trim().to_ascii_lowercase();
        match name.as_str() {
            "type" => kind.claim(index)?,
            "client" => client.claim(index)?,
            "tx" => tx.claim(index)?,
            "amount" => amount.claim(index)?,
            _ => return Err(RowError::Malformed),
        }
    }
    Ok(Columns {
        kind: kind.position()?,
        client: client.position()?,
        tx: tx.position()?,
        amount: amount.position()?,
        width,
    })
}
