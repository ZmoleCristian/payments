mod common;
use common::{drive, row_for};

#[test]
fn wcgw34_broken_output_sink_is_fatal_error() {
    struct DeadWriter;
    impl std::io::Write for DeadWriter {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "closed"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let reader = std::io::BufReader::new("type,client,tx,amount\ndeposit,1,1,1.0\n".as_bytes());
    let mut report: Vec<u8> = Vec::new();
    let mut dead = DeadWriter;
    let result = payments::run::run(reader, &mut dead, &mut report);
    match result {
        Ok(()) => panic!("broken output must be fatal"),
        Err(e) => {
            let text = format!("{e}");
            assert!(text.contains("io failure"), "error: {text}");
        }
    }
}

#[test]
fn wcgw37_row_errors_do_not_affect_run_success() {
    let o = drive("type,client,tx,amount\ngarbage line\ndeposit,1,1,1.0\nwithdrawal,1,2,999\n").expect("run must succeed despite bad rows");
    assert!(o.stderr.contains("line 2"), "stderr: {}", o.stderr);
    assert!(o.stderr.contains("line 4: insufficient funds"), "stderr: {}", o.stderr);
    assert_eq!(row_for(&o.stdout, 1), "1,1.0000,0.0000,1.0000,false");
}

#[test]
fn wcgw39_output_sorted_by_client_for_determinism() {
    let o = drive("type,client,tx,amount\ndeposit,9,1,1\ndeposit,1,2,1\ndeposit,5,3,1\n").expect("run");
    let mut lines = o.stdout.lines();
    let header = lines.next();
    assert_eq!(header, Some("client,available,held,total,locked"));
    let ids: Vec<&str> = lines.map(|l| l.split(',').next().unwrap_or("?")).collect();
    assert_eq!(ids, ["1", "5", "9"], "sorted: {ids:?}");
}

#[test]
fn wcgw40_overlong_line_capped_drained_reported() {
    let long_amount = "9".repeat(70000);
    let input = format!("type,client,tx,amount\ndeposit,1,1,{long_amount}\ndeposit,2,2,2.0\n");
    let o = drive(&input).expect("run");
    assert!(o.stderr.contains("line 2: malformed row"), "stderr: {}", o.stderr);
    assert_eq!(row_for(&o.stdout, 2), "2,2.0000,0.0000,2.0000,false");
}

#[test]
fn wcgw41_header_always_emitted_and_shape_exact() {
    let o = drive("type,client,tx,amount\n").expect("run");
    assert_eq!(o.stdout, "client,available,held,total,locked\n", "empty input still emits header");
}

#[test]
fn wcgw41_locked_lowercase_and_four_decimals() {
    let o = drive("type,client,tx,amount\ndeposit,1,1,2\ndispute,1,1,\nchargeback,1,1,\n").expect("run");
    assert_eq!(row_for(&o.stdout, 1), "1,0.0000,0.0000,0.0000,true");
}
