use crate::structs::record::{FieldError, Record};

impl Record {
    pub fn reset(&mut self, first_line: u64) {
        self.bytes.clear();
        self.bounds.clear();
        self.bounds.push(0);
        self.width = 0;
        self.first_line = first_line;
        self.last_line = first_line;
    }

    pub fn close_field(&mut self) {
        let end = self.bytes.len();
        self.bounds.push(end);
        self.width += 1;
    }

    pub fn field(&self, index: usize) -> Result<&[u8], FieldError> {
        let Some(start) = self.bounds.get(index) else {
            return Err(FieldError::OutOfRange);
        };
        let Some(next) = index.checked_add(1) else {
            return Err(FieldError::OutOfRange);
        };
        let Some(end) = self.bounds.get(next) else {
            return Err(FieldError::OutOfRange);
        };
        let Some(slice) = self.bytes.get(*start..*end) else {
            return Err(FieldError::OutOfRange);
        };
        Ok(slice)
    }
}
