use crate::errors::RowError;
use crate::structs::account::Account;
use crate::structs::ids::{ClientId, TxId};
use crate::structs::ledger::Ledger;
use crate::structs::money::Money;
use crate::structs::row::Row;
use crate::structs::row::RowKind;
use crate::structs::transaction::{Transaction, TxKind};
use std::collections::hash_map::Entry;

impl Ledger {
    pub fn new() -> Ledger {
        Ledger {
            accounts: std::collections::HashMap::new(),
            transactions: std::collections::HashMap::new(),
        }
    }

    pub fn apply(&mut self, row: &Row) -> Result<(), RowError> {
        self.account_for(row.client);
        self.gate_locked(row.client)?;
        match row.kind {
            RowKind::Deposit { amount } => self.deposit(row.client, row.tx, amount),
            RowKind::Withdrawal { amount } => self.withdraw(row.client, row.tx, amount),
            RowKind::Dispute => self.dispute(row.client, row.tx),
            RowKind::Resolve => self.resolve(row.client, row.tx),
            RowKind::Chargeback => self.chargeback(row.client, row.tx),
        }
    }

    fn gate_locked(&self, client: ClientId) -> Result<(), RowError> {
        let Some(account) = self.accounts.get(&client) else {
            return Ok(());
        };
        if account.locked {
            return Err(RowError::AccountLocked);
        }
        Ok(())
    }

    fn account_for(&mut self, client: ClientId) -> &mut Account {
        match self.accounts.entry(client) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(Account::new()),
        }
    }

    fn own_tx(&self, client: ClientId, tx: TxId) -> Result<&Transaction, RowError> {
        match self.transactions.get(&tx) {
            Some(record) => {
                if record.client == client && !record.burned {
                    Ok(record)
                } else {
                    Err(RowError::UnknownTx)
                }
            }
            None => Err(RowError::UnknownTx),
        }
    }

    fn deposit(&mut self, client: ClientId, tx: TxId, amount: Money) -> Result<(), RowError> {
        if self.transactions.contains_key(&tx) {
            return Err(RowError::DuplicateTx);
        }
        let account = self.account_for(client);
        account.available = account.available.checked_add(amount)?;
        self.transactions.insert(
            tx,
            Transaction {
                client,
                kind: TxKind::Deposit,
                amount,
                disputed: false,
                burned: false,
            },
        );
        Ok(())
    }

    fn withdraw(&mut self, client: ClientId, tx: TxId, amount: Money) -> Result<(), RowError> {
        if self.transactions.contains_key(&tx) {
            return Err(RowError::DuplicateTx);
        }
        let account = self.account_for(client);
        if !account.available.sufficient(amount) {
            return Err(RowError::InsufficientFunds);
        }
        account.available = account.available.checked_sub(amount)?;
        self.transactions.insert(
            tx,
            Transaction {
                client,
                kind: TxKind::Withdrawal,
                amount,
                disputed: false,
                burned: false,
            },
        );
        Ok(())
    }

    fn dispute(&mut self, client: ClientId, tx: TxId) -> Result<(), RowError> {
        let amount = {
            let record = self.own_tx(client, tx)?;
            if record.disputed {
                return Err(RowError::AlreadyDisputed);
            }
            record.amount
        };
        let account = self.account_for(client);
        account.available = account.available.checked_sub(amount)?;
        account.held = account.held.checked_add(amount)?;
        match self.transactions.get_mut(&tx) {
            Some(record) => {
                record.disputed = true;
                Ok(())
            }
            None => Err(RowError::UnknownTx),
        }
    }

    fn resolve(&mut self, client: ClientId, tx: TxId) -> Result<(), RowError> {
        let amount = {
            let record = self.own_tx(client, tx)?;
            if !record.disputed {
                return Err(RowError::NotDisputed);
            }
            record.amount
        };
        let account = self.account_for(client);
        account.held = account.held.checked_sub(amount)?;
        account.available = account.available.checked_add(amount)?;
        match self.transactions.get_mut(&tx) {
            Some(record) => {
                record.disputed = false;
                Ok(())
            }
            None => Err(RowError::UnknownTx),
        }
    }

    fn chargeback(&mut self, client: ClientId, tx: TxId) -> Result<(), RowError> {
        let (amount, kind) = {
            let record = self.own_tx(client, tx)?;
            if !record.disputed {
                return Err(RowError::NotDisputed);
            }
            (record.amount, record.kind)
        };
        let account = self.account_for(client);
        account.held = account.held.checked_sub(amount)?;
        match kind {
            TxKind::Deposit => {}
            TxKind::Withdrawal => {
                account.available = account.available.checked_add(amount)?;
                account.available = account.available.checked_add(amount)?;
            }
        }
        account.locked = true;
        match self.transactions.get_mut(&tx) {
            Some(record) => {
                record.disputed = false;
                record.burned = true;
            }
            None => return Err(RowError::UnknownTx),
        }
        Ok(())
    }
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new()
    }
}
