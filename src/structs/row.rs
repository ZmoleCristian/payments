use crate::structs::ids::{ClientId, TxId};
use crate::structs::money::Money;

pub enum RowKind {
    Deposit { amount: Money },
    Withdrawal { amount: Money },
    Dispute,
    Resolve,
    Chargeback,
}

pub struct Row {
    pub kind: RowKind,
    pub client: ClientId,
    pub tx: TxId,
}
