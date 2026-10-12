// The Compare window's two panes as text rows: `Diff::padding` and
// `Diff::spans` from apps/wipemark-app/src/diff.rs itself, lines wrapped at a
// fixed width — a picture of the pairing and the word marks, not of the fonts.
// Run by ../pairing.sh, which says what it is for.
#[allow(dead_code)]
mod diff {
    include!("diff_items.rs");
    pub fn hunks_of(d: &Diff) -> Vec<(Range<usize>, Range<usize>)> {
        d.hunks.iter().map(|h| (h.removed.clone(), h.added.clone())).collect()
    }
}
use diff::{Diff, Rows};

const W: usize = 30;

fn wrap(line: &str) -> Vec<String> {
    let mut rows = vec![String::new()];
    for word in line.split(' ') {
        let last = rows.last_mut().unwrap();
        if !last.is_empty() && last.len() + 1 + word.len() > W {
            rows.push(word.to_string());
        } else {
            if !last.is_empty() { last.push(' '); }
            last.push_str(word);
        }
    }
    rows
}

fn side(text: &str, pads: &[diff::Pad], changed: &dyn Fn(usize) -> bool, mark: char) -> Vec<String> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out = Vec::new();
    let pad_rows = |out: &mut Vec<String>, pad: &diff::Pad| {
        for _ in 0..pad.rows {
            out.push(if pad.filler { format!("    {}", "░".repeat(W)) } else { format!("    {:W$}", "·") });
        }
    };
    for (i, line) in lines.iter().enumerate() {
        for pad in pads.iter().filter(|p| p.line == i && p.above) { pad_rows(&mut out, pad); }
        for (k, row) in wrap(line).into_iter().enumerate() {
            let m = if changed(i) { mark } else { ' ' };
            let num = if k == 0 { format!("{:>2}", i + 1) } else { "  ".into() };
            out.push(format!("{m}{num} {row:W$}"));
        }
        for pad in pads.iter().filter(|p| p.line == i && !p.above) { pad_rows(&mut out, pad); }
    }
    out
}

fn show(title: &str, a: &[&str], b: &[&str]) {
    let (a, b) = (a.join("\n") + "\n", b.join("\n") + "\n");
    let d = Diff::of(&a, &b);
    let hunks = diff::hunks_of(&d);
    let la: Vec<&str> = a.split('\n').collect();
    let lb: Vec<&str> = b.split('\n').collect();
    let padding = d.padding(
        Rows { lines: la.len(), rows: |i: usize| wrap(la[i]).len() },
        Rows { lines: lb.len(), rows: |i: usize| wrap(lb[i]).len() },
    );
    let left = side(&a, &padding.original, &|i| hunks.iter().any(|(r, _)| r.contains(&i)), '-');
    let right = side(&b, &padding.result, &|i| hunks.iter().any(|(_, r)| r.contains(&i)), '+');
    println!("\n=== {title}");
    let sp = d.spans(diff::Grain::Words, &a, &b);
    println!("  dark on the left:  {:?}", sp.removed.iter().map(|r| &a[r.clone()]).collect::<Vec<_>>());
    println!("  dark on the right: {:?}", sp.added.iter().map(|r| &b[r.clone()]).collect::<Vec<_>>());
    println!("{:<w$}  │ {}", "  ORIGINAL", "  RESULT", w = W + 4);
    for k in 0..left.len().max(right.len()) {
        let l = left.get(k).cloned().unwrap_or_default();
        let r = right.get(k).cloned().unwrap_or_default();
        println!("{:<w$}  │ {}", l, r, w = W + 4);
    }
}

fn main() {
    show("0. the owner's example, words within one line", &["This is black fox."], &["This is a red fox."]);
    let s1 = "The harbour was quiet in the early hours.";
    let x = "A gull walked the length of the quay as if it owned it.";
    let s2 = "Nobody spoke much; the work was known.";
    let a = "Fishing boats came in one by one and tied up along the old wall.";
    let a2 = "Fishing boats came in one by one and tied up along the stone wall.";
    let b = "By six the first crates were stacked on the quay.";
    let b2 = "By six the first crates stood stacked on the quay, ice between them.";
    let c = "Then the bell rang and the harbour woke.";
    let c2 = "Then the bell rang, and the whole harbour woke at once.";
    show("1. a paragraph deleted between unchanged ones", &[s1, x, s2], &[s1, s2]);
    show("2. deleted LAST in a rewritten block (C gone, A and B rewritten)", &[s1, a, b, c, s2], &[s1, a2, b2, s2]);
    show("3. deleted FIRST in a rewritten block (A gone, B and C rewritten)", &[s1, a, b, c, s2], &[s1, b2, c2, s2]);
}
