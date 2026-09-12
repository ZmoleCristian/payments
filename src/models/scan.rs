use crate::structs::scan::Scan;

impl Scan {
    pub fn reset(&mut self, first_line: u64) {
        self.in_quotes = false;
        self.field_start = true;
        self.just_closed = false;
        self.saw_any = false;
        self.bad = false;
        self.quoted_any = false;
        self.line = first_line;
    }
}
