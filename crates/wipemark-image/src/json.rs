//! The JSON form of both reports, written by this crate — one writer for
//! every surface, as `wipemark_core`'s is for text (D9): the CLI prints it
//! for `--json` and inside `audit --json`, the MCP server hands it to a
//! client. `std` alone, and small, because a report carries no picture and
//! no value: only ids, block names, offsets and the signatures that
//! matched.
//!
//! Three properties are the writer's, not the data's:
//!
//! * **The third shelf is always there.** `not_established` is written
//!   from `wipemark_core::report::not_established::ALL` as the last key of
//!   both forms, so no surface can forget it — and it always carries
//!   [`crate::PIXEL_DOMAIN`].
//! * **The output is ASCII, and nothing read out of a file is said back
//!   as it was.** A PNG text keyword is Latin-1 and may hold a soft
//!   hyphen or a no-break space — characters this product removes — so
//!   every string that came out of the file (`chunk`, `key`, `field`) goes
//!   through [`spell`] first: printable ASCII as itself, anything else as
//!   `U+XXXX`. The MCP server renders this report into a response a model
//!   reads, and a report must not carry the marks it reports.
//! * **One line, keys in a fixed order**, no whitespace outside strings.

use std::fmt::Write as _;

use wipemark_core::report::not_established;

use crate::{Evidence, ImageReport, MetadataFinding, Signal, StripReport};

impl ImageReport {
    /// One line of ASCII JSON: `container`, `ai_metadata`, `c2pa`,
    /// `findings`, `not_established`. Machines read it; nothing in it is
    /// translated.
    pub fn to_json(&self) -> String {
        let mut out = String::with_capacity(256);
        out.push_str("{\"container\":");
        push_string(&mut out, self.container.id());
        out.push_str(",\"ai_metadata\":");
        push_bool(&mut out, self.has_ai_metadata());
        out.push_str(",\"c2pa\":");
        push_bool(&mut out, self.has_c2pa());
        out.push(',');
        findings(&mut out, "findings", &self.findings);
        tail(&mut out);
        out
    }
}

impl StripReport {
    /// As [`ImageReport::to_json`], with what the output still carries
    /// first: `container`, `still_has_ai_metadata`, `still_has_c2pa`,
    /// `removed` (offsets in the input), `kept` (offsets in the output),
    /// `orientation_removed` (the EXIF value, 2–8, or `null`),
    /// `not_established`.
    pub fn to_json(&self) -> String {
        let mut out = String::with_capacity(256);
        out.push_str("{\"container\":");
        push_string(&mut out, self.container.id());
        out.push_str(",\"still_has_ai_metadata\":");
        push_bool(&mut out, self.still_has_ai_metadata);
        out.push_str(",\"still_has_c2pa\":");
        push_bool(&mut out, self.still_has_c2pa);
        out.push(',');
        findings(&mut out, "removed", &self.removed);
        out.push(',');
        findings(&mut out, "kept", &self.kept);
        out.push_str(",\"orientation_removed\":");
        match self.orientation_removed {
            Some(value) => {
                let _ = write!(out, "{value}");
            }
            None => out.push_str("null"),
        }
        tail(&mut out);
        out
    }
}

/// A string read out of a file, safe to say back: printable ASCII as
/// itself, anything else — a control, a Latin-1 soft hyphen, a no-break
/// space — as `U+XXXX`. The human report spells a keyword the same way,
/// so a terminal and a parser are shown one thing.
pub fn spell(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        if character == ' ' || character.is_ascii_graphic() {
            out.push(character);
        } else {
            let _ = write!(out, "U+{:04X}", u32::from(character));
        }
    }
    out
}

fn findings(out: &mut String, key: &str, rows: &[MetadataFinding]) {
    push_string(out, key);
    out.push_str(":[");
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("{\"kind\":");
        push_string(out, row.kind.id());
        out.push_str(",\"chunk\":");
        push_string(out, &spell(&row.chunk));
        out.push_str(",\"key\":");
        match &row.key {
            Some(key) => push_string(out, &spell(key)),
            None => out.push_str("null"),
        }
        let _ = write!(
            out,
            ",\"offset\":{},\"length\":{},\"ai\":",
            row.offset, row.len
        );
        push_bool(out, row.is_ai_provenance());
        out.push_str(",\"c2pa\":");
        push_bool(out, row.is_c2pa());
        out.push_str(",\"evidence\":[");
        for (j, evidence) in row.evidence.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            push_evidence(out, evidence);
        }
        out.push_str("]}");
    }
    out.push(']');
}

fn push_evidence(out: &mut String, evidence: &Evidence) {
    out.push_str("{\"signal\":");
    push_string(out, evidence.signal.id());
    out.push_str(",\"generator\":");
    match evidence.signal {
        Signal::GeneratorKey(generator) | Signal::GeneratorText(generator) => {
            push_string(out, generator.id());
        }
        _ => out.push_str("null"),
    }
    out.push_str(",\"source_type\":");
    match evidence.signal {
        Signal::DigitalSourceType(source) => push_string(out, source.code()),
        _ => out.push_str("null"),
    }
    out.push_str(",\"field\":");
    push_string(out, &spell(&evidence.field));
    out.push_str(",\"matched\":");
    push_string(out, &spell(evidence.matched));
    out.push('}');
}

/// The third shelf, read from core's constant and never re-typed, then
/// the closing brace.
fn tail(out: &mut String) {
    out.push_str(",\"not_established\":[");
    for (i, (id, _)) in not_established::ALL.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_string(out, id);
    }
    out.push_str("]}");
}

fn push_bool(out: &mut String, value: bool) {
    out.push_str(if value { "true" } else { "false" });
}

/// A JSON string of something already ASCII: only `"` and `\` need an
/// escape. Anything else is escaped as `\u` too, so the output stays ASCII
/// whatever a later caller hands this.
fn push_string(out: &mut String, value: &str) {
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if !(' '..'\u{7F}').contains(&c) => {
                for unit in c.encode_utf16(&mut [0u16; 2]) {
                    let _ = write!(out, "\\u{unit:04x}");
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::spell;
    use crate::{
        Evidence, Generator, ImageContainer, ImageReport, MetadataFinding, MetadataKind, Signal,
        SourceType, StripReport, PIXEL_DOMAIN,
    };

    fn finding(kind: MetadataKind, key: &str, evidence: Vec<Evidence>) -> MetadataFinding {
        MetadataFinding {
            kind,
            chunk: "tEXt".into(),
            key: Some(key.into()),
            offset: 33,
            len: 40,
            evidence,
        }
    }

    /// The exact form, so a surface that splices it — `audit --json`, the
    /// MCP server — is splicing what this test says.
    #[test]
    fn the_json_form_of_an_image_report_is_exact() {
        let report = ImageReport {
            container: ImageContainer::Png,
            findings: vec![finding(
                MetadataKind::GeneratorParameters,
                "parameters",
                vec![
                    Evidence {
                        signal: Signal::GeneratorKey(Generator::StableDiffusionWebUi),
                        field: "parameters".into(),
                        matched: "parameters",
                    },
                    Evidence {
                        signal: Signal::DigitalSourceType(SourceType::TrainedAlgorithmicMedia),
                        field: "parameters".into(),
                        matched: "trainedAlgorithmicMedia",
                    },
                ],
            )],
            not_established: Vec::new(),
        };
        assert_eq!(
            report.to_json(),
            "{\"container\":\"png\",\"ai_metadata\":true,\"c2pa\":false,\"findings\":[\
             {\"kind\":\"generator-parameters\",\"chunk\":\"tEXt\",\"key\":\"parameters\",\
             \"offset\":33,\"length\":40,\"ai\":true,\"c2pa\":false,\"evidence\":[\
             {\"signal\":\"generator-key\",\"generator\":\"stable-diffusion-webui\",\
             \"source_type\":null,\"field\":\"parameters\",\"matched\":\"parameters\"},\
             {\"signal\":\"digital-source-type\",\"generator\":null,\
             \"source_type\":\"trainedAlgorithmicMedia\",\"field\":\"parameters\",\
             \"matched\":\"trainedAlgorithmicMedia\"}]}],\
             \"not_established\":[\"vendor-detector-evasion\",\"human-authorship\",\
             \"unknown-mark-schemes\"]}"
        );
    }

    /// The writer's own property: the third shelf is written from the
    /// constant, whatever the report's field holds — an empty field above,
    /// and the pixel domain on the shelf anyway.
    #[test]
    fn the_third_shelf_is_written_whatever_the_field_says() {
        let empty = StripReport {
            container: ImageContainer::Jpeg,
            removed: Vec::new(),
            kept: Vec::new(),
            still_has_c2pa: false,
            still_has_ai_metadata: false,
            orientation_removed: None,
            not_established: Vec::new(),
        };
        let json = empty.to_json();
        assert!(json.ends_with(&format!(
            "\"not_established\":[\"vendor-detector-evasion\",\"human-authorship\",\"{PIXEL_DOMAIN}\"]}}"
        )));
        assert!(json.starts_with(
            "{\"container\":\"jpeg\",\"still_has_ai_metadata\":false,\"still_has_c2pa\":false,"
        ));
    }

    /// The exact form of a strip, with the rotation that went with an
    /// EXIF block — and `null` when none did, so the key is always there.
    #[test]
    fn the_json_form_of_a_strip_report_is_exact() {
        let mut report = StripReport {
            container: ImageContainer::Jpeg,
            removed: vec![MetadataFinding {
                kind: MetadataKind::Exif,
                chunk: "APP1".into(),
                key: Some("Exif".into()),
                offset: 2,
                len: 60,
                evidence: Vec::new(),
            }],
            kept: Vec::new(),
            still_has_c2pa: false,
            still_has_ai_metadata: false,
            orientation_removed: Some(6),
            not_established: Vec::new(),
        };
        assert_eq!(
            report.to_json(),
            "{\"container\":\"jpeg\",\"still_has_ai_metadata\":false,\
             \"still_has_c2pa\":false,\"removed\":[{\"kind\":\"exif\",\"chunk\":\"APP1\",\
             \"key\":\"Exif\",\"offset\":2,\"length\":60,\"ai\":false,\"c2pa\":false,\
             \"evidence\":[]}],\"kept\":[],\"orientation_removed\":6,\
             \"not_established\":[\"vendor-detector-evasion\",\"human-authorship\",\
             \"unknown-mark-schemes\"]}"
        );
        report.orientation_removed = None;
        assert!(report
            .to_json()
            .contains("\"kept\":[],\"orientation_removed\":null,\"not_established\""));
    }

    /// A keyword is Latin-1 and is the file's, not ours: a soft hyphen and
    /// a no-break space in it come back spelled, and the whole line is
    /// ASCII.
    #[test]
    fn a_keyword_read_out_of_the_file_is_spelled() {
        let report = ImageReport {
            container: ImageContainer::Png,
            findings: vec![finding(
                MetadataKind::OtherText,
                "Co\u{AD}mment\u{A0}\"x\\",
                Vec::new(),
            )],
            not_established: Vec::new(),
        };
        let json = report.to_json();
        assert!(json.is_ascii(), "{json}");
        assert!(
            json.contains("\"key\":\"CoU+00ADmmentU+00A0\\\"x\\\\\""),
            "{json}"
        );
        assert_eq!(spell("caBX"), "caBX");
        assert_eq!(spell("a\u{200B}"), "aU+200B");
    }

    /// Every id is a format: lower case, ASCII, and distinct within its
    /// enum — two kinds with one id are one kind to every parser.
    #[test]
    fn every_id_is_distinct_and_ascii() {
        fn check(ids: &[&str]) {
            for (i, id) in ids.iter().enumerate() {
                assert!(
                    id.bytes()
                        .all(|b| b.is_ascii_lowercase() || b == b'-' || b.is_ascii_digit()),
                    "{id}"
                );
                assert!(!ids[i + 1..].contains(id), "{id} twice");
            }
        }
        check(&MetadataKind::ALL.map(MetadataKind::id));
        check(&ImageContainer::ALL.map(ImageContainer::id));
        check(&crate::Defect::IDS);
        let signals = [
            Signal::C2paManifest,
            Signal::C2paReference,
            Signal::DigitalSourceType(SourceType::AlgorithmicMedia),
            Signal::GeneratorKey(Generator::ComfyUi),
            Signal::GeneratorText(Generator::ComfyUi),
        ];
        check(&signals.map(Signal::id));
    }
}
