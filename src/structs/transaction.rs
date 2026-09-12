use crate::structs::ids::ClientId;
use crate::structs::money::Money;

pub struct Transaction {
    pub client: ClientId,
    pub amount: Money,
    pub disputed: bool,
}
