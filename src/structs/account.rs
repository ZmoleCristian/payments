use crate::structs::money::Money;

#[derive(Default)]
pub struct Account {
    pub available: Money,
    pub held: Money,
    pub locked: bool,
}
