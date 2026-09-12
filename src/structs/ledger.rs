use crate::structs::account::Account;
use crate::structs::ids::{ClientId, TxId};
use crate::structs::transaction::Transaction;
use std::collections::HashMap;

pub struct Ledger {
    pub accounts: HashMap<ClientId, Account>,
    pub transactions: HashMap<TxId, Transaction>,
}
