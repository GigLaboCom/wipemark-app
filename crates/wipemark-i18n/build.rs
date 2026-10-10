//! Generates the [`Message`] enum from the fallback catalogue.
//!
//! Two jobs, both of them gates:
//!
//! 1. **Every catalogue is parsed.** A `.ftl` with a syntax error fails
//!    `cargo build`, not `cargo test` — Fluent's parser recovers from
//!    junk by dropping the entry, so an unparseable line would otherwise
//!    surface as one message silently missing from one language.
//! 2. **The fallback catalogue becomes Rust.** Every message id in
//!    `i18n/en-US/wipemark.ftl` gets an enum variant, so a caller cannot
//!    name a key that does not exist. That is the property `fl!`-style
//!    macros buy with a proc macro and a config file; here it is thirty
//!    lines of codegen and the same idiom `wipemark-core` already uses
//!    to turn committed UCD text into static tables.
//!
//! The generated file also records, per message, which `$variables` its
//! pattern references. `Message::variables` is what lets a test call
//! every message with the right arguments without a hand-written table
//! that would drift.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

use fluent_syntax::{ast, parser};

/// The language every other one falls back to, and the one the enum is
/// generated from. Emitted as a constant so the runtime cannot disagree
/// with the code generator about which directory that is.
const FALLBACK: &str = "en-US";

/// Every catalogue is `<language>/CATALOGUE`. One file per language
/// keeps a translator's unit of work equal to a file.
const CATALOGUE: &str = "wipemark.ftl";

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let i18n = root.join("i18n");
    println!("cargo:rerun-if-changed={}", i18n.display());
    println!("cargo:rerun-if-changed=build.rs");

    let mut languages: Vec<String> = fs::read_dir(&i18n)
        .unwrap_or_else(|error| panic!("reading {}: {error}", i18n.display()))
        .filter_map(|entry| {
            let entry = entry.expect("directory entry");
            entry
                .file_type()
                .expect("file type")
                .is_dir()
                .then(|| entry.file_name().to_string_lossy().into_owned())
        })
        .collect();
    languages.sort();

    assert!(
        languages.iter().any(|language| language == FALLBACK),
        "i18n/{FALLBACK}/ is the fallback catalogue and must exist"
    );

    // Job 1: everything parses. Done for every language, including the
    // fallback, before anything is generated.
    for language in &languages {
        let path = i18n.join(language).join(CATALOGUE);
        println!("cargo:rerun-if-changed={}", path.display());
        parse(&path);
    }

    // Job 2: the fallback becomes Rust.
    let fallback = parse(&i18n.join(FALLBACK).join(CATALOGUE));
    let generated = generate(&fallback, &languages);

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("messages.rs");
    fs::write(&out, generated).unwrap_or_else(|error| panic!("writing {}: {error}", out.display()));
}

/// Parse one catalogue, turning Fluent's error recovery into a build
/// failure. `parse` hands back a resource *and* the errors it recovered
/// from; taking the resource and dropping the errors is how a typo
/// becomes an invisible missing string.
fn parse(path: &Path) -> ast::Resource<String> {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    match parser::parse(source) {
        Ok(resource) => resource,
        Err((_, errors)) => {
            let mut report = format!("{} is not valid Fluent:\n", path.display());
            for error in errors {
                let _ = writeln!(report, "  - {error:?}");
            }
            panic!("{report}");
        }
    }
}

fn generate(fallback: &ast::Resource<String>, languages: &[String]) -> String {
    let messages: Vec<(String, String, Vec<String>, Option<String>)> = fallback
        .body
        .iter()
        .filter_map(|entry| match entry {
            // Terms (`-brand-name`) are referenced by other messages and
            // never shown on their own, so they get no variant: nothing
            // may ask for one by hand.
            ast::Entry::Message(message) => Some(message),
            _ => None,
        })
        .map(|message| {
            let id = message.id.name.clone();
            let mut variables = BTreeSet::new();
            if let Some(pattern) = &message.value {
                collect_variables(pattern, &mut variables);
            }
            for attribute in &message.attributes {
                collect_variables(&attribute.value, &mut variables);
            }
            let preview = message.value.as_ref().map(preview);
            let comment = message
                .comment
                .as_ref()
                .map(|comment| comment.content.join(" "));
            (
                id,
                comment.unwrap_or_default(),
                variables.into_iter().collect(),
                preview,
            )
        })
        .collect();

    assert!(!messages.is_empty(), "the fallback catalogue is empty");

    let mut out = String::new();
    out.push_str(
        "// @generated by build.rs from i18n/en-US/wipemark.ftl. Do not edit.\n\
         //\n\
         // Regenerate by editing the catalogue: the enum is downstream of it,\n\
         // never the other way round.\n\n",
    );

    let _ = writeln!(
        out,
        "/// The language the enum is generated from, and the last link \
         of every\n/// fallback chain.\npub const FALLBACK_LANGUAGE: &str = {FALLBACK:?};\n"
    );

    let list = languages
        .iter()
        .map(|language| format!("{language:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(
        out,
        "/// Every catalogue directory that shipped in this binary, sorted.\n\
         /// The embedded assets are the runtime source of truth; this is what\n\
         /// build time saw, and `catalogue_matches_the_build` compares them.\n\
         pub const BUILT_LANGUAGES: [&str; {}] = [{list}];\n",
        languages.len()
    );

    out.push_str(
        "/// One message in the catalogue.\n\
         ///\n\
         /// Generated, so a key that is not in `i18n/en-US/wipemark.ftl` does\n\
         /// not compile. Translations may lag; the enum may not.\n\
         #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]\n\
         #[non_exhaustive]\n\
         pub enum Message {\n",
    );
    for (id, comment, variables, preview) in &messages {
        if !comment.is_empty() {
            let _ = writeln!(out, "    /// {}", escape_doc(comment));
            out.push_str("    ///\n");
        }
        if let Some(preview) = preview {
            let _ = writeln!(out, "    /// en-US: `{}`", escape_doc(preview));
        }
        if !variables.is_empty() {
            let _ = writeln!(
                out,
                "    ///\n    /// Variables: {}",
                variables
                    .iter()
                    .map(|variable| format!("`${variable}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        let _ = writeln!(out, "    {},", variant(id));
    }
    out.push_str("}\n\n");

    out.push_str("impl Message {\n");
    let _ = writeln!(
        out,
        "    /// Every message, in catalogue order. Iterated by the tests \
         that\n    /// prove each language can render all of them.\n    \
         pub const ALL: [Message; {}] = [",
        messages.len()
    );
    for (id, ..) in &messages {
        let _ = writeln!(out, "        Message::{},", variant(id));
    }
    out.push_str("    ];\n\n");

    out.push_str(
        "    /// The Fluent message id. This is the wire between the enum and\n\
         \x20   /// the catalogue, and the only string form a message has.\n\
         \x20   pub const fn id(self) -> &'static str {\n        match self {\n",
    );
    for (id, ..) in &messages {
        let _ = writeln!(out, "            Message::{} => {id:?},", variant(id));
    }
    out.push_str("        }\n    }\n\n");

    out.push_str(
        "    /// The `$variables` the fallback pattern references, sorted.\n\
         \x20   ///\n\
         \x20   /// A caller that passes none of these still renders — Fluent\n\
         \x20   /// prints the variable name and reports an error — so this is\n\
         \x20   /// what lets the suite call every message correctly rather\n\
         \x20   /// than eyeballing the catalogue.\n\
         \x20   pub const fn variables(self) -> &'static [&'static str] {\n        match self {\n",
    );
    for (id, _, variables, _) in &messages {
        let list = variables
            .iter()
            .map(|variable| format!("{variable:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(out, "            Message::{} => &[{list}],", variant(id));
    }
    out.push_str("        }\n    }\n}\n");

    out
}

/// `panel-title` -> `PanelTitle`.
fn variant(id: &str) -> String {
    id.split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect()
}

/// The literal text of a pattern, for the rustdoc line. Placeables are
/// shown as their source spelling rather than resolved — the point is
/// to recognise the string, not to render it.
fn preview(pattern: &ast::Pattern<String>) -> String {
    let mut out = String::new();
    for element in &pattern.elements {
        match element {
            ast::PatternElement::TextElement { value } => out.push_str(value),
            ast::PatternElement::Placeable { .. } => out.push_str("{ … }"),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Doc comments end up in rustdoc, so a stray backtick or newline in a
/// translator note must not break the block.
fn escape_doc(text: &str) -> String {
    text.replace('\n', " ").replace('`', "'")
}

fn collect_variables(pattern: &ast::Pattern<String>, found: &mut BTreeSet<String>) {
    for element in &pattern.elements {
        if let ast::PatternElement::Placeable { expression } = element {
            collect_from_expression(expression, found);
        }
    }
}

fn collect_from_expression(expression: &ast::Expression<String>, found: &mut BTreeSet<String>) {
    match expression {
        ast::Expression::Inline(inline) => collect_from_inline(inline, found),
        ast::Expression::Select { selector, variants } => {
            collect_from_inline(selector, found);
            for variant in variants {
                collect_variables(&variant.value, found);
            }
        }
    }
}

fn collect_from_inline(inline: &ast::InlineExpression<String>, found: &mut BTreeSet<String>) {
    match inline {
        ast::InlineExpression::VariableReference { id } => {
            found.insert(id.name.clone());
        }
        ast::InlineExpression::Placeable { expression } => {
            collect_from_expression(expression, found);
        }
        ast::InlineExpression::FunctionReference { arguments, .. } => {
            for positional in &arguments.positional {
                collect_from_inline(positional, found);
            }
            for named in &arguments.named {
                collect_from_inline(&named.value, found);
            }
        }
        ast::InlineExpression::TermReference {
            arguments: Some(arguments),
            ..
        } => {
            for positional in &arguments.positional {
                collect_from_inline(positional, found);
            }
            for named in &arguments.named {
                collect_from_inline(&named.value, found);
            }
        }
        ast::InlineExpression::StringLiteral { .. }
        | ast::InlineExpression::NumberLiteral { .. }
        | ast::InlineExpression::MessageReference { .. }
        | ast::InlineExpression::TermReference { .. } => {}
    }
}
