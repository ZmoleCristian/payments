mod common;

use common::{drive, row_for};
use payments::run::run;
use std::io::{BufReader, Write};
use std::os::unix::net::UnixStream;

type Io<T> = Result<T, std::io::Error>;

const CSV: &str = "type,client,tx,amount\ndeposit,1,1,10.0\ndeposit,2,2,5.0\nwithdrawal,1,3,3.0\ndispute,2,2,\nwithdrawal,2,4,1.0\nchargeback,2,2,\ndeposit,1,5,0.0001\n";

fn over_socket(csv: &str, chunk: usize) -> Io<(String, String)> {
    let (mut writer, reader) = UnixStream::pair()?;
    let payload = csv.as_bytes().to_vec();
    let feeder = std::thread::spawn(move || -> Io<()> {
        for piece in payload.chunks(chunk) {
            writer.write_all(piece)?;
            writer.flush()?;
        }
        writer.shutdown(std::net::Shutdown::Write)
    });
    let mut out: Vec<u8> = Vec::new();
    let mut report: Vec<u8> = Vec::new();
    let outcome = run(BufReader::new(reader), &mut out, &mut report);
    match feeder.join() {
        Ok(fed) => fed?,
        Err(panic) => return Err(std::io::Error::other(format!("feeder panicked: {panic:?}"))),
    }
    match outcome {
        Ok(()) => {}
        Err(fatal) => return Err(std::io::Error::other(format!("run failed: {fatal}"))),
    }
    let stdout = String::from_utf8(out).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let stderr = String::from_utf8(report).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok((stdout, stderr))
}

#[test]
fn socket_stream_matches_the_in_memory_run_wcgw42() {
    let memory = drive(CSV).expect("in-memory run");
    let (stdout, stderr) = over_socket(CSV, 7).expect("socket run");
    assert_eq!(stdout, memory.stdout, "ledger differs between a socket and a buffer");
    assert_eq!(stderr, memory.stderr, "diagnostics differ between a socket and a buffer");
    assert_eq!(row_for(&stdout, 1), "1,7.0001,0.0000,7.0001,false", "client 1 over a socket");
    assert_eq!(row_for(&stdout, 2), "2,0.0000,0.0000,0.0000,true", "client 2 over a socket");
}

#[test]
fn record_split_across_reads_is_still_one_record_wcgw42() {
    let memory = drive(CSV).expect("in-memory run");
    for chunk in [1, 2, 3, 5, 13, 64] {
        let (stdout, stderr) = over_socket(CSV, chunk).expect("socket run");
        assert_eq!(stdout, memory.stdout, "ledger changed at chunk size {chunk}");
        assert_eq!(stderr, memory.stderr, "diagnostics changed at chunk size {chunk}");
    }
}
