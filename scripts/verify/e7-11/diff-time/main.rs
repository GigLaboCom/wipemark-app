// Times what the Compare window asks of diff.rs for two texts — `Diff::of`,
// `Diff::spans` at the words grain, `Diff::padding` — the median of five runs
// each. Run by ../diff-time.sh, which says what it is for.
#[allow(dead_code)]
mod diff {
    include!("diff_items.rs");
}
use std::time::Instant;

use diff::{Diff, Grain, Rows};

fn median(mut runs: Vec<f64>) -> f64 {
    runs.sort_by(f64::total_cmp);
    runs[runs.len() / 2]
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let original = std::fs::read_to_string(&args[1]).expect("the original");
    let result = std::fs::read_to_string(&args[2]).expect("the result");
    let time = |f: &mut dyn FnMut()| {
        median(
            (0..5)
                .map(|_| {
                    let start = Instant::now();
                    f();
                    start.elapsed().as_secs_f64() * 1000.0
                })
                .collect(),
        )
    };
    let mut diff = Diff::of(&original, &result);
    let of = time(&mut || diff = Diff::of(&original, &result));
    let spans = time(&mut || {
        std::hint::black_box(diff.spans(Grain::Words, &original, &result));
    });
    // Rows as a pane 100 columns wide would wrap them.
    let rows = |text: &str| -> Vec<usize> {
        text.split('\n')
            .map(|line| line.chars().count() / 100 + 1)
            .collect()
    };
    let (rows_a, rows_b) = (rows(&original), rows(&result));
    let padding = time(&mut || {
        std::hint::black_box(diff.padding(
            Rows {
                lines: rows_a.len(),
                rows: |i: usize| rows_a[i],
            },
            Rows {
                lines: rows_b.len(),
                rows: |i: usize| rows_b[i],
            },
        ));
    });
    println!(
        "{:>9} bytes  {:>5} lines  -{:<5} +{:<5}  of {:>9.2} ms  spans {:>9.2} ms  padding {:>7.3} ms",
        original.len(),
        rows_a.len(),
        diff.removed(),
        diff.added(),
        of,
        spans,
        padding
    );
}
