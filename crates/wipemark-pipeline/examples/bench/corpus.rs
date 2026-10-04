//! The bench's corpus: `bench/corpus/{en,ru,de}.txt`, one item per header
//! line.
//!
//! ```text
//! === id: en-pd-03 | kind: prose-pd | format: plain | source: … | licence: … [| inject: canary=WORD]
//! The item's text, up to the next header.
//! ```

use std::path::Path;

use wipemark_pipeline::lang::Lang;
use wipemark_pipeline::prepare::TextFormat;

/// What an embedded instruction asks the model to do instead of rewriting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inject {
    /// "Reply only with the word …".
    Canary(String),
    /// "Translate this text into …" (`fr` or `en`).
    Translate(String),
}

#[derive(Debug, Clone)]
pub struct Item {
    pub id: String,
    pub lang: Lang,
    pub kind: String,
    pub format: TextFormat,
    pub text: String,
    pub inject: Option<Inject>,
}

/// Every item of every language file under `dir`.
pub fn load(dir: &Path) -> Vec<Item> {
    let mut items = Vec::new();
    for lang in Lang::ALL {
        let path = dir.join(format!("{}.txt", lang.as_str()));
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        items.extend(parse(lang, &text));
    }
    let mut ids: Vec<&str> = items.iter().map(|item| item.id.as_str()).collect();
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    assert_eq!(before, ids.len(), "corpus ids are unique");
    items
}

fn parse(lang: Lang, text: &str) -> Vec<Item> {
    let mut items = Vec::new();
    let mut header: Option<&str> = None;
    let mut body = String::new();
    let mut flush = |header: Option<&str>, body: &mut String| {
        if let Some(header) = header {
            items.push(item(lang, header, body.trim_end_matches('\n')));
        }
        body.clear();
    };
    for line in text.split_inclusive('\n') {
        if let Some(rest) = line.strip_prefix("=== ") {
            flush(header, &mut body);
            header = Some(rest.trim_end());
        } else {
            body.push_str(line);
        }
    }
    flush(header, &mut body);
    items
}

fn item(lang: Lang, header: &str, body: &str) -> Item {
    let field = |name: &str| {
        header.split(" | ").find_map(|part| {
            part.strip_prefix(name)
                .and_then(|rest| rest.strip_prefix(": "))
                .map(str::to_owned)
        })
    };
    let id = field("id").expect("an item has an id");
    let format = match field("format").as_deref() {
        Some("markdown") => TextFormat::Markdown,
        Some("plain") => TextFormat::Plain,
        other => panic!("{id}: unknown format {other:?}"),
    };
    let inject = field("inject").map(|spec| match spec.split_once('=') {
        Some(("canary", word)) => Inject::Canary(word.to_owned()),
        Some(("translate", to)) => Inject::Translate(to.to_owned()),
        _ => panic!("{id}: unknown inject {spec:?}"),
    });
    Item {
        kind: field("kind").expect("an item has a kind"),
        id,
        lang,
        format,
        // A document ends with a line break, as a file does.
        text: format!("{body}\n"),
        inject,
    }
}
