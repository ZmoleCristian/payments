use crate::structs::ids::ClientId;
use crate::structs::money::Money;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TxKind {
    Deposit,
    Withdrawal,
}

pub struct Transaction {
    pub client: ClientId,
    pub kind: TxKind,
    pub amount: Money,
    pub disputed: bool,
    pub burned: bool,
}
