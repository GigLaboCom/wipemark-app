//! JSON-RPC 2.0 and the MCP methods, as functions over values.
//!
//! Nothing here touches a socket. [`respond`] takes the bytes of one
//! request body and gives back the bytes of one response — or `None`,
//! which is a notification and gets no reply at all — so every method
//! this server answers can be tested without binding anything.
//!
//! # The tools refuse, and say why
//!
//! The server is real and the protocol is real; Layer A is epic **E1**
//! and is not. So `tools/list` lists what is coming and `tools/call`
//! refuses by name, with the epic in the message. That is the rule the
//! whole repository keeps — *stubs refuse loudly, and nothing exits 0
//! for work that did not happen* — pointed at a protocol instead of at
//! a shell. An agent that got `{"removed": 0}` back from a scrubber
//! that never ran would file the document as clean, and the next thing
//! it did with that document would be built on it.
//!
//! A refusal is reported the way MCP wants one: a *successful*
//! JSON-RPC response carrying `isError: true`, so the failure reaches
//! the model as a tool result it can read and act on, rather than as a
//! transport error the client swallows before the model sees it. That
//! distinction is the protocol's, not ours — a `-32603` here would be
//! a server that broke, and this server did not break.
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
    pub fn description(self) -> &'static str {
        match self {
            Self::Inspect => {
                "Report the invisible Unicode in a piece of text — zero-width characters, bidi \
                 controls, tag characters, variation selectors, private-use and noncharacter \
                 code points — with a count and a position for each, without changing anything."
            }
            Self::Clean => {
                "Remove the invisible Unicode from a piece of text and return the cleaned text \
                 with a report of exactly what was removed and where. Deterministic: the same \
                 input always gives the same output, and every removal is verifiable."
            }
        }
    }

    /// The arguments, as the JSON Schema a client validates against.
    pub fn schema(self) -> Value {
        let mut properties = Map::new();
        properties.insert(
            "text".to_owned(),
            json!({
                "type": "string",
                "description": "The text to read. Passed in full rather than as a path.",
            }),
        );
        properties.insert(
            "aggressive".to_owned(),
            json!({
                "type": "boolean",
                "default": false,
                "description": "Also act on homoglyphs and exotic spaces. Higher false positive \
                                rate, hence opt-in.",
            }),
        );
        if self == Self::Clean {
            properties.insert(
                "nfkc".to_owned(),
                json!({
                    "type": "boolean",
                    "default": false,
                    "description": "Apply NFKC normalisation. Off by default — it rewrites more \
                                    than provenance marks.",
                }),
            );
        }

        json!({
            "type": "object",
            "properties": Value::Object(properties),
            "required": ["text"],
        })
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

    /// The refusal, in the shape MCP reserves for a tool that ran and
    /// could not do the job.
    fn refusal(self) -> Value {
        json!({
            "content": [{
                "type": "text",
                "text": format!(
                    "`{name}` is not implemented yet. In {IMPLEMENTATION} {VERSION} the protocol, \
                     the tool names and the argument shapes are final, the behaviour is not. \
                     Refusing rather than answering — a report that says nothing was found \
                     because nothing was read is worse than no report at all.",
                    name = self.name(),
                ),
            }],
            "isError": true,
        })
    }
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
            message: format!("method not found: {other}"),
        }),
    }
}

/// `tools/call`, which is the whole of what this build refuses.
fn call(params: &Value) -> Answer {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return Answer::Error {
            code: INVALID_PARAMS,
            message: "tools/call needs a `name`".to_owned(),
        };
    };
    match Tool::named(name) {
        // A tool that exists and cannot run yet. Reported as a result
        // so the model reads the refusal, not as a transport error the
        // client swallows on its way past.
        Some(tool) => Answer::Result(tool.refusal()),
        // A tool that does not exist is the client's mistake and
        // belongs at the protocol level, where a client can tell it
        // apart from a tool that ran.
        None => Answer::Error {
            code: INVALID_PARAMS,
            message: format!(
                "unknown tool: {name}. This build offers: {}",
                Tool::ALL.map(|tool| tool.name()).join(", ")
            ),
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
    /// tool that has not been implemented refuses, names itself, and
    /// says the work did not happen. The failure this catches is the
    /// worst one this build could ship — an agent filing a document as
    /// clean because a scrubber that never ran reported nothing.
    #[test]
    fn a_tool_that_cannot_run_refuses_rather_than_reporting_nothing() {
        for tool in Tool::ALL {
            let response = answer(&format!(
                r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"{}","arguments":{{"text":"hi"}}}}}}"#,
                tool.name()
            ));
            let result = &response["result"];
            assert_eq!(
                result["isError"],
                json!(true),
                "{tool:?} answered as though it had done the work: {response}"
            );
            let text = result["content"][0]["text"]
                .as_str()
                .expect("a refusal a model can read");
            assert!(
                text.contains(tool.name()),
                "the refusal does not name the tool"
            );
            assert!(
                text.contains("not implemented"),
                "the refusal does not say that the work did not happen"
            );
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
    /// server says may carry one, by any route.
    #[test]
    fn nothing_the_server_says_carries_an_invisible_character() {
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
        ];
        requests.extend(Tool::ALL.map(|tool| {
            format!(
                r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"{}"}}}}"#,
                tool.name()
            )
        }));
        for request in requests {
            let body = respond(&request).expect("an answer");
            for character in body.chars() {
                assert!(
                    !matches!(
                        character,
                        '\u{2066}'..='\u{2069}'   // bidi isolates, Fluent's own
                            | '\u{200b}'..='\u{200f}' // zero width and the bidi marks
                            | '\u{202a}'..='\u{202e}' // the embedding overrides
                            | '\u{feff}'              // the byte order mark, mid-string
                            | '\u{e0000}'..='\u{e007f}' // tag characters
                    ),
                    "the answer to {request} carries U+{:04X}",
                    character as u32
                );
            }
        }
    }
}
