use std::{env, time::Instant};

fn main() {
    let dir = env::args().nth(1).expect("Usage: bench-analyze <path>");

    let start = Instant::now();
    let (result, _changes) = edgerun_codelyzer::analyzer::analyze_full(&dir);
    let elapsed = start.elapsed();

    let funcs = result.program.functions.len();
    let edges = result.program.edges.len();
    let files = result
        .program
        .functions
        .values()
        .map(|f| &f.file)
        .collect::<std::collections::HashSet<_>>()
        .len();

    println!("Analyzed: {dir}");
    println!("  Files scanned:    {}", result.file_snapshot.len());
    println!("  Source files:     {files}");
    println!("  Functions:        {funcs}");
    println!("  Edges (calls):    {edges}");
    println!(
        "  Changes:          {} added, {} edges",
        _changes.added.len(),
        _changes.edges_added.len()
    );
    println!("  Total time:       {:.3}s", elapsed.as_secs_f64());
    println!(
        "  Per file avg:     {:.2}ms",
        elapsed.as_millis() as f64 / result.file_snapshot.len().max(1) as f64
    );
    if funcs > 0 {
        println!("  Edges per func:   {:.1}", edges as f64 / funcs as f64);
    }

    // Top 10 functions by edge count
    let mut conn: std::collections::HashMap<&String, usize> = std::collections::HashMap::new();
    for e in &result.program.edges {
        *conn.entry(&e.caller).or_default() += 1;
    }
    let mut sorted: Vec<_> = conn.iter().collect();
    sorted.sort_by_key(|(_, &c)| -(c as isize));

    println!("\nTop 10 most-connected functions:");
    for (id, &count) in sorted.iter().take(10) {
        let short = id.split("::").last().unwrap_or(id);
        println!("  {count:>4}  {short}");
    }
}
