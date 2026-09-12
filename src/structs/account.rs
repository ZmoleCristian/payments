use crate::structs::money::Money;

pub struct Account {
    pub available: Money,
    pub held: Money,
    pub locked: bool,
}
