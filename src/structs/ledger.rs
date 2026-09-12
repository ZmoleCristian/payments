use crate::structs::account::Account;
use crate::structs::ids::{ClientId, TxId};
use crate::structs::transaction::Transaction;
use std::collections::HashMap;

#[derive(Default)]
pub struct Ledger {
    pub accounts: HashMap<ClientId, Account>,
    pub transactions: HashMap<TxId, Transaction>,
}
