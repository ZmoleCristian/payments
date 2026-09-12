use std::io::Write;
use std::process::{Command, Stdio};

type Io<T> = Result<T, std::io::Error>;

fn binary_path() -> Io<std::path::PathBuf> {
    let mut path = std::env::current_exe()?;
    path.pop();
    path.pop();
    path.push("payments");
    Ok(path)
}

fn run_binary(args: &[&str], stdin: &str) -> Io<(i32, String, String)> {
    let mut child = Command::new(binary_path()?).args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let mut input = match child.stdin.take() {
        Some(pipe) => pipe,
        None => return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "stdin pipe missing")),
    };
    let payload = stdin.to_string();
    let writer = std::thread::spawn(move || input.write_all(payload.as_bytes()));
    let output = child.wait_with_output()?;
    match writer.join() {
        Ok(done) => done?,
        Err(panic) => return Err(std::io::Error::other(format!("writer panicked: {panic:?}"))),
    }
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8(output.stdout).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let stderr = String::from_utf8(output.stderr).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok((code, stdout, stderr))
}

fn temp_csv(name: &str, contents: &str) -> Io<std::path::PathBuf> {
    let mut path = std::env::temp_dir();
    path.push(format!("payments-binary-test-{}-{}", std::process::id(), name));
    let mut file = std::fs::File::create(&path)?;
    file.write_all(contents.as_bytes())?;
    Ok(path)
}

#[test]
fn binary_processes_file_to_stdout() {
    let csv = temp_csv("basic.csv", "type,client,tx,amount\ndeposit,1,1,1.0\ndeposit,2,2,2.0\nwithdrawal,1,3,0.5\n").expect("temp csv");
    let path = csv.to_str().expect("utf8 path");
    let (code, stdout, stderr) = run_binary(&[path], "").expect("run binary");
    std::fs::remove_file(&csv).expect("cleanup");
    assert_eq!(code, 0, "exit code, stderr: {stderr}");
    assert!(stdout.starts_with("client,available,held,total,locked\n"), "header: {stdout}");
    assert!(stdout.contains("1,0.5000,0.0000,0.5000,false"), "client 1: {stdout}");
    assert!(stdout.contains("2,2.0000,0.0000,2.0000,false"), "client 2: {stdout}");
}

#[test]
fn binary_missing_file_exits_nonzero_with_message() {
    let (code, stdout, stderr) = run_binary(&["/nonexistent/path/to/nowhere.csv"], "").expect("run binary");
    assert_eq!(code, 1, "exit code");
    assert!(stdout.is_empty(), "no stdout on fatal: {stdout}");
    assert!(stderr.contains("nowhere.csv"), "error names the path: {stderr}");
}

#[test]
fn binary_no_args_prints_usage_exit_two() {
    let (code, _stdout, stderr) = run_binary(&[], "").expect("run binary");
    assert_eq!(code, 2, "exit code");
    assert!(stderr.contains("usage:"), "usage line: {stderr}");
}

#[test]
fn binary_extra_args_prints_usage_exit_two() {
    let (code, _stdout, stderr) = run_binary(&["a.csv", "b.csv"], "").expect("run binary");
    assert_eq!(code, 2, "exit code");
    assert!(stderr.contains("usage:"), "usage line: {stderr}");
}

#[test]
fn binary_row_errors_on_stderr_exit_zero() {
    let csv = temp_csv("badrows.csv", "type,client,tx,amount\ndeposit,1,1,10\nwithdrawal,1,2,999\ngarbage\n").expect("temp csv");
    let path = csv.to_str().expect("utf8 path");
    let (code, stdout, stderr) = run_binary(&[path], "").expect("run binary");
    std::fs::remove_file(&csv).expect("cleanup");
    assert_eq!(code, 0, "row errors never touch exit code");
    assert!(stderr.contains("line 3: insufficient funds"), "stderr: {stderr}");
    assert!(stderr.contains("line 4"), "stderr: {stderr}");
    assert!(stdout.contains("1,10.0000,0.0000,10.0000,false"), "stdout: {stdout}");
}
