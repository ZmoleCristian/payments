pub struct Columns {
    pub kind: usize,
    pub client: usize,
    pub tx: usize,
    pub amount: usize,
    pub width: usize,
}

pub struct Slot {
    pub seen: bool,
    pub index: usize,
}
