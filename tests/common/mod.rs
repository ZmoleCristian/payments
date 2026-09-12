use payments::errors::FatalError;
use payments::run::run;
use std::io::BufReader;
use std::string::FromUtf8Error;

pub(crate) struct Outcome {
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

pub(crate) fn drive(input: &str) -> Result<Outcome, FatalError> {
    let reader = BufReader::new(input.as_bytes());
    let mut out: Vec<u8> = Vec::new();
    let mut report: Vec<u8> = Vec::new();
    run(reader, &mut out, &mut report)?;
    let stdout = utf8(out, "stdout")?;
    let stderr = utf8(report, "stderr")?;
    Ok(Outcome { stdout, stderr })
}

fn utf8(bytes: Vec<u8>, which: &str) -> Result<String, FatalError> {
    match String::from_utf8(bytes) {
        Ok(s) => Ok(s),
        Err(e) => Err(bad_bytes(which, e)),
    }
}

fn bad_bytes(which: &str, e: FromUtf8Error) -> FatalError {
    let io = std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{which} not utf-8: {e}"));
    FatalError::Io(io)
}

pub(crate) fn row_for(stdout: &str, client: u16) -> String {
    let needle = format!("{client},");
    let mut found = String::new();
    for line in stdout.lines() {
        if line.starts_with(&needle) {
            found = line.to_string();
        }
    }
    assert!(!found.is_empty(), "no output row for client {client} in:\n{stdout}");
    found
}
