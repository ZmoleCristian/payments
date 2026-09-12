#[derive(Default)]
pub struct Scan {
    pub in_quotes: bool,
    pub field_start: bool,
    pub just_closed: bool,
    pub saw_any: bool,
    pub bad: bool,
    pub quoted_any: bool,
    pub line: u64,
}
