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

/// The work this server will offer, one variant per tool.
///
/// Both are Layer A: deterministic, verifiable, and exactly the step an
/// agent should be able to run over text it just produced. Neither
/// takes a path — an agent cleaning its own output has the text in
/// hand, and a tool that read files would have to answer the path
/// containment question (spec §4.4) before it could answer anything
/// else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Inspect,
    Clean,
}

impl Tool {
    /// Every tool, in the order `tools/list` reports them.
    pub const ALL: [Tool; 2] = [Self::Inspect, Self::Clean];

    /// The name a client calls it by. A format.
    pub fn name(self) -> &'static str {
        match self {
            Self::Inspect => "inspect",
            Self::Clean => "clean",
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
        }
    }

    /// The arguments, as the JSON Schema a client validates against.
    ///
    /// `additionalProperties: false` because it is true: an argument the
    /// tool does not take is refused by name rather than ignored.
    pub fn schema(self) -> Value {
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
                },
            }),
        );
        if self == Self::Clean {
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
            "required": ["text"],
            "additionalProperties": false,
        })
    }

    /// The arguments this tool takes, in the order a refusal lists them.
    fn arguments(self) -> &'static [&'static str] {
        match self {
            Self::Inspect => &["text", "aggressive"],
            Self::Clean => &["text", "aggressive", "nfkc"],
        }
    }

    /// The same list, spelled for a sentence.
    fn argument_list(self) -> &'static str {
        match self {
            Self::Inspect => "`text` (a string, required), `aggressive` (true or false)",
            Self::Clean => {
                "`text` (a string, required), `aggressive` (true or false), `nfkc` (true or \
                 false)"
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
                     than answering — a report about text that was never read would say that \
                     nothing was found.",
                    name = self.name(),
                    list = self.argument_list(),
                ),
            }],
            "isError": true,
        })
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
}

impl Problem {
    /// The clause of the refusal that says this one.
    fn said(&self) -> String {
        match self {
            Self::Missing(name) => format!("the argument `{name}` is missing"),
            Self::WrongType { name, wants } => format!("the argument `{name}` must be {wants}"),
            Self::NotTaken(name) => format!("it takes no argument `{name}`"),
        }
    }

    /// The log's word for it: static, because the argument names are
    /// the client's and a log line is not where they go.
    fn kind(&self) -> &'static str {
        match self {
            Self::Missing(_) => "missing text",
            Self::WrongType { name: "text", .. } => "wrong type: text",
            Self::WrongType {
                name: "aggressive", ..
            } => "wrong type: aggressive",
            Self::WrongType { .. } => "wrong type: nfkc",
            Self::NotTaken(_) => "not taken",
        }
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

/// Answer one request body.
///
/// `None` is the honest answer to a notification: JSON-RPC says a
/// message with no `id` gets no reply, and the transport above turns
/// that into `204 No Content`.
pub fn respond(body: &str) -> Option<String> {
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

    let answer = dispatch(method, &params)?;
    // A method that produced an answer for a message carrying no id is
    // a notification that was handled: the work is done, and the reply
    // is not owed. Both halves have to be checked — `notifications/*`
    // is a notification by its name, and `ping` with no id is one by
    // its shape.
    let id = id?;
    Some(render(&with_id(id, answer)))
}

/// The result or error for one method, without the envelope.
fn dispatch(method: &str, params: &Value) -> Option<Answer> {
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

        "tools/call" => Some(call(params)),

        other => Some(Answer::Error {
            code: METHOD_NOT_FOUND,
            message: format!("method not found: {}", spelled(other)),
        }),
    }
}

/// `tools/call`: name → tool → arguments → read → run or refuse.
fn call(params: &Value) -> Answer {
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
    match read_call(tool, arguments) {
        Ok(call) => tool.run(&call),
        // Reported as a result so the model reads the refusal, not as a
        // transport error the client swallows on its way past.
        Err(problems) => Answer::Result(tool.refuse(&problems)),
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
                assert!(
                    text.contains("`text`"),
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
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"rewrite","arguments":{}}}"#,
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
            assert_eq!(listed["inputSchema"]["required"], json!(["text"]));
            assert!(listed["inputSchema"]["properties"]["text"].is_object());
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
            r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"rewrite"}}"#
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
}
