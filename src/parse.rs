use crate::errors::RowError;
use crate::structs::columns::{Columns, Slot};

pub fn parse_header(line: &str) -> Result<Columns, RowError> {
    let line = line.trim_start_matches('\u{feff}');
    let mut kind = Slot::empty();
    let mut client = Slot::empty();
    let mut tx = Slot::empty();
    let mut amount = Slot::empty();
    let mut width = 0;
    for (index, raw) in line.split(',').enumerate() {
        width = index + 1;
        match raw.trim().to_ascii_lowercase().as_str() {
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
