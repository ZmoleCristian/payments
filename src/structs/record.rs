#[derive(Default)]
pub struct Record {
    pub bytes: Vec<u8>,
    pub bounds: Vec<usize>,
    pub width: usize,
    pub first_line: u64,
    pub last_line: u64,
}

#[derive(Debug)]
pub enum FieldError {
    OutOfRange,
}
