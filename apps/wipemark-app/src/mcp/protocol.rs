//! JSON-RPC 2.0 and the MCP methods, as functions over values.
//!
//! Nothing here touches a socket. [`respond`] takes the bytes of one
//! request body and gives back the bytes of one response — or `None`,
//! which is a notification and gets no reply at all — so every method
//! this server answers can be tested without binding anything.
//!
//! # The tools run, and refuse only what they cannot run
//!
//! `tools/call inspect { text, aggressive }` runs
//! `wipemark_core::inspect` and answers with the report of A §7.1 —
//! `InspectReport::to_json`, verbatim, as `content[0].text`, and the same
//! object parsed as `structuredContent`, the two places MCP 2025-06-18
//! asks a tool with structured output to put it. `tools/call clean
//! { text, aggressive, nfkc }` runs `wipemark_core::clean` and answers
//! with `{"text": <cleaned>, "report": <§7.1>}` in the same two places.
//! Positions are byte offsets into the UTF-8 text as the server received
//! it, and every report carries the third shelf — core's writer puts
//! `not_established` into every one, so a surface cannot forget it.
//!
//! A call that cannot run — `text` missing or not a string, a flag that
//! is not a boolean, an argument the tool does not take — is **refused**,
//! as a *successful* JSON-RPC response carrying `isError: true` and a
//! sentence that names the argument. Never as an empty report: an agent
//! that got `{"findings": []}` back for a text that was never read would
//! file the document as clean, and the next thing it did with it would
//! be built on that. `a_tool_that_cannot_run_refuses_rather_than_reporting_nothing`
//! is the gate. A refusal is a *result* rather than a protocol error
//! because that is the difference between the model reading it and the
//! client swallowing it on its way past — the case the 2025-11-25
//! revision of the specification moved to tool execution errors "that
//! language models can use to self-correct". Only a request that is not
//! a `CallToolRequest` at all (an `arguments` that is not an object, a
//! missing `name`, a tool this build does not have) stays a `-32602`.
//!
//! The text limit is the transport's: a body over a megabyte is answered
//! `413` before it is read (D13), never truncated.
//!
//! # `rewrite`
//!
//! `tools/call rewrite { text, tactic, intensity, candidates, rounds,
//! format, aggressive, nfkc, seed, dry_run }` runs Layer A, the pipeline on
//! the application's engine and Layer A again ([`super::rewrite`]), and
//! answers `{"text": <rewritten>, "report": <JobReport>}` the way `clean`
//! answers — the report is `JobReport::to_json`, ASCII, with its three
//! shelves. It refuses by name, as an `isError` result, everything the
//! other two refuse and three things more: a value outside a list (a
//! tactic, an intensity, a format — said back spelled), a tactic this
//! surface does not run, and every way a job can fail to happen — no
//! engine on duty, an engine that cannot answer, a job that failed, a
//! client that went away, the ceiling. Never with the input handed back as
//! though it were a rewrite: an agent that got its own text back with no
//! error would file it as rewritten.
//!
//! # `inspect_image` and `clean_image`
//!
//! `tools/call inspect_image { data }` and `clean_image { data, scope }`
//! take a PNG, JPEG or WebP as base64 — the standard alphabet, padded,
//! nothing looser — and run `wipemark-image` over it ([`super::image`]):
//! `ImageReport::to_json`, or `{"data": <base64>, "report":
//! <StripReport>}`, in the same two places the scrubber's reports go. Every
//! report carries the third shelf with the pixel domain on it: only the
//! metadata is examined. `scope` is `ai-provenance` (the default) or
//! `all-metadata`, which takes camera data, EXIF orientation included;
//! colour profiles are kept by both. They refuse, as an `isError` result
//! naming why, everything the scrubber's tools refuse and five things
//! more: `data` that is not base64, bytes that are not a picture this
//! server reads, a TIFF, HEIC or AVIF ("not in this version yet"), a
//! picture that could not be read, a JPEG whose MPF index a removal would
//! move — and a result that would still carry AI provenance, for which no
//! image comes back. No path argument: see [`super::image`].
//!
//! # What is said back
//!
//! Three answers quote something the client sent — an unknown method, an
//! unknown tool, an argument the tool does not take — and each goes
//! through [`spelled`] first: printable ASCII as itself, anything else as
//! `U+XXXX`. A tool called `cl` U+200B `ean` must not come back carrying
//! the U+200B. Two things are said back unspelled, on purpose: the
//! JSON-RPC `id`, which the client needs back exactly, and the cleaned
//! text of a `clean` result, which carries exactly what Layer A *kept* —
//! a ZWJ inside an emoji family, a U+FEFF at byte 0 — because it is the
//! user's text and not the server's words.
//! `nothing_the_server_says_carries_an_invisible_character` permits a
//! forbidden character only where the same response's report declares
//! it kept, and counts it.
//!
//! # Nothing here is localized
//!
//! Everything below is read by a machine, so none of it comes from the
//! catalogue — and for this module that is a correctness rule rather
//! than a convention. See the note at the top of [`super`]: the
//! application runs `wipemark-i18n` in `Rendering::Ui`, which keeps
//! Fluent's U+2068/U+2069 isolates around every interpolated value, and
//! those are `UnicodeClass::BidiControl`. A localized MCP response
//! would hand the agent the exact invisible characters this product
//! exists to remove, in the reply to a request to remove them.
//! `nothing_the_server_says_carries_an_invisible_character` is the
//! gate.

use serde_json::{json, Map, Value};
use wipemark_pipeline::asked::{self, Asked, NotOffered};
use wipemark_pipeline::prompt::row::Laid;
use wipemark_pipeline::prompt::{Intensity, Tactic};

use super::rewrite::{self, Done, Rewriter, Stop, Unrun};

/// The revision of the MCP specification this server implements.
///
/// Sent back from `initialize` verbatim. A client that speaks a later
/// one is told what it is talking to and decides for itself; a client
/// that speaks an earlier one is the same case in the other direction.
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// What the server calls itself when it introduces itself, and the
/// version it reports. Both are formats.
pub const IMPLEMENTATION: &str = super::SERVER_NAME;

/// The build, so that a bug report from an agent's transcript names one.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// The JSON-RPC 2.0 error codes. Named because a bare -32601 in a match
// arm is a number nobody can read.
const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;

/// What the server reaches beyond Layer A: the road to the engine on
/// duty, for `rewrite`. Empty, `rewrite` refuses — the arrangement of a
/// test that is not about the engine.
#[derive(Clone, Default)]
pub struct Services {
    pub rewriter: Option<Rewriter>,
}

/// The work this server will offer, one variant per tool.
///
/// The first two are Layer A: deterministic, verifiable, and exactly the
/// step an agent should be able to run over text it just produced. The
/// third rewrites with the model the application has on duty, between two
/// passes of Layer A. None takes a path — an agent cleaning its own output
/// has the text in hand, and a tool that read files would have to answer
/// the path containment question (spec §4.4) before it could answer
/// anything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Inspect,
    Clean,
    Rewrite,
    InspectImage,
    CleanImage,
}

/// The tactics `rewrite` runs, as a sentence and a schema list them — kept
/// beside [`asked::OFFERED`] by `the_rewrite_lists_what_it_runs`.
const TACTICS: &str = "paraphrase, humanize, back_translate";

/// The intensities, likewise.
const INTENSITIES: &str = "light, moderate, strong";

/// The formats, likewise.
const FORMATS: &str = "plain, markdown, html";

/// What a count must be.
const A_COUNT: &str = "a whole number from 1 to 8";

impl Tool {
    /// Every tool, in the order `tools/list` reports them.
    pub const ALL: [Tool; 5] = [
        Self::Inspect,
        Self::Clean,
        Self::Rewrite,
        Self::InspectImage,
        Self::CleanImage,
    ];

    /// The name a client calls it by. A format.
    pub fn name(self) -> &'static str {
        match self {
            Self::Inspect => "inspect",
            Self::Clean => "clean",
            Self::Rewrite => "rewrite",
            Self::InspectImage => "inspect_image",
            Self::CleanImage => "clean_image",
        }
    }

    /// Whether the tool takes a picture rather than a text.
    fn is_image(self) -> bool {
        matches!(self, Self::InspectImage | Self::CleanImage)
    }

    /// The one argument the tool cannot run without.
    fn required(self) -> &'static str {
        if self.is_image() {
            "data"
        } else {
            "text"
        }
    }

    /// What the tool does, for the model choosing between them.
    ///
    /// ASCII, every byte of it: a listing is what an agent reads to call
    /// this tool, and a non-ASCII letter in it is exactly the sort of
    /// thing this product flags.
    pub fn description(self) -> &'static str {
        match self {
            Self::Inspect => {
                "Report what Wipemark's deterministic Unicode scrubber finds in a piece of text, \
                 without changing anything: zero-width characters, bidi controls, tag \
                 characters, variation selectors, soft hyphens, unusual spaces, private-use and \
                 noncharacter code points, other invisible format characters, and letters \
                 borrowed from another script inside a word (homoglyphs). Each finding has its \
                 code point, its Unicode name, a confidence and every position as a byte offset \
                 into the UTF-8 text. Characters that carry real orthography - an emoji \
                 sequence, a Persian non-joiner - are listed as kept. Every report also lists \
                 what it does not establish. Takes up to about 1 MB of text."
            }
            Self::Clean => {
                "Remove the invisible Unicode from a piece of text and return the cleaned text \
                 with a report of exactly what was removed or replaced and where (byte offsets \
                 into the UTF-8 text you sent). Deterministic: the same text and options always \
                 give the same result, and cleaning the result again changes nothing. \
                 Characters that carry real orthography are kept and listed as kept; homoglyphs \
                 are replaced only with aggressive. Every report also lists what it does not \
                 establish. Takes up to about 1 MB of text."
            }
            Self::Rewrite => {
                "Rewrite a piece of text with the language model this application has on duty - \
                 the one its Engine settings choose: a model on this machine, or an endpoint, in \
                 which case the text is sent there. Between two passes of the deterministic \
                 Unicode scrubber: invisible characters are removed first, the model rewrites \
                 paragraph by paragraph, every candidate is checked against guards \
                 (placeholders, numbers, length, script, identifiers) and a candidate that fails \
                 is never used, and the result is scrubbed again. Returns the text and a report \
                 of every attempt; a paragraph no candidate passed keeps its scrubbed original, \
                 and the report says so. Rewriting is best-effort, and the report lists what it \
                 does not establish. Positions in its scrubber reports are byte offsets into the \
                 UTF-8 text. The call waits until the job ends, at most 60 minutes; closing the \
                 connection cancels it. With dry_run, returns only the estimated cost and loads \
                 nothing. Takes up to about 1 MB of text."
            }
            Self::InspectImage => {
                "Report the metadata blocks of a PNG, JPEG or WebP image without changing                  anything: EXIF, XMP, IPTC, C2PA manifests, PNG text chunks, colour profiles and                  the rest, each with its kind, its chunk or segment, and its byte offset and                  length in the file. A block that is AI provenance says which signal matched and                  which signature: a C2PA manifest or a reference to one, an IPTC digital source                  type naming a model or an algorithm, a text key or a signature an image                  generator writes - never the value, so a prompt is not quoted back. Camera data                  is listed and is not AI provenance. Only the metadata is examined, never the                  pixels, and every report lists what it does not establish. TIFF, HEIC and AVIF                  are refused by name. Pass the image as base64 in data; a request takes up to                  about 1 MB, so an image up to about 750 KB."
            }
            Self::CleanImage => {
                "Remove AI provenance metadata from a PNG, JPEG or WebP image and return the                  image as base64 in data, with a report of what was removed (byte offsets into                  the image you sent) and what the result still carries (byte offsets into the                  image returned), read off a second inspection of the result. Every other byte                  is kept as it was: the pixels are never decoded or re-encoded. With scope                  all-metadata, camera data goes too, EXIF orientation included; colour profiles                  are kept whatever the scope. When the result would still carry AI provenance,                  no image comes back. Only the metadata is examined, never the pixels, and every                  report lists what it does not establish. Takes up to about 1 MB of request, so                  an image up to about 750 KB as base64."
            }
        }
    }

    /// The arguments, as the JSON Schema a client validates against.
    ///
    /// `additionalProperties: false` because it is true: an argument the
    /// tool does not take is refused by name rather than ignored.
    pub fn schema(self) -> Value {
        if self.is_image() {
            return self.image_schema();
        }
        let mut properties = Map::new();
        properties.insert(
            "text".to_owned(),
            json!({
                "type": "string",
                "description": "The text itself, as a string - not a path. Up to about 1 MB; a \
                                larger request is refused whole, never truncated.",
            }),
        );
        properties.insert(
            "aggressive".to_owned(),
            json!({
                "type": "boolean",
                "default": false,
                "description": match self {
                    Self::Inspect => {
                        "List homoglyphs as findings clean would replace, rather than as kept. \
                         They are reported either way. Default false."
                    }
                    Self::Clean => {
                        "Also replace homoglyphs - a letter from another script inside a word, \
                         such as a Cyrillic letter that looks like a Latin one in an English \
                         word - with the matching letter of the word's own script. They are \
                         reported either way; higher false-positive rate, hence opt-in. Default \
                         false."
                    }
                    Self::Rewrite => {
                        "Also replace homoglyphs in the scrubber's passes before and after the \
                         model, as clean does with aggressive. Default false."
                    }
                    // `image_schema` answered for these before anything
                    // here was built.
                    Self::InspectImage | Self::CleanImage => "",
                },
            }),
        );
        if self == Self::Rewrite {
            properties.insert(
                "tactic".to_owned(),
                json!({
                    "type": "string",
                    "enum": asked::OFFERED.map(Tactic::as_str),
                    "default": "paraphrase",
                    "description": "How the model is asked: paraphrase (different words, the \
                                    default), humanize (reads as if a person wrote it), or \
                                    back_translate (into another language and back, two calls \
                                    a paragraph).",
                }),
            );
            properties.insert(
                "intensity".to_owned(),
                json!({
                    "type": "string",
                    "enum": Intensity::ALL.map(Intensity::as_str),
                    "default": "moderate",
                    "description": "How far a paraphrase or humanize may move from the wording. \
                                    Default moderate.",
                }),
            );
            for (name, description) in [
                (
                    "candidates",
                    "Candidates the model writes for each paragraph in a round, 1 to 8. Without \
                     it, the application decides: 1 for a model on this machine's processor \
                     alone, 2 for one on a graphics card or an endpoint.",
                ),
                (
                    "rounds",
                    "Rounds for each paragraph at most, 1 to 8; a round runs only when no \
                     candidate of the one before passed. Without it, up to 2.",
                ),
            ] {
                properties.insert(
                    name.to_owned(),
                    json!({
                        "type": "integer",
                        "minimum": 1,
                        "maximum": asked::MOST,
                        "description": description,
                    }),
                );
            }
            properties.insert(
                "format".to_owned(),
                json!({
                    "type": "string",
                    "enum": asked::FORMATS.map(asked::format_id),
                    "default": "plain",
                    "description": "What the text is. In markdown and html only the prose is \
                                    rewritten; code, headings, tables and markup come back byte \
                                    for byte. Default plain.",
                }),
            );
            properties.insert(
                "seed".to_owned(),
                json!({
                    "type": "integer",
                    "minimum": 0,
                    "description": "The base seed. Without it every call gets a new one, so \
                                    asking again gives a different rewrite; the report's \
                                    base_seed given back repeats a run on a model on this \
                                    machine.",
                }),
            );
            properties.insert(
                "templates".to_owned(),
                json!({
                    "type": "object",
                    "additionalProperties": { "type": ["string", "object"] },
                    "description": "Prompt templates for this call only, over the ones the \
                                    application saved: each key a template row such as \
                                    prompts.en.paraphrase.1.user, each value the template's text. \
                                    A template that breaks a rule is refused by its key and the \
                                    rule's id, and nothing runs.",
                }),
            );
            properties.insert(
                "dry_run".to_owned(),
                json!({
                    "type": "boolean",
                    "default": false,
                    "description": "Return only what the job would cost - calls, tokens and, \
                                    when the model has been checked, seconds - and run nothing. \
                                    Default false.",
                }),
            );
        }
        if self != Self::Inspect {
            properties.insert(
                "nfkc".to_owned(),
                json!({
                    "type": "boolean",
                    "default": false,
                    "description": "Apply NFKC normalisation after cleaning, and clean again \
                                    whatever NFKC uncovers until nothing is left to clean. Off \
                                    by default: NFKC changes more than provenance marks \
                                    (ligatures, full-width letters, superscripts), code \
                                    included.",
                }),
            );
        }

        json!({
            "type": "object",
            "properties": Value::Object(properties),
            "required": [self.required()],
            "additionalProperties": false,
        })
    }

    /// The picture tools' schema: `data`, and for `clean_image` the scope.
    fn image_schema(self) -> Value {
        let mut properties = Map::new();
        properties.insert(
            "data".to_owned(),
            json!({
                "type": "string",
                "contentEncoding": "base64",
                "description": "The image itself - a PNG, JPEG or WebP file's bytes - as base64:                                 the standard alphabet, padded, with no line breaks. Not a path.                                 The request takes up to about 1 MB; a larger one is refused                                 whole, never truncated.",
            }),
        );
        if self == Self::CleanImage {
            properties.insert(
                "scope".to_owned(),
                json!({
                    "type": "string",
                    "enum": wipemark_image::Scope::ALL.map(wipemark_image::Scope::id),
                    "default": "ai-provenance",
                    "description": "What is removed: ai-provenance (the default) removes only the                                     blocks that are AI provenance; all-metadata removes every                                     block but colour - camera data too, EXIF orientation                                     included, so a picture that relied on it may show turned.                                     Colour profiles are kept by both.",
                }),
            );
        }
        json!({
            "type": "object",
            "properties": Value::Object(properties),
            "required": [self.required()],
            "additionalProperties": false,
        })
    }

    /// The arguments this tool takes, in the order a refusal lists them.
    fn arguments(self) -> &'static [&'static str] {
        match self {
            Self::InspectImage => &["data"],
            Self::CleanImage => &["data", "scope"],
            Self::Inspect => &["text", "aggressive"],
            Self::Clean => &["text", "aggressive", "nfkc"],
            Self::Rewrite => &[
                "text",
                "tactic",
                "intensity",
                "candidates",
                "rounds",
                "format",
                "aggressive",
                "nfkc",
                "seed",
                "templates",
                "dry_run",
            ],
        }
    }

    /// The same list, spelled for a sentence.
    fn argument_list(self) -> &'static str {
        match self {
            Self::InspectImage => "`data` (an image as base64, required)",
            Self::CleanImage => {
                "`data` (an image as base64, required), `scope` (ai-provenance or all-metadata)"
            }
            Self::Inspect => "`text` (a string, required), `aggressive` (true or false)",
            Self::Clean => {
                "`text` (a string, required), `aggressive` (true or false), `nfkc` (true or \
                 false)"
            }
            Self::Rewrite => {
                "`text` (a string, required), `tactic` (paraphrase, humanize or back_translate), \
                 `intensity` (light, moderate or strong), `candidates` and `rounds` (1 to 8), \
                 `format` (plain, markdown or html), `aggressive`, `nfkc` and `dry_run` (true or \
                 false), `seed` (a whole number), `templates` (an object of template rows)"
            }
        }
    }

    /// The entry `tools/list` reports.
    fn listing(self) -> Value {
        json!({
            "name": self.name(),
            "description": self.description(),
            "inputSchema": self.schema(),
        })
    }

    /// The tool a client asked for, or `None` for a name this build
    /// does not have.
    fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tool| tool.name() == name)
    }

    /// Run Layer A over a call that has been read and checked.
    ///
    /// On the connection thread: Layer A is O(n), a body is at most a
    /// megabyte, and nothing here is near the GPUI thread.
    fn run(self, call: &Call) -> Answer {
        let (json, findings, kept) = match self {
            // `call` reads and runs a rewrite apart; nothing reaches here
            // with one. Said at the protocol level rather than panicking on
            // a connection's thread, should that ever change.
            Self::Rewrite | Self::InspectImage | Self::CleanImage => {
                tracing::error!(
                    tool = self.name(),
                    "MCP: a tool reached the scrubber's road"
                );
                return Answer::Error {
                    code: INTERNAL_ERROR,
                    message: format!("{} is not a scrubber call", self.name()),
                };
            }
            Self::Inspect => {
                let report = wipemark_core::inspect(&call.text, &call.options);
                (report.to_json(), report.findings.len(), report.kept.len())
            }
            Self::Clean => {
                let cleaned = wipemark_core::clean(&call.text, &call.options);
                // `to_string` of a `&str` cannot fail; the report goes in
                // verbatim so the text block keeps §7.1's key order.
                let text = serde_json::to_string(&cleaned.text).unwrap_or_default();
                (
                    format!(r#"{{"text":{text},"report":{}}}"#, cleaned.report.to_json()),
                    cleaned.report.findings.len(),
                    cleaned.report.kept.len(),
                )
            }
        };
        tracing::info!(
            tool = self.name(),
            text = %wipemark_log::Elided::from(&call.text),
            aggressive = call.options.aggressive,
            nfkc = call.options.nfkc,
            findings,
            kept,
            "MCP: tools/call answered"
        );
        answered(json)
    }

    /// A rewrite that was read and did not happen: a result carrying
    /// `isError`, the reason, and no text — an agent handed its own text
    /// back with no error would file it as rewritten.
    fn unrun(self, unrun: &Unrun) -> Value {
        tracing::info!(tool = self.name(), unrun = ?unrun, "MCP: tools/call did not run");
        json!({
            "content": [{
                "type": "text",
                "text": format!(
                    "`{name}` did not run: {said}. Nothing was rewritten, and no text comes \
                     back - a rewrite that did not happen is not reported as one.",
                    name = self.name(),
                    said = unrun_said(unrun),
                ),
            }],
            "isError": true,
        })
    }

    /// The refusal, in the shape MCP reserves for a tool that was called
    /// and could not do the job: a result a model reads, carrying
    /// `isError`, and never a report — no `structuredContent`, no
    /// `findings`, nothing an agent could mistake for a scan that ran.
    fn refuse(self, problems: &[Problem]) -> Value {
        let kinds: Vec<&'static str> = problems.iter().map(Problem::kind).collect();
        tracing::info!(tool = self.name(), problems = ?kinds, "MCP: tools/call refused");
        let said = problems
            .iter()
            .map(Problem::said)
            .collect::<Vec<_>>()
            .join("; ");
        json!({
            "content": [{
                "type": "text",
                "text": format!(
                    "`{name}` did not run: {said}. Its arguments are {list}. Refusing rather \
                     than answering — a report about {what} that was never read would say that \
                     nothing was found.",
                    name = self.name(),
                    list = self.argument_list(),
                    what = if self.is_image() { "an image" } else { "text" },
                ),
            }],
            "isError": true,
        })
    }
}

/// A picture tool's call, read and checked: the bytes `data` decoded to.
struct ImageCall {
    bytes: Vec<u8>,
    scope: wipemark_image::Scope,
}

/// A picture that was read and could not be inspected or cleaned: a result
/// carrying `isError`, the reason, and neither a report nor an image.
fn image_refused(tool: Tool, refusal: &super::image::Refusal) -> Value {
    tracing::info!(
        tool = tool.name(),
        refusal = refusal.kind(),
        "MCP: tools/call did not run"
    );
    json!({
        "content": [{
            "type": "text",
            "text": format!(
                "`{name}` did not run: {said}. No report comes back, and no image - a report \
                 about a picture that was never read would say that nothing was found.",
                name = tool.name(),
                said = refusal.said(),
            ),
        }],
        "isError": true,
    })
}

/// Read a picture tool's arguments: `data` as base64, `scope` from its two
/// values. Every problem collected and named, none coerced, as
/// [`read_call`] does.
fn read_image(tool: Tool, arguments: &Map<String, Value>) -> Result<ImageCall, Vec<Problem>> {
    let mut problems = Vec::new();

    let bytes = match arguments.get("data") {
        None => {
            problems.push(Problem::Missing("data"));
            None
        }
        Some(Value::String(data)) => match super::image::decode(data) {
            Some(bytes) => Some(bytes),
            None => {
                problems.push(Problem::WrongType {
                    name: "data",
                    wants: "an image as base64: the standard alphabet, padded, with no line \
                            breaks",
                });
                None
            }
        },
        Some(_) => {
            problems.push(Problem::WrongType {
                name: "data",
                wants: "a string",
            });
            None
        }
    };

    let scope = match arguments.get("scope").filter(|_| tool == Tool::CleanImage) {
        None => wipemark_image::Scope::AiProvenance,
        Some(Value::String(said)) => wipemark_image::Scope::ALL
            .into_iter()
            .find(|scope| scope.id() == said)
            .unwrap_or_else(|| {
                problems.push(Problem::NotOneOf {
                    name: "scope",
                    said: spelled(said),
                    wants: "ai-provenance, all-metadata",
                });
                wipemark_image::Scope::AiProvenance
            }),
        Some(_) => {
            problems.push(Problem::WrongType {
                name: "scope",
                wants: "a string",
            });
            wipemark_image::Scope::AiProvenance
        }
    };

    let mut extra: Vec<&String> = arguments
        .keys()
        .filter(|key| !tool.arguments().contains(&key.as_str()))
        .collect();
    extra.sort();
    problems.extend(extra.into_iter().map(|key| Problem::NotTaken(spelled(key))));

    match bytes {
        Some(bytes) if problems.is_empty() => Ok(ImageCall { bytes, scope }),
        _ => Err(problems),
    }
}

/// Run a picture tool over a call that was read, and answer.
fn image_answer(tool: Tool, call: &ImageCall) -> Answer {
    let ran = if tool == Tool::CleanImage {
        super::image::clean(&call.bytes, call.scope)
    } else {
        super::image::inspect(&call.bytes)
    };
    match ran {
        Ok(json) => answered(json),
        Err(refusal) => Answer::Result(image_refused(tool, &refusal)),
    }
}

/// One call, read and checked.
struct Call {
    text: String,
    /// `aggressive` and `nfkc` from the call; `normalize_spaces` and
    /// `keep_soft_hyphen` stay off on every surface in E1 (A §7.4, Q-A1).
    options: wipemark_core::Options,
}

/// What is wrong with a call, in the order the refusal says it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Problem {
    /// A required argument is not there.
    Missing(&'static str),
    /// An argument is there and is the wrong kind of value. `wants` is
    /// "a string" or "true or false".
    WrongType {
        name: &'static str,
        wants: &'static str,
    },
    /// An argument the tool does not take, already [`spelled`].
    NotTaken(String),
    /// A string outside the values an argument takes. `said` is the
    /// client's value, already [`spelled`]; `wants` lists the values.
    NotOneOf {
        name: &'static str,
        said: String,
        wants: &'static str,
    },
    /// A tactic this surface does not run, and why (H8).
    NotOffered { tactic: Tactic, why: NotOffered },
}

impl Problem {
    /// The clause of the refusal that says this one.
    fn said(&self) -> String {
        match self {
            Self::Missing(name) => format!("the argument `{name}` is missing"),
            Self::WrongType { name, wants } => format!("the argument `{name}` must be {wants}"),
            Self::NotTaken(name) => format!("it takes no argument `{name}`"),
            Self::NotOneOf { name, said, wants } => {
                format!("the argument `{name}` is `{said}`, which is not one of {wants}")
            }
            Self::NotOffered { tactic, why } => {
                format!("the tactic `{}` is not run here: {why}", tactic.as_str())
            }
        }
    }

    /// The log's word for it: static, because the values are the
    /// client's and a log line is not where they go.
    fn kind(&self) -> &'static str {
        match self {
            Self::Missing(_) => "missing",
            Self::WrongType { .. } => "wrong type",
            Self::NotTaken(_) => "not taken",
            Self::NotOneOf { .. } => "not one of",
            Self::NotOffered { .. } => "not offered",
        }
    }
}

/// The sentence for a rewrite that did not happen. English, because
/// nothing the server says comes from the catalogue; anything in it that
/// came from outside this program — an engine's or a server's words, a
/// path — is [`spelled`].
fn unrun_said(unrun: &Unrun) -> String {
    match unrun {
        Unrun::Nobody => "this server has no way to an engine".to_owned(),
        Unrun::Unavailable(why) => {
            format!("no engine could answer: {}", spelled(&why.to_string()))
        }
        Unrun::Engine(error) => format!(
            "the engine on duty could not be taken: {}",
            spelled(&error.to_string())
        ),
        Unrun::NotOffered(why) => format!("the tactic is not run here: {why}"),
        Unrun::Templates(Laid::UnknownRow { key }) => format!(
            "`templates` names `{}`, which is not a template row this build has",
            spelled(key)
        ),
        Unrun::Templates(Laid::Unreadable { key }) => format!(
            "`templates` gives `{}` a value that is neither a template's text nor a saved row",
            spelled(key)
        ),
        Unrun::Templates(Laid::Breaks { key, rule }) => {
            format!("the template `{}` breaks the rule `{rule}`", spelled(key))
        }
        Unrun::Refused(why) => format!("the job did not start: {}", spelled(&why.to_string())),
        Unrun::Failed(error) => format!("the job failed: {}", spelled(&error.to_string())),
        Unrun::Stopped(Some(Stop::HungUp)) => {
            "the client closed the connection, and the job was cancelled".to_owned()
        }
        Unrun::Stopped(Some(Stop::Ceiling)) => format!(
            "the job ran past {} minutes and was cancelled",
            rewrite::CEILING.as_secs() / 60
        ),
        Unrun::Stopped(None) => "the job was cancelled".to_owned(),
        Unrun::Lost => "the job stopped without an answer".to_owned(),
    }
}

/// Read a `rewrite` call's arguments.
///
/// The same bargain as [`read_call`]: every problem collected, each named,
/// none coerced — a count of `"2"` is a string and is refused, because
/// running a different job from the one asked for and reporting it as that
/// one is the failure every refusal here exists to prevent.
fn read_rewrite(arguments: &Map<String, Value>) -> Result<rewrite::Call, Vec<Problem>> {
    let mut problems = Vec::new();

    let text = match arguments.get("text") {
        None => {
            problems.push(Problem::Missing("text"));
            None
        }
        Some(Value::String(text)) => Some(text.clone()),
        Some(_) => {
            problems.push(Problem::WrongType {
                name: "text",
                wants: "a string",
            });
            None
        }
    };

    // One of a list, by its id: `parse` names the value or there is none.
    fn one_of<T>(
        arguments: &Map<String, Value>,
        problems: &mut Vec<Problem>,
        name: &'static str,
        wants: &'static str,
        parse: impl Fn(&str) -> Option<T>,
    ) -> Option<T> {
        match arguments.get(name)? {
            Value::String(said) => {
                let parsed = parse(said);
                if parsed.is_none() {
                    problems.push(Problem::NotOneOf {
                        name,
                        said: spelled(said),
                        wants,
                    });
                }
                parsed
            }
            _ => {
                problems.push(Problem::WrongType {
                    name,
                    wants: "a string",
                });
                None
            }
        }
    }

    let tactic =
        one_of(arguments, &mut problems, "tactic", TACTICS, Tactic::parse).and_then(|tactic| {
            match asked::offered(tactic) {
                Ok(tactic) => Some(tactic),
                Err(why) => {
                    problems.push(Problem::NotOffered { tactic, why });
                    None
                }
            }
        });
    let intensity = one_of(
        arguments,
        &mut problems,
        "intensity",
        INTENSITIES,
        Intensity::parse,
    );
    let format = one_of(
        arguments,
        &mut problems,
        "format",
        FORMATS,
        asked::format_of,
    );

    let mut count = |name: &'static str| match arguments.get(name) {
        None => None,
        Some(value) => match value.as_u64().filter(|n| asked::count_ok(*n)) {
            Some(n) => u8::try_from(n).ok(),
            None => {
                problems.push(Problem::WrongType {
                    name,
                    wants: A_COUNT,
                });
                None
            }
        },
    };
    let candidates = count("candidates");
    let rounds = count("rounds");

    let seed = match arguments.get("seed") {
        None => None,
        Some(value) => {
            let seed = value.as_u64();
            if seed.is_none() {
                problems.push(Problem::WrongType {
                    name: "seed",
                    wants: "a whole number, 0 or more",
                });
            }
            seed
        }
    };

    let mut flag = |name: &'static str| match arguments.get(name) {
        None => false,
        Some(Value::Bool(value)) => *value,
        Some(_) => {
            problems.push(Problem::WrongType {
                name,
                wants: "true or false",
            });
            false
        }
    };
    let aggressive = flag("aggressive");
    let nfkc = flag("nfkc");
    let dry_run = flag("dry_run");

    let templates = match arguments.get("templates") {
        None => Map::new(),
        Some(Value::Object(templates)) => templates.clone(),
        Some(_) => {
            problems.push(Problem::WrongType {
                name: "templates",
                wants: "an object of template rows",
            });
            Map::new()
        }
    };

    let mut extra: Vec<&String> = arguments
        .keys()
        .filter(|key| !Tool::Rewrite.arguments().contains(&key.as_str()))
        .collect();
    extra.sort();
    problems.extend(extra.into_iter().map(|key| Problem::NotTaken(spelled(key))));

    match text {
        Some(text) if problems.is_empty() => Ok(rewrite::Call {
            text,
            asked: Asked {
                tactic: tactic.unwrap_or(Tactic::Paraphrase),
                intensity: intensity.unwrap_or_default(),
                candidates,
                rounds,
                format: format.unwrap_or(wipemark_pipeline::prepare::TextFormat::Plain),
                aggressive,
                nfkc,
                seed,
            },
            templates,
            dry_run,
        }),
        _ => Err(problems),
    }
}

/// Run a rewrite that was read, and answer — the text and its report, the
/// price, or why nothing was rewritten.
fn rewrite_answer(call: rewrite::Call, services: &Services, gone: &dyn Fn() -> bool) -> Answer {
    let Some(rewriter) = &services.rewriter else {
        return Answer::Result(Tool::Rewrite.unrun(&Unrun::Nobody));
    };
    match rewriter.run(call, gone) {
        // The report goes in verbatim, as `clean`'s does, so the text
        // block is the bytes `JobReport::to_json` wrote.
        Ok(Done::Rewritten { text, report }) => {
            let text = serde_json::to_string(&text).unwrap_or_default();
            answered(format!(r#"{{"text":{text},"report":{report}}}"#))
        }
        Ok(Done::Priced(price)) => answered(price.to_string()),
        Err(unrun) => Answer::Result(Tool::Rewrite.unrun(&unrun)),
    }
}

/// Read a call's arguments against what the tool takes.
///
/// Every problem is collected rather than the first returned, so a model
/// that got two things wrong is told both in one round trip. An empty
/// `text` is valid: Layer A runs over it and reports nothing, which is a
/// true report of a scan that ran.
fn read_call(tool: Tool, arguments: &Map<String, Value>) -> Result<Call, Vec<Problem>> {
    let mut problems = Vec::new();

    let text = match arguments.get("text") {
        None => {
            problems.push(Problem::Missing("text"));
            None
        }
        Some(Value::String(text)) => Some(text.clone()),
        Some(_) => {
            problems.push(Problem::WrongType {
                name: "text",
                wants: "a string",
            });
            None
        }
    };

    let mut flag = |name: &'static str| match arguments.get(name) {
        None => false,
        Some(Value::Bool(value)) => *value,
        Some(_) => {
            problems.push(Problem::WrongType {
                name,
                wants: "true or false",
            });
            false
        }
    };
    let aggressive = flag("aggressive");
    let nfkc = tool.arguments().contains(&"nfkc") && flag("nfkc");

    // Sorted, so the sentence does not depend on whether `serde_json`
    // was built with `preserve_order` in this particular build.
    let mut extra: Vec<&String> = arguments
        .keys()
        .filter(|key| !tool.arguments().contains(&key.as_str()))
        .collect();
    extra.sort();
    problems.extend(extra.into_iter().map(|key| Problem::NotTaken(spelled(key))));

    match text {
        Some(text) if problems.is_empty() => Ok(Call {
            text,
            options: wipemark_core::Options {
                aggressive,
                nfkc,
                ..wipemark_core::Options::default()
            },
        }),
        _ => Err(problems),
    }
}

/// The answer MCP asks for when a tool returns structured content: the
/// object, and the same object serialized in a text block "for backwards
/// compatibility" (MCP 2025-06-18, Tools › Structured Content).
///
/// The text block is the string Layer A's writer produced, not a
/// re-serialization of the parsed value: `serde_json` orders keys
/// differently from one build of this workspace to the next, and the
/// text an older client shows a model should be the same bytes every
/// time.
fn answered(json: String) -> Answer {
    match serde_json::from_str::<Value>(&json) {
        Ok(structured) => Answer::Result(json!({
            "content": [{ "type": "text", "text": json }],
            "structuredContent": structured,
            "isError": false,
        })),
        // Unreachable while `to_json` is right; `an_mcp_report_is_json_a_client_can_parse`
        // is what keeps it so. A server that broke says so at the protocol level.
        Err(error) => {
            tracing::error!(%error, "MCP: a Layer A report is not JSON");
            Answer::Error {
                code: INTERNAL_ERROR,
                message: "the report could not be rendered".to_owned(),
            }
        }
    }
}

/// A string the client sent, safe to say back: printable ASCII as itself,
/// anything else spelled as `U+XXXX`.
///
/// The rule `nothing_the_server_says_carries_an_invisible_character`
/// enforces, applied at the three places an answer quotes the client.
fn spelled(client: &str) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(client.len());
    for character in client.chars() {
        if character == ' ' || character.is_ascii_graphic() {
            out.push(character);
        } else {
            let _ = write!(out, "U+{:04X}", u32::from(character));
        }
    }
    out
}

/// [`respond_with`] for a server with nothing beyond Layer A and a client
/// that never hangs up — what every test that is not about `rewrite`
/// asks.
#[cfg(test)]
pub fn respond(body: &str) -> Option<String> {
    respond_with(body, &Services::default(), &|| false)
}

/// Answer one request body.
///
/// `None` is the honest answer to a notification: JSON-RPC says a
/// message with no `id` gets no reply, and the transport above turns
/// that into `204 No Content`. `services` is what `rewrite` runs on, and
/// `gone` says whether the client has hung up — a rewrite waits for its
/// job, and a client that went away cancels it.
pub fn respond_with(body: &str, services: &Services, gone: &dyn Fn() -> bool) -> Option<String> {
    let request: Value = match serde_json::from_str(body) {
        Ok(request) => request,
        Err(error) => {
            // No id to echo — there is no parsed request to take one
            // from — so `null`, which is what the specification says to
            // send when the id cannot be determined.
            return Some(render(&failure(
                Value::Null,
                PARSE_ERROR,
                &format!("could not parse the request body as JSON: {error}"),
            )));
        }
    };

    // Batching left the specification in 2025-06-18, and a server that
    // half-implements it is worse than one that says so.
    let Some(request) = request.as_object() else {
        return Some(render(&failure(
            Value::Null,
            INVALID_REQUEST,
            "expected a single JSON-RPC request object",
        )));
    };

    let id = request.get("id").cloned();
    let Some(method) = request.get("method").and_then(Value::as_str) else {
        return Some(render(&failure(
            id.unwrap_or(Value::Null),
            INVALID_REQUEST,
            "the request names no method",
        )));
    };
    let params = request.get("params").cloned().unwrap_or_else(|| json!({}));

    let answer = dispatch(method, &params, services, gone)?;
    // A method that produced an answer for a message carrying no id is
    // a notification that was handled: the work is done, and the reply
    // is not owed. Both halves have to be checked — `notifications/*`
    // is a notification by its name, and `ping` with no id is one by
    // its shape.
    let id = id?;
    Some(render(&with_id(id, answer)))
}

/// The result or error for one method, without the envelope.
fn dispatch(
    method: &str,
    params: &Value,
    services: &Services,
    gone: &dyn Fn() -> bool,
) -> Option<Answer> {
    match method {
        "initialize" => Some(Answer::Result(json!({
            "protocolVersion": PROTOCOL_VERSION,
            // Tools and nothing else. Declaring resources or prompts
            // here would be an invitation to call `resources/list`,
            // which this server answers with "method not found" —
            // advertising a door that is not there is worse than
            // having fewer doors.
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": IMPLEMENTATION, "version": VERSION },
        }))),

        // Every `notifications/*` message is exactly that, and the
        // reply to a notification is silence.
        method if method.starts_with("notifications/") => None,

        "ping" => Some(Answer::Result(json!({}))),

        "tools/list" => Some(Answer::Result(json!({
            "tools": Tool::ALL.map(Tool::listing).to_vec(),
        }))),

        "tools/call" => Some(call(params, services, gone)),

        other => Some(Answer::Error {
            code: METHOD_NOT_FOUND,
            message: format!("method not found: {}", spelled(other)),
        }),
    }
}

/// `tools/call`: name → tool → arguments → read → run or refuse.
fn call(params: &Value, services: &Services, gone: &dyn Fn() -> bool) -> Answer {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return Answer::Error {
            code: INVALID_PARAMS,
            message: "tools/call needs a `name`".to_owned(),
        };
    };
    let Some(tool) = Tool::named(name) else {
        // A tool that does not exist is the client's mistake and belongs
        // at the protocol level, where a client can tell it apart from a
        // tool that ran.
        return Answer::Error {
            code: INVALID_PARAMS,
            message: format!(
                "unknown tool: {}. This build offers: {}",
                spelled(name),
                Tool::ALL.map(|tool| tool.name()).join(", ")
            ),
        };
    };
    // Absent and `null` are the same thing: no arguments. Anything else
    // that is not an object fails the `CallToolRequest` shape itself,
    // which both revisions of MCP put at the protocol level.
    let empty = Map::new();
    let arguments = match params.get("arguments") {
        None | Some(Value::Null) => &empty,
        Some(Value::Object(arguments)) => arguments,
        Some(_) => {
            return Answer::Error {
                code: INVALID_PARAMS,
                message: "tools/call `arguments` must be an object".to_owned(),
            };
        }
    };
    // Refusals are reported as results so the model reads them, not as
    // transport errors the client swallows on its way past.
    match tool {
        Tool::Rewrite => match read_rewrite(arguments) {
            Ok(call) => rewrite_answer(call, services, gone),
            Err(problems) => Answer::Result(tool.refuse(&problems)),
        },
        Tool::Inspect | Tool::Clean => match read_call(tool, arguments) {
            Ok(call) => tool.run(&call),
            Err(problems) => Answer::Result(tool.refuse(&problems)),
        },
        Tool::InspectImage | Tool::CleanImage => match read_image(tool, arguments) {
            Ok(call) => image_answer(tool, &call),
            Err(problems) => Answer::Result(tool.refuse(&problems)),
        },
    }
}

/// One method's answer, before the JSON-RPC envelope is put round it.
enum Answer {
    Result(Value),
    Error { code: i64, message: String },
}

fn with_id(id: Value, answer: Answer) -> Value {
    match answer {
        Answer::Result(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Answer::Error { code, message } => failure(id, code, &message),
    }
}

fn failure(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// Compact, unlike the snippets in [`super`]: this one is read by a
/// client and counted by a `Content-Length`, and no part of it is
/// pasted into a file by hand.
fn render(response: &Value) -> String {
    serde_json::to_string(response).unwrap_or_else(|error| {
        // Unreachable: everything above is built from `json!` literals
        // and owned strings. Rendered by hand rather than panicking,
        // because the caller is a socket thread and a panic there takes
        // the connection down without an answer.
        tracing::error!(%error, "could not render an MCP response");
        format!(
            r#"{{"jsonrpc":"2.0","id":null,"error":{{"code":{PARSE_ERROR},"message":"could not render the response"}}}}"#
        )
    })
}

#[cfg(test)]
mod tests {
    use wipemark_engine::fake::FakeEngine;

    use super::*;

    /// Parse a response back, for a test that wants to look inside it.
    fn answer(request: &str) -> Value {
        let body = respond(request).expect("a request with an id is owed an answer");
        serde_json::from_str(&body).expect("the server rendered something that is not JSON")
    }

    /// The first thing every client sends, and the only handshake there
    /// is. A server that gets this wrong is one that never appears.
    #[test]
    fn an_initialize_names_the_protocol_and_the_server() {
        let response = answer(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#,
        );
        assert_eq!(response["id"], json!(1));
        assert_eq!(response["jsonrpc"], json!("2.0"));
        assert_eq!(
            response["result"]["protocolVersion"],
            json!(PROTOCOL_VERSION)
        );
        assert_eq!(
            response["result"]["serverInfo"]["name"],
            json!(IMPLEMENTATION)
        );
        assert!(response["result"]["capabilities"]["tools"].is_object());
        assert!(
            response["error"].is_null(),
            "the handshake came back as an error: {response}"
        );
    }

    /// A notification is a message with no id, and the specification
    /// says it gets no reply. A server that answers one puts a response
    /// on the wire the client is not reading for, and some clients
    /// treat that as a protocol violation and hang up.
    #[test]
    fn a_notification_is_answered_with_silence() {
        for notification in [
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":1}}"#,
            r#"{"jsonrpc":"2.0","method":"ping"}"#,
        ] {
            assert_eq!(
                respond(notification),
                None,
                "answered a notification: {notification}"
            );
        }
    }

    /// The rule the whole repository keeps, pointed at a protocol: a
    /// tool that cannot run refuses, names itself and the argument it is
    /// missing, and says the work did not happen. The failure this
    /// catches is the worst one this build could ship — an agent filing
    /// a document as clean because a scrubber that never read it
    /// reported nothing. So a refusal carries no report at all: no
    /// `structuredContent`, nothing shaped like findings (D14).
    #[test]
    fn a_tool_that_cannot_run_refuses_rather_than_reporting_nothing() {
        for tool in Tool::ALL {
            for params in [
                format!(r#"{{"name":"{}","arguments":{{}}}}"#, tool.name()),
                format!(r#"{{"name":"{}"}}"#, tool.name()),
            ] {
                let response = answer(&format!(
                    r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{params}}}"#
                ));
                let result = &response["result"];
                assert_eq!(
                    result["isError"],
                    json!(true),
                    "{tool:?} answered as though it had done the work: {response}"
                );
                assert!(
                    result.get("structuredContent").is_none(),
                    "{tool:?} refused with a report attached: {response}"
                );
                let text = result["content"][0]["text"]
                    .as_str()
                    .expect("a refusal a model can read");
                assert!(
                    text.contains(tool.name()),
                    "the refusal does not name the tool"
                );
                let required = format!("`{}`", tool.required());
                assert!(
                    text.contains(&required),
                    "the refusal does not name the missing argument: {text}"
                );
            }
        }
    }

    /// And it refuses as a *result*, not as a transport error. The
    /// difference is whether the model ever sees it: a JSON-RPC error
    /// is handled by the client, a tool result reaches the
    /// conversation.
    #[test]
    fn a_refusal_reaches_the_model_rather_than_the_client() {
        let response = answer(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"clean","arguments":{}}}"#,
        );
        assert!(
            response["error"].is_null(),
            "the refusal was filed as a transport error, where a model never reads it"
        );
        assert!(response["result"]["isError"].as_bool().unwrap_or(false));
    }

    /// Send `tools/call` for `tool` with these arguments (a JSON
    /// literal) and parse the answer.
    fn called(tool: &str, arguments: &str) -> Value {
        answer(&format!(
            r#"{{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
        ))
    }

    /// The refusal's sentence, asserting on the way that it is one.
    fn refusal_text(response: &Value) -> String {
        assert!(response["error"].is_null(), "a protocol error: {response}");
        assert_eq!(response["result"]["isError"], json!(true), "{response}");
        assert!(
            response["result"].get("structuredContent").is_none(),
            "a refusal with a report attached: {response}"
        );
        response["result"]["content"][0]["text"]
            .as_str()
            .expect("a refusal a model can read")
            .to_owned()
    }

    /// A value of the wrong kind is refused, by the argument's name. A
    /// flag coerced to `false` would run a different scan from the one
    /// that was asked for and report it as that one.
    #[test]
    fn an_argument_of_the_wrong_type_is_refused_by_name() {
        for (tool, arguments, named) in [
            ("inspect", r#"{"text":5}"#, "`text`"),
            ("clean", r#"{"text":null}"#, "`text`"),
            (
                "inspect",
                r#"{"text":"a","aggressive":"yes"}"#,
                "`aggressive`",
            ),
            ("clean", r#"{"text":"a","aggressive":null}"#, "`aggressive`"),
            ("clean", r#"{"text":"a","nfkc":1}"#, "`nfkc`"),
        ] {
            let text = refusal_text(&called(tool, arguments));
            assert!(text.contains(named), "{tool} {arguments}: {text}");
        }
    }

    /// An argument the tool does not take is refused rather than
    /// ignored — `aggresive` misspelled would otherwise run the default
    /// scan under a name that says it was the aggressive one. The
    /// schema says so too.
    #[test]
    fn an_argument_the_tool_does_not_take_is_refused_by_name() {
        for (tool, arguments, named) in [
            ("inspect", r#"{"text":"a","nfkc":true}"#, "`nfkc`"),
            ("clean", r#"{"text":"a","aggresive":true}"#, "`aggresive`"),
        ] {
            let text = refusal_text(&called(tool, arguments));
            assert!(text.contains(named), "{tool} {arguments}: {text}");
            let listed = Tool::named(tool).expect("a tool").argument_list();
            assert!(
                text.contains(listed),
                "{tool}: the arguments are not listed: {text}"
            );
        }

        let response = answer(r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#);
        for listed in response["result"]["tools"].as_array().expect("tools") {
            assert_eq!(
                listed["inputSchema"]["additionalProperties"],
                json!(false),
                "{listed}"
            );
        }
    }

    /// A request that is not a `CallToolRequest` at all is the client's
    /// protocol mistake, and is answered at the protocol level.
    #[test]
    fn arguments_that_are_not_an_object_are_a_protocol_error() {
        for arguments in [r#""text""#, "[1]"] {
            let response = called("inspect", arguments);
            assert_eq!(
                response["error"]["code"],
                json!(INVALID_PARAMS),
                "{response}"
            );
            assert!(response.get("result").is_none(), "{response}");
        }
    }

    /// `clean` hands back the text with the mark gone, and the report of
    /// what went and where.
    #[test]
    fn a_clean_result_carries_the_cleaned_text_and_its_report() {
        let response = called("clean", r#"{"text":"a\u200Bb"}"#);
        let result = &response["result"];
        assert_eq!(result["isError"], json!(false), "{response}");
        let structured = &result["structuredContent"];
        assert_eq!(structured["text"], json!("ab"));
        assert_eq!(structured["report"]["removed"], json!({"zero-width": 1}));
        let finding = &structured["report"]["findings"][0];
        assert_eq!(finding["codepoint"], json!("U+200B"));
        assert_eq!(finding["name"], json!("ZERO WIDTH SPACE"));
        assert_eq!(finding["positions"], json!([1]));
    }

    /// Every input the report tests below walk: a mark, nothing, and a
    /// homoglyph, each without and with `aggressive` — and for `clean`
    /// without and with `nfkc` too.
    fn report_calls() -> Vec<(Tool, String)> {
        let mut calls = Vec::new();
        for text in [r#""a\u200Bb""#, r#""""#, r#""p\u0430y""#] {
            for aggressive in [false, true] {
                calls.push((
                    Tool::Inspect,
                    format!(r#"{{"text":{text},"aggressive":{aggressive}}}"#),
                ));
                for nfkc in [false, true] {
                    calls.push((
                        Tool::Clean,
                        format!(r#"{{"text":{text},"aggressive":{aggressive},"nfkc":{nfkc}}}"#),
                    ));
                }
            }
        }
        calls
    }

    /// The text block and the structured content are the same object,
    /// and that object is the §7.1 report — `suspicious` and `stats`
    /// included in the clean form (D28), ASCII in the inspect text
    /// block (D29).
    #[test]
    fn an_mcp_report_is_json_a_client_can_parse() {
        for (tool, arguments) in report_calls() {
            let response = called(tool.name(), &arguments);
            let result = &response["result"];
            assert_eq!(result["isError"], json!(false), "{response}");
            let text = result["content"][0]["text"].as_str().expect("a text block");
            let parsed: Value = serde_json::from_str(text).expect("the text block is JSON");
            let structured = &result["structuredContent"];
            assert!(structured.is_object(), "{tool:?} {arguments}: {response}");
            assert_eq!(&parsed, structured, "{tool:?} {arguments}");
            match tool {
                Tool::Inspect => {
                    assert!(text.is_ascii(), "{arguments}: {text}");
                    assert_eq!(
                        structured["unicode_version"],
                        json!(wipemark_core::UNICODE_VERSION)
                    );
                    for key in ["suspicious", "findings", "kept", "stats"] {
                        assert!(structured.get(key).is_some(), "{arguments}: no {key}");
                    }
                }
                Tool::Clean => {
                    let keys: Vec<&String> =
                        structured.as_object().expect("an object").keys().collect();
                    assert_eq!(keys.len(), 2, "{arguments}: {keys:?}");
                    assert!(structured["text"].is_string(), "{arguments}");
                    for key in [
                        "suspicious",
                        "stats",
                        "findings",
                        "kept",
                        "removed",
                        "normalized",
                        "output_len",
                        "unicode_version",
                    ] {
                        assert!(
                            structured["report"].get(key).is_some(),
                            "{arguments}: the report has no {key}"
                        );
                    }
                }
                Tool::Rewrite | Tool::InspectImage | Tool::CleanImage => {
                    unreachable!("report_calls holds Layer A calls only")
                }
            }
        }
    }

    /// The third shelf on every answer that is a report, in core's order.
    #[test]
    fn every_layer_a_answer_carries_the_third_shelf() {
        let ids: Vec<Value> = wipemark_core::report::not_established::ALL
            .iter()
            .map(|(id, _)| json!(id))
            .collect();
        for (tool, arguments) in report_calls() {
            let response = called(tool.name(), &arguments);
            let structured = &response["result"]["structuredContent"];
            let shelf = match tool {
                Tool::Inspect => &structured["not_established"],
                Tool::Clean => &structured["report"]["not_established"],
                Tool::Rewrite | Tool::InspectImage | Tool::CleanImage => {
                    unreachable!("report_calls holds Layer A calls only")
                }
            };
            assert_eq!(shelf, &Value::Array(ids.clone()), "{tool:?} {arguments}");
        }
    }

    /// `inspect` reports and changes nothing: no text comes back, no
    /// counters of what was removed — and what it reports is exactly
    /// what `clean` would act on (A §5.2).
    #[test]
    fn inspect_does_not_change_anything() {
        fn has_key(value: &Value, wanted: &str) -> bool {
            match value {
                Value::Object(map) => map
                    .iter()
                    .any(|(key, inner)| key == wanted || has_key(inner, wanted)),
                Value::Array(items) => items.iter().any(|inner| has_key(inner, wanted)),
                _ => false,
            }
        }
        for text in [r#""a\u200Bb""#, r#""p\u0430y""#] {
            for aggressive in [false, true] {
                let arguments = format!(r#"{{"text":{text},"aggressive":{aggressive}}}"#);
                let inspected =
                    called("inspect", &arguments)["result"]["structuredContent"].clone();
                assert!(!has_key(&inspected, "text"), "{arguments}: {inspected}");
                for key in ["removed", "normalized", "output_len"] {
                    assert!(inspected.get(key).is_none(), "{arguments}: {key}");
                }
                let cleaned = called("clean", &arguments)["result"]["structuredContent"].clone();
                assert_eq!(
                    inspected["findings"], cleaned["report"]["findings"],
                    "{arguments}"
                );
                assert_eq!(inspected["kept"], cleaned["report"]["kept"], "{arguments}");
            }
        }
    }

    /// What an agent reads to call these tools: ASCII throughout, the
    /// limit and the unit of a position stated, and nothing promised
    /// that there is no oracle for.
    #[test]
    fn the_tool_listing_says_what_it_takes_and_promises_nothing_more() {
        let body = respond(r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#).expect("an answer");
        assert!(body.is_ascii(), "the listing is not ASCII: {body}");
        for tool in Tool::ALL {
            let description = tool.description();
            assert!(description.contains("1 MB"), "{tool:?}: no limit");
            assert!(description.contains("byte offset"), "{tool:?}: no unit");
            assert!(
                !description.to_lowercase().contains("undetect"),
                "{tool:?} promises what nothing can establish"
            );
        }
        assert!(Tool::Inspect.schema()["properties"].get("nfkc").is_none());
        assert!(Tool::Clean.schema()["properties"].get("nfkc").is_some());
    }

    /// A tool the client invented is the client's mistake, and it
    /// belongs at the protocol level where a client can tell it apart
    /// from a tool that ran and failed.
    #[test]
    fn a_tool_this_build_does_not_have_is_refused_at_the_protocol_level() {
        let response = answer(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"translate","arguments":{}}}"#,
        );
        assert_eq!(response["error"]["code"], json!(INVALID_PARAMS));
        let message = response["error"]["message"].as_str().unwrap_or_default();
        for tool in Tool::ALL {
            assert!(
                message.contains(tool.name()),
                "the refusal does not say what this build does offer"
            );
        }
    }

    /// A model picks a tool by reading its listing. One with no
    /// description, or a schema that does not say what to pass, is a
    /// tool that is called with the wrong arguments or not at all.
    #[test]
    fn every_tool_is_listed_with_something_an_agent_can_act_on() {
        let response = answer(r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#);
        let tools = response["result"]["tools"]
            .as_array()
            .expect("tools/list returns a list")
            .clone();
        assert_eq!(tools.len(), Tool::ALL.len());
        for (listed, tool) in tools.iter().zip(Tool::ALL) {
            assert_eq!(listed["name"], json!(tool.name()));
            assert!(
                listed["description"].as_str().unwrap_or_default().len() > 40,
                "{tool:?} is listed with nothing a model could choose it by"
            );
            assert_eq!(listed["inputSchema"]["type"], json!("object"));
            assert_eq!(listed["inputSchema"]["required"], json!([tool.required()]));
            assert!(listed["inputSchema"]["properties"][tool.required()].is_object());
            assert_eq!(listed["inputSchema"]["additionalProperties"], json!(false));
        }
    }

    /// Two tools that read the same are one tool and a coin toss.
    #[test]
    fn no_two_tools_read_the_same() {
        for (index, tool) in Tool::ALL.iter().enumerate() {
            for other in &Tool::ALL[index + 1..] {
                assert_ne!(tool.name(), other.name());
                assert_ne!(tool.description(), other.description());
            }
        }
    }

    /// The body arrives over a socket from something that is not
    /// necessarily an MCP client. Garbage is an error response, not a
    /// dropped connection and not a panic on the accept thread.
    #[test]
    fn a_body_that_is_not_a_request_is_an_error_and_not_a_panic() {
        for body in [
            "",
            "{",
            "null",
            "[]",
            "[{\"jsonrpc\":\"2.0\"}]",
            "\"hello\"",
        ] {
            let response = answer(body);
            assert!(
                response["error"]["code"].is_i64(),
                "{body:?} did not come back as a JSON-RPC error: {response}"
            );
            assert_eq!(response["jsonrpc"], json!("2.0"));
        }
    }

    /// A request whose method this build has never heard of is refused
    /// by name — including the ones a client will try because the
    /// capabilities did not mention them.
    #[test]
    fn a_method_this_build_does_not_have_is_refused_by_name() {
        for method in ["resources/list", "prompts/list", "completion/complete"] {
            let response = answer(&format!(
                r#"{{"jsonrpc":"2.0","id":4,"method":"{method}"}}"#
            ));
            assert_eq!(response["error"]["code"], json!(METHOD_NOT_FOUND));
            assert!(response["error"]["message"]
                .as_str()
                .unwrap_or_default()
                .contains(method));
        }
    }

    /// The id is how a client matches an answer to a question. Echoing
    /// the wrong one, or dropping it, is a client that waits forever.
    #[test]
    fn the_id_comes_back_exactly_as_it_was_sent() {
        for id in ["1", r#""abc""#, "0", "-4"] {
            let response = answer(&format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"ping"}}"#));
            assert_eq!(
                response["id"],
                serde_json::from_str::<Value>(id).expect("a literal"),
                "the id {id} did not come back"
            );
        }
    }

    /// The gate that makes the "nothing here is localized" rule
    /// enforceable rather than merely written down.
    ///
    /// The application runs the catalogue in `Rendering::Ui`, which
    /// wraps every interpolated value in U+2068/U+2069. Those are
    /// `UnicodeClass::BidiControl` — Layer A removes them — so a
    /// localized response would hand an agent the exact invisible
    /// characters it asked this server to take out. Nothing this
    /// server says may carry one, by any route: not its own words, not
    /// a client's string quoted back, not a report.
    ///
    /// The second half is the one declared exception, counted: the
    /// cleaned text of a `clean` result carries what Layer A *kept*,
    /// because it is the user's text. A forbidden character is allowed
    /// there only as often as the same response's report says it was
    /// kept (twice per kept character: the text block and the
    /// structured content), or — for U+FEFF — as the mark the input
    /// began with. Escaping the text instead would hide a character
    /// from this test without hiding it from the agent, which decodes
    /// the JSON.
    #[test]
    fn nothing_the_server_says_carries_an_invisible_character() {
        fn forbidden(character: char) -> bool {
            matches!(
                character,
                '\u{2066}'..='\u{2069}'   // bidi isolates, Fluent's own
                    | '\u{200b}'..='\u{200f}' // zero width and the bidi marks
                    | '\u{202a}'..='\u{202e}' // the embedding overrides
                    | '\u{feff}'              // the byte order mark, mid-string
                    | '\u{e0000}'..='\u{e007f}' // tag characters
            )
        }

        // Every method `dispatch` answers, and both ways of failing to
        // be a request. A method missing from this list is a method
        // that can go on carrying an isolate unnoticed, which is how
        // the first version of this test passed while `ping` had one.
        let mut requests = vec![
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#.to_owned(),
            r#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#.to_owned(),
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#.to_owned(),
            r#"{"jsonrpc":"2.0","id":4,"method":"nonsense"}"#.to_owned(),
            r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"translate"}}"#
                .to_owned(),
            r#"{"jsonrpc":"2.0","id":6,"method":"tools/call"}"#.to_owned(),
            "{".to_owned(),
            "[]".to_owned(),
            // The three places an answer quotes the client.
            r#"{"jsonrpc":"2.0","id":8,"method":"ping\u200B"}"#.to_owned(),
            r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"cl\u200Bean"}}"#
                .to_owned(),
            r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"inspect","arguments":{"text":"a","te\u200Bxt":"b"}}}"#
                .to_owned(),
        ];
        requests.extend(Tool::ALL.map(|tool| {
            format!(
                r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"{}"}}}}"#,
                tool.name()
            )
        }));
        // And the reports, over a text carrying a zero-width space, an
        // override and a tag character — none of which Layer A keeps.
        let marked = r#""a\u200Bb\u202Ec\uDB40\uDC41d""#;
        for aggressive in [false, true] {
            requests.push(format!(
                r#"{{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{{"name":"inspect","arguments":{{"text":{marked},"aggressive":{aggressive}}}}}}}"#
            ));
            for nfkc in [false, true] {
                requests.push(format!(
                    r#"{{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{{"name":"clean","arguments":{{"text":{marked},"aggressive":{aggressive},"nfkc":{nfkc}}}}}}}"#
                ));
            }
        }
        // And the picture tools, run and refused: over a PNG whose keyword
        // carries a zero-width space and an isolate as Latin-1 cannot — so
        // a soft hyphen and a C1 control — and with a client's invisible
        // characters in a value and a name.
        let sly = super::super::image::encode(&png_with_text(b"Co\xADmm\x9Bent", b"x"));
        for (tool, extra) in [
            ("inspect_image", ""),
            ("clean_image", ""),
            ("clean_image", r#","scope":"all-metadata""#),
            ("clean_image", r#","scope":"all\u200B""#),
            ("inspect_image", r#","da\u2068ta":1"#),
        ] {
            requests.push(format!(
                r#"{{"jsonrpc":"2.0","id":16,"method":"tools/call","params":{{"name":"{tool}","arguments":{{"data":"{sly}"{extra}}}}}}}"#
            ));
        }
        for request in requests {
            let body = respond(&request).expect("an answer");
            for character in body.chars() {
                assert!(
                    !forbidden(character),
                    "the answer to {request} carries U+{:04X}",
                    character as u32
                );
            }
        }

        // And `rewrite`, run and refused: over a marked text, with a model
        // that slips a zero-width space and an isolate into its answer, and
        // with values that carry invisible characters said back.
        let sly = FakeEngine::answering(|req, _| format!("{}\u{200B}\u{2068}", swapped(req)));
        let (services, _) = serving(sly);
        let mut rewrites = vec![format!(
            r#"{{"jsonrpc":"2.0","id":14,"method":"tools/call","params":{{"name":"rewrite","arguments":{{"text":{marked}}}}}}}"#
        )];
        for arguments in [
            r#"{"text":"a","tactic":"para\u200Bphrase"}"#,
            r#"{"text":"a","format":"\u2066html"}"#,
            r#"{"text":"a","intensity":"stro\u202Eng"}"#,
            r#"{"text":"a","te\u200Bxt":"b"}"#,
            r#"{"text":"a","dry_run":true}"#,
        ] {
            rewrites.push(format!(
                r#"{{"jsonrpc":"2.0","id":15,"method":"tools/call","params":{{"name":"rewrite","arguments":{arguments}}}}}"#
            ));
        }
        for request in rewrites {
            let body = respond_with(&request, &services, &|| false).expect("an answer");
            for character in body.chars() {
                assert!(
                    !forbidden(character),
                    "the answer to {request} carries U+{:04X}",
                    character as u32
                );
            }
        }

        // The declared exceptions, counted.
        let family =
            r#""\uD83D\uDC69\u200D\uD83D\uDC69\u200D\uD83D\uDC67\u200D\uD83D\uDC66 a\u200Bb""#;
        let body = respond(&format!(
            r#"{{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{{"name":"clean","arguments":{{"text":{family}}}}}}}"#
        ))
        .expect("an answer");
        let carried: Vec<char> = body.chars().filter(|c| forbidden(*c)).collect();
        assert_eq!(
            carried,
            vec!['\u{200d}'; 6],
            "the family's joiners, twice each"
        );
        let response: Value = serde_json::from_str(&body).expect("JSON");
        let kept = &response["result"]["structuredContent"]["report"]["kept"];
        assert_eq!(kept[0]["codepoint"], json!("U+200D"), "{kept}");
        assert_eq!(kept[0]["count"], json!(3), "{kept}");

        let body = respond(
            r#"{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"clean","arguments":{"text":"\uFEFFa\u200Bb"}}}"#,
        )
        .expect("an answer");
        let carried: Vec<char> = body.chars().filter(|c| forbidden(*c)).collect();
        assert_eq!(carried, vec!['\u{feff}'; 2], "the leading mark, twice");
    }

    /// An English paragraph `lang::detect` reads as English.
    const PARAGRAPH: &str = "The build takes about twelve minutes on an ordinary laptop, and the \
                             second run is much faster because all of the dependencies are \
                             already compiled and kept in the target directory.";

    /// The text a request asks to rewrite, every two neighbouring words of
    /// its first half swapped: an answer every guard accepts and the no-op
    /// floor (0.2) does not call a copy.
    fn swapped(req: &wipemark_engine::ChatRequest) -> String {
        const BEGIN: &str = "[[[BEGIN TEXT]]]\n";
        const END: &str = "\n[[[END TEXT]]]";
        let start = req.prompt.find(BEGIN).map_or(0, |at| at + BEGIN.len());
        let stop = req.prompt[start..]
            .find(END)
            .map_or(req.prompt.len(), |at| at + start);
        let mut words: Vec<&str> = req.prompt[start..stop].split(' ').collect();
        let half = words.len() / 2;
        for pair in words[..half].chunks_mut(2) {
            pair.reverse();
        }
        words.join(" ")
    }

    /// Services over `engine`, priced as an endpoint at twenty tokens a
    /// second, and what the handle would tell a host.
    fn serving(engine: FakeEngine) -> (Services, flume::Receiver<crate::engine_host::Event>) {
        let (handle, inbox) = crate::engine_host::EngineHandle::serving(
            std::sync::Arc::new(engine),
            crate::engine_host::Pace {
                executor: Some(wipemark_pipeline::cost::Executor::Endpoint),
                tokens_per_second: Some(20.0),
            },
        );
        (
            Services {
                rewriter: Some(Rewriter::new(handle, None)),
            },
            inbox,
        )
    }

    /// `tools/call rewrite` with these arguments, answered over `services`.
    fn rewritten(services: &Services, arguments: &str, gone: &dyn Fn() -> bool) -> Value {
        let body = respond_with(
            &format!(
                r#"{{"jsonrpc":"2.0","id":21,"method":"tools/call","params":{{"name":"rewrite","arguments":{arguments}}}}}"#
            ),
            services,
            gone,
        )
        .expect("a request with an id is owed an answer");
        serde_json::from_str(&body).expect("the server rendered something that is not JSON")
    }

    /// The headline: a paragraph rewritten on the engine on duty, answered
    /// as `{text, report}` in both places — the report the job's own JSON,
    /// ASCII, with its third shelf, its seed and every attempt.
    #[test]
    fn a_rewrite_answers_the_text_and_its_report() {
        let (services, inbox) = serving(FakeEngine::answering(|req, _| swapped(req)));
        let arguments = format!(r#"{{"text":{},"seed":7}}"#, json!(PARAGRAPH));
        let response = rewritten(&services, &arguments, &|| false);
        let result = &response["result"];
        assert_eq!(result["isError"], json!(false), "{response}");

        let structured = &result["structuredContent"];
        let block: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().expect("a text block"))
                .expect("the text block is JSON");
        assert_eq!(&block, structured);
        let keys: Vec<&String> = structured.as_object().expect("an object").keys().collect();
        assert_eq!(keys.len(), 2, "{keys:?}");

        let text = structured["text"].as_str().expect("the text");
        assert_ne!(text, PARAGRAPH, "the text came back as it went");
        assert!(text.starts_with("build The about takes"), "{text}");

        let report = &structured["report"];
        assert_eq!(report["version"], json!(2));
        assert_eq!(report["best_effort"]["base_seed"], json!(7));
        assert_eq!(report["best_effort"]["ladder"], json!(["paraphrase"]));
        assert_eq!(
            report["best_effort"]["candidates"],
            json!(2),
            "an endpoint's D61"
        );
        assert_eq!(report["best_effort"]["totals"]["rewritten"], json!(1));
        let shelf = report["not_established"]
            .as_array()
            .expect("the third shelf");
        assert!(!shelf.is_empty());
        assert!(shelf.contains(&json!("unknown-mark-schemes")), "{shelf:?}");
        for key in ["before", "after"] {
            assert!(report["verifiable"][key]["findings"].is_array(), "{key}");
        }

        // One job, announced once and let go of once — the second when the
        // job's thread lets go of the engine, a moment after it answered.
        let wait = std::time::Duration::from_secs(5);
        assert_eq!(
            inbox.recv_timeout(wait),
            Ok(crate::engine_host::Event::JobStarted)
        );
        assert_eq!(
            inbox.recv_timeout(wait),
            Ok(crate::engine_host::Event::JobEnded)
        );
        assert!(inbox.is_empty(), "one job, one start and one end");
    }

    /// Every way a rewrite cannot run is a refusal a model reads, naming
    /// what is wrong — and never the input handed back as a rewrite.
    #[test]
    fn a_rewrite_that_cannot_run_refuses_by_name() {
        let input = "Words nobody may have back as a rewrite.";
        let call = format!(r#"{{"text":{}}}"#, json!(input));

        let unrun = |services: &Services, said: &str| {
            let response = rewritten(services, &call, &|| false);
            let text = refusal_text(&response);
            assert!(text.contains("`rewrite` did not run"), "{text}");
            assert!(text.contains(said), "{said:?} missing from {text}");
            assert!(!text.contains(input), "the input came back: {text}");
        };
        // No road to an engine, and a road with nobody at the end of it.
        unrun(&Services::default(), "no way to an engine");
        let (nobody, _) = crate::engine_host::EngineHandle::new();
        unrun(
            &Services {
                rewriter: Some(Rewriter::new(nobody, None)),
            },
            "nothing is on duty",
        );

        let (services, inbox) = serving(FakeEngine::answering(|req, _| swapped(req)));
        for (arguments, named) in [
            (
                r#"{"text":"a","tactic":"summarize"}"#,
                "`tactic` is `summarize`",
            ),
            (
                r#"{"text":"a","tactic":"structural"}"#,
                "`structural` is not run here",
            ),
            (r#"{"text":"a","tactic":"code"}"#, "`code` is not run here"),
            (r#"{"text":"a","tactic":5}"#, "`tactic` must be a string"),
            (
                r#"{"text":"a","intensity":"extreme"}"#,
                "`intensity` is `extreme`",
            ),
            (r#"{"text":"a","format":"code"}"#, "`format` is `code`"),
            (r#"{"text":"a","candidates":0}"#, "`candidates` must be"),
            (r#"{"text":"a","candidates":9}"#, "`candidates` must be"),
            (r#"{"text":"a","rounds":"2"}"#, "`rounds` must be"),
            (r#"{"text":"a","rounds":1.5}"#, "`rounds` must be"),
            (r#"{"text":"a","seed":-1}"#, "`seed` must be"),
            (r#"{"text":"a","dry_run":"yes"}"#, "`dry_run` must be"),
            (
                r#"{"text":"a","temperature":0.2}"#,
                "no argument `temperature`",
            ),
            (r#"{"tactic":"humanize"}"#, "`text` is missing"),
            (r#"{"text":"a","templates":[]}"#, "`templates` must be"),
            (
                r#"{"text":"a","templates":{"prompts.fr.paraphrase.1.user":"x"}}"#,
                "`prompts.fr.paraphrase.1.user`, which is not a template row",
            ),
            (
                r#"{"text":"a","templates":{"prompts.en.paraphrase.1.user":"Again. {PROTECTED}"}}"#,
                "breaks the rule `missing-variable`",
            ),
        ] {
            let text = refusal_text(&rewritten(&services, arguments, &|| false));
            assert!(text.contains(named), "{arguments}: {text}");
        }
        assert!(inbox.is_empty(), "a refused call started a job");
    }

    /// A call waits for its job — and a client that hangs up, or a ceiling
    /// that passes, cancels it rather than leaving a model writing for
    /// nobody.
    #[test]
    fn a_client_that_hangs_up_cancels_its_rewrite() {
        let slow = || {
            FakeEngine::answering(|req, _| swapped(req))
                .with_token_delay(std::time::Duration::from_millis(30))
        };
        let call = format!(r#"{{"text":{}}}"#, json!(PARAGRAPH));

        let (services, _) = serving(slow());
        let text = refusal_text(&rewritten(&services, &call, &|| true));
        assert!(text.contains("closed the connection"), "{text}");

        let (handle, _) = crate::engine_host::EngineHandle::serving(
            std::sync::Arc::new(slow()),
            crate::engine_host::Pace::default(),
        );
        let impatient = Services {
            rewriter: Some(Rewriter::new(handle, None).with_ceiling(std::time::Duration::ZERO)),
        };
        let text = refusal_text(&rewritten(&impatient, &call, &|| false));
        assert!(text.contains("minutes and was cancelled"), "{text}");
    }

    /// The price before a run (D61): calls, tokens and — with a measured
    /// rate — seconds, for the effort the run would take; and nothing
    /// asked of the model, nothing loaded, no job begun.
    #[test]
    fn a_dry_run_prices_and_loads_nothing() {
        let engine = FakeEngine::answering(|req, _| swapped(req));
        let (services, inbox) = serving(engine.clone());
        let arguments = format!(r#"{{"text":{},"dry_run":true}}"#, json!(PARAGRAPH));
        let response = rewritten(&services, &arguments, &|| false);
        let result = &response["result"];
        assert_eq!(result["isError"], json!(false), "{response}");
        let price = &result["structuredContent"];
        assert!(
            price.get("text").is_none(),
            "a price is not a rewrite: {price}"
        );
        assert!(
            price.get("report").is_none(),
            "a price is not a report: {price}"
        );
        assert_eq!(price["executor"], json!("endpoint"));
        assert_eq!(price["candidates"], json!(2));
        assert_eq!(price["cost"]["chunks"], json!(1));
        assert_eq!(
            price["cost"]["calls"]["worst"],
            json!(4),
            "2 candidates x 2 rounds"
        );
        assert_eq!(price["cost"]["calls"]["expected"], json!(2));
        assert!(
            price["cost"]["seconds"]["worst"].as_f64().unwrap_or(0.0) > 0.0,
            "{price}"
        );

        assert!(engine.asked().is_empty(), "the model was asked");
        assert!(
            inbox.is_empty(),
            "a price began a job: {:?}",
            inbox.drain().collect::<Vec<_>>()
        );
    }

    /// The lists a refusal and the schema give are the lists the pipeline
    /// runs.
    #[test]
    fn the_rewrite_lists_what_it_runs() {
        assert_eq!(TACTICS, asked::OFFERED.map(Tactic::as_str).join(", "));
        assert_eq!(
            INTENSITIES,
            Intensity::ALL.map(Intensity::as_str).join(", ")
        );
        assert_eq!(FORMATS, asked::FORMATS.map(asked::format_id).join(", "));
        let schema = Tool::Rewrite.schema();
        for name in Tool::Rewrite.arguments() {
            assert!(
                schema["properties"].get(*name).is_some(),
                "{name} has no schema"
            );
            assert!(
                Tool::Rewrite.argument_list().contains(name),
                "{name} is not listed"
            );
        }
    }

    // ─── inspect_image and clean_image ─────────────────────────────────

    fn fixture(name: &str) -> Vec<u8> {
        let path = format!("{}/../../fixtures/image/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
    }

    /// A PNG of one grey pixel's header and one `tEXt` chunk, CRCs zero
    /// (the library does not read them): the smallest picture a keyword
    /// can be smuggled into.
    fn png_with_text(key: &[u8], value: &[u8]) -> Vec<u8> {
        let mut text = key.to_vec();
        text.push(0);
        text.extend_from_slice(value);
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        for (kind, data) in [
            (&b"IHDR"[..], &[0, 0, 0, 1, 0, 0, 0, 1, 8, 0, 0, 0, 0][..]),
            (b"tEXt", &text),
            (b"IEND", b""),
        ] {
            png.extend_from_slice(&(data.len() as u32).to_be_bytes());
            png.extend_from_slice(kind);
            png.extend_from_slice(data);
            png.extend_from_slice(&[0; 4]);
        }
        png
    }

    fn image_call(tool: &str, bytes: &[u8], extra: &str) -> Value {
        called(
            tool,
            &format!(
                r#"{{"data":"{}"{extra}}}"#,
                super::super::image::encode(bytes)
            ),
        )
    }

    /// Every picture tool answers with a report carrying the third shelf,
    /// the pixel domain on it, in the text block and the structured content
    /// alike — and the text block is ASCII.
    #[test]
    fn every_image_answer_carries_the_third_shelf() {
        let ids: Vec<Value> = wipemark_core::report::not_established::ALL
            .iter()
            .map(|(id, _)| json!(id))
            .collect();
        for name in [
            "c2pa-jumbf.jpg",
            "xmp-provenance.jpg",
            "xmp-provenance-url.png",
            "exif-xmp.webp",
        ] {
            let bytes = fixture(name);
            for (tool, extra) in [
                ("inspect_image", ""),
                ("clean_image", ""),
                ("clean_image", r#","scope":"all-metadata""#),
            ] {
                let response = image_call(tool, &bytes, extra);
                let result = &response["result"];
                assert_eq!(result["isError"], json!(false), "{name} {tool}: {response}");
                let text = result["content"][0]["text"].as_str().expect("a text block");
                assert!(text.is_ascii(), "{name} {tool}");
                let parsed: Value = serde_json::from_str(text).expect("JSON");
                assert_eq!(&parsed, &result["structuredContent"], "{name} {tool}");
                let report = if tool == "clean_image" {
                    &parsed["report"]
                } else {
                    &parsed
                };
                assert_eq!(
                    report["not_established"],
                    Value::Array(ids.clone()),
                    "{name} {tool}"
                );
                assert!(ids.contains(&json!(wipemark_image::PIXEL_DOMAIN)));
            }
        }
    }

    /// What goes in comes back byte for byte when there is nothing to
    /// remove; and what comes back from a marked picture inspects clean of
    /// provenance.
    #[test]
    fn the_base64_round_trip_is_byte_identical() {
        let plain = fixture("exif-xmp.webp");
        let response = image_call("clean_image", &plain, "");
        let data = response["result"]["structuredContent"]["data"]
            .as_str()
            .expect("an image");
        assert_eq!(
            super::super::image::decode(data),
            Some(plain),
            "the bytes moved"
        );

        let marked = fixture("c2pa-jumbf.jpg");
        let response = image_call("clean_image", &marked, "");
        let data = response["result"]["structuredContent"]["data"]
            .as_str()
            .expect("an image");
        let cleaned = super::super::image::decode(data).expect("base64");
        assert!(cleaned.len() < marked.len());
        let again = image_call("inspect_image", &cleaned, "");
        assert_eq!(
            again["result"]["structuredContent"]["ai_metadata"],
            json!(false),
            "{again}"
        );
        assert_eq!(
            response["result"]["structuredContent"]["report"]["still_has_c2pa"],
            json!(false)
        );
    }

    /// Every way a picture tool can fail to run is an `isError` result
    /// naming why, with no report and no image.
    #[test]
    fn every_image_refusal_is_an_error_result_naming_why() {
        let jpeg = fixture("c2pa-jumbf.jpg");
        let mut mpf = vec![0xFF, 0xD8, 0xFF, 0xE2, 0x00, 0x0E];
        mpf.extend_from_slice(b"MPF\x00MM\x00*\x00\x00\x00\x08");
        mpf.extend_from_slice(&jpeg[2..]);
        let encoded = |bytes: &[u8]| super::super::image::encode(bytes);
        for (tool, arguments, says) in [
            ("inspect_image", json!({}), "`data` is missing".to_owned()),
            (
                "inspect_image",
                json!({ "data": 5 }),
                "must be a string".to_owned(),
            ),
            (
                "inspect_image",
                json!({ "data": "@@@" }),
                "base64".to_owned(),
            ),
            (
                "clean_image",
                json!({ "data": "Zm9v\n" }),
                "base64".to_owned(),
            ),
            (
                "inspect_image",
                json!({ "data": encoded(b"hello, world\n") }),
                "not an image this server reads".to_owned(),
            ),
            (
                "inspect_image",
                json!({ "data": encoded(b"II*\x00\x08\x00\x00\x00\x00\x00\x00\x00") }),
                "TIFF images are not in this version yet".to_owned(),
            ),
            (
                "clean_image",
                json!({ "data": encoded(&jpeg[..jpeg.len() / 2]) }),
                "the JPEG could not be read".to_owned(),
            ),
            (
                "clean_image",
                json!({ "data": encoded(&mpf) }),
                "(MPF)".to_owned(),
            ),
            (
                "clean_image",
                json!({ "data": encoded(&jpeg), "scope": "everything" }),
                "`scope` is `everything`".to_owned(),
            ),
            (
                "inspect_image",
                json!({ "data": encoded(&jpeg), "scope": "all-metadata" }),
                "takes no argument `scope`".to_owned(),
            ),
            (
                "clean_image",
                json!({ "data": encoded(&jpeg), "path": "/etc/passwd" }),
                "takes no argument `path`".to_owned(),
            ),
        ] {
            let response = called(tool, &arguments.to_string());
            let text = refusal_text(&response);
            assert!(text.contains(&says), "{tool} {arguments}: {text}");
            assert!(text.contains(tool), "{text}");
            assert!(
                !text.contains("E11"),
                "an epic number left the repository: {text}"
            );
        }
    }

    /// A keyword is the file's Latin-1, and a soft hyphen, a no-break space
    /// or a C1 control in it is said back spelled: every answer of a picture
    /// tool is ASCII from the first byte to the last — refusals included,
    /// with a client's own invisible characters in its arguments.
    #[test]
    fn no_image_answer_carries_a_character_it_read_out_of_the_file() {
        let sly = png_with_text(b"Co\xADmm\xA0ent\x9B", b"a holiday");
        let mut bodies = Vec::new();
        for (tool, extra) in [
            ("inspect_image", ""),
            ("clean_image", ""),
            ("clean_image", r#","scope":"all-metadata""#),
            ("clean_image", r#","scope":"all\u200B""#),
            ("inspect_image", r#","da\u2068ta":1"#),
        ] {
            let request = format!(
                r#"{{"jsonrpc":"2.0","id":20,"method":"tools/call","params":{{"name":"{tool}","arguments":{{"data":"{}"{extra}}}}}}}"#,
                super::super::image::encode(&sly)
            );
            bodies.push(respond(&request).expect("an answer"));
        }
        bodies.push(
            respond(
                r#"{"jsonrpc":"2.0","id":21,"method":"tools/call","params":{"name":"inspect_image","arguments":{"data":"\u200BZm9v"}}}"#,
            )
            .expect("an answer"),
        );
        // The answers themselves are ASCII to the byte; a refusal of an
        // argument is the scrubber tools' sentence, whose one non-ASCII
        // character is its own dash.
        for body in &bodies[..3] {
            assert!(body.is_ascii(), "{body}");
        }
        for body in &bodies[3..] {
            assert!(
                body.chars().all(|c| c.is_ascii() || c == '\u{2014}'),
                "{body}"
            );
        }
        let inspected: Value = serde_json::from_str(&bodies[0]).expect("JSON");
        assert_eq!(
            inspected["result"]["structuredContent"]["findings"][0]["key"],
            json!("CoU+00ADmmU+00A0entU+009B"),
            "{inspected}"
        );
    }

    /// `tools/list` gives each picture tool a JSON schema: `data` required,
    /// `scope` from its two values on `clean_image` alone, nothing else
    /// accepted.
    #[test]
    fn each_image_tool_has_a_schema() {
        let inspect = Tool::InspectImage.schema();
        assert_eq!(inspect["required"], json!(["data"]));
        assert_eq!(
            inspect["properties"]["data"]["contentEncoding"],
            json!("base64")
        );
        assert!(inspect["properties"].get("scope").is_none());
        assert!(inspect["properties"].get("text").is_none());
        let clean = Tool::CleanImage.schema();
        assert_eq!(
            clean["properties"]["scope"]["enum"],
            json!(["ai-provenance", "all-metadata"])
        );
        for tool in [Tool::InspectImage, Tool::CleanImage] {
            assert_eq!(tool.schema()["additionalProperties"], json!(false));
            let description = tool.description();
            assert!(description.contains("never the pixels"), "{tool:?}");
            assert!(!description.contains("path"), "{tool:?}: {description}");
        }
    }
}
