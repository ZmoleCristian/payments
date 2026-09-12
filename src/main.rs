#![deny(unused_must_use)]
#![deny(for_loops_over_fallibles)]
#![deny(dead_code)]
#![deny(unused_variables)]
#![deny(unused_assignments)]

use payments::run::run;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.as_slice() {
        [_, path] => open_and_run(path),
        [program, ..] => {
            eprintln!("usage: {program} <transactions.csv>");
            ExitCode::from(2)
        }
        [] => std::process::exit(2),
    }
}

fn open_and_run(path: &str) -> ExitCode {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("{path}: {e}");
            std::process::exit(1)
        }
    };
    let input = BufReader::new(file);
    let mut out = BufWriter::new(std::io::stdout().lock());
    let mut report = std::io::stderr().lock();
    let result = run(input, &mut out, &mut report);
    let flushed = out.flush();
    match (result, flushed) {
        (Ok(()), Ok(())) => ExitCode::SUCCESS,
        (Err(e), _) => {
            eprintln!("{e}");
            std::process::exit(1)
        }
        (Ok(()), Err(e)) => {
            eprintln!("output flush failed: {e}");
            std::process::exit(1)
        }
    }
}
