use crate::structs::account::Account;
use crate::structs::money::Money;

impl Account {
    pub fn new() -> Account {
        Account {
            available: Money::zero(),
            held: Money::zero(),
            locked: false,
        }
    }
}

impl Default for Account {
    fn default() -> Self {
        Self::new()
    }
}
