use crate::config::OUTPUT_HEADER;
use crate::errors::FatalError;
use crate::structs::account::Account;
use crate::structs::ids::ClientId;
use crate::structs::ledger::Ledger;
use std::io::Write;

pub fn render(ledger: &Ledger, out: &mut impl Write) -> Result<(), FatalError> {
    let mut clients: Vec<(&ClientId, &Account)> = ledger.accounts.iter().collect();
    clients.sort_by_key(|entry| entry.0 .0);
    let mut buf = String::from(OUTPUT_HEADER);
    for (id, account) in clients {
        let total = account.available.total_of(account.held)?;
        buf.push_str(&format!("{},{},{},{},{}\n", id.0, account.available, account.held, total, account.locked));
    }
    out.write_all(buf.as_bytes())?;
    Ok(())
}
