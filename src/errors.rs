use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io;

#[derive(Debug)]
pub enum RowError {
    Malformed,
    UnknownType,
    BadClient(String),
    BadTx(String),
    BadAmount,
    DuplicateTx,
    UnknownTx,
    NotDisputed,
    AlreadyDisputed,
    InsufficientFunds,
    AccountLocked,
    WithdrawalDispute,
}

#[derive(Debug)]
pub enum FatalError {
    Io(io::Error),
    Report(io::Error),
    Corrupt(RowError),
    Header,
    UnterminatedQuote,
}

impl Display for RowError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            RowError::Malformed => write!(f, "malformed row"),
            RowError::UnknownType => write!(f, "unknown transaction type"),
            RowError::BadClient(text) => write!(f, "invalid client id: {text}"),
            RowError::BadTx(text) => write!(f, "invalid transaction id: {text}"),
            RowError::BadAmount => write!(f, "invalid amount"),
            RowError::DuplicateTx => write!(f, "duplicate transaction id"),
            RowError::UnknownTx => write!(f, "unknown transaction id"),
            RowError::NotDisputed => write!(f, "transaction not under dispute"),
            RowError::AlreadyDisputed => write!(f, "transaction already under dispute"),
            RowError::InsufficientFunds => write!(f, "insufficient funds"),
            RowError::AccountLocked => write!(f, "account locked"),
            RowError::WithdrawalDispute => write!(f, "withdrawals cannot be disputed"),
        }
    }
}

impl Error for RowError {}

impl Display for FatalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            FatalError::Io(e) => write!(f, "io failure: {e}"),
            FatalError::Report(e) => write!(f, "report sink failure: {e}"),
            FatalError::Corrupt(e) => write!(f, "corrupt ledger state: {e}"),
            FatalError::Header => write!(f, "missing or invalid header: expected columns type,client,tx,amount in any order"),
            FatalError::UnterminatedQuote => write!(f, "broken file: quoted field opened but never closed"),
        }
    }
}

impl Error for FatalError {}

impl From<io::Error> for FatalError {
    fn from(e: io::Error) -> Self {
        FatalError::Io(e)
    }
}

impl From<RowError> for FatalError {
    fn from(e: RowError) -> Self {
        FatalError::Corrupt(e)
    }
}
