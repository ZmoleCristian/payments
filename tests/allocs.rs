fn csv_of(rows: usize, row: &str) -> String {
    let mut csv = String::from("type,client,tx,amount\n");
    for i in 0..rows {
        csv.push_str(&row.replace("{i}", &i.to_string()).replace("{c}", &(i % 64).to_string()));
        csv.push('\n');
    }
    csv
}

fn allocations(csv: &str) -> Result<u64, payments::errors::FatalError> {
    let mut out: Vec<u8> = Vec::with_capacity(1 << 22);
    let mut report: Vec<u8> = Vec::with_capacity(1 << 22);
    let mut outcome: Result<(), payments::errors::FatalError> = Ok(());
    let info = allocation_counter::measure(|| {
        outcome = payments::run::run(std::io::BufReader::new(csv.as_bytes()), &mut out, &mut report);
    });
    match outcome {
        Ok(()) => Ok(info.count_total),
        Err(e) => Err(e),
    }
}

fn growth(row: &str) -> Result<u64, payments::errors::FatalError> {
    let small = allocations(&csv_of(1_000, row))?;
    let large = allocations(&csv_of(9_000, row))?;
    Ok(large.saturating_sub(small))
}

#[test]
fn rejected_rows_parse_and_report_without_allocating() {
    let extra = growth("dispute,{c},{i},").expect("run");
    assert_eq!(extra, 0, "8000 extra rejected rows cost {extra} allocations; parse and report must reuse their buffers");
}

#[test]
fn accepted_rows_allocate_only_the_transactions_map() {
    let extra = growth("deposit,{c},{i},1.0").expect("run");
    assert!(extra < 200, "8000 extra accepted rows cost {extra} allocations, beyond the transactions map's amortized doubling");
}

#[test]
fn quoted_rows_do_not_allocate_per_row() {
    let extra = growth("\"dispute\",\"{c}\",\"{i}\",").expect("run");
    assert_eq!(extra, 0, "8000 extra quoted rows cost {extra} allocations");
}
