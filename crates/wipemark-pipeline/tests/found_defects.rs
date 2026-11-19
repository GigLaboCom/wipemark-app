//! What every guard passed on 2026-11-19, and the guards of D856–D860 and
//! D862 now refuse. Ten technical articles were rewritten under Keep voice by a
//! 12B model and every changed block judged; these are the answers the
//! judge — or a person — caught, word for word as the model gave them,
//! each against its chunk as `prepare` makes it from the source around it
//! and judged by the loop's own verdict.

use wipemark_core::{GuardOutcome, RejectReason};
use wipemark_pipeline::cost::Executor;
use wipemark_pipeline::job::verdict;
use wipemark_pipeline::prepare::{prepare, Budget, Chunk, TextFormat};
use wipemark_pipeline::report::Rejection;
use wipemark_pipeline::Options;

fn options() -> Options {
    Options::for_executor(Executor::Endpoint)
}

/// Chunk `index` of the Markdown `source`, its text pinned to `text` so the
/// placeholders the answers name are the ones the model saw.
fn chunk(source: &str, index: usize, text: &str) -> Chunk {
    let prepared = prepare(source, TextFormat::Markdown, Budget::DEFAULT);
    let chunk = prepared.chunks()[index].clone();
    assert_eq!(chunk.text, text);
    chunk
}

/// The loop's verdict on `answer`: refused by the guard `name`.
fn refused_by(chunk: &Chunk, answer: &str, name: &str) -> RejectReason {
    match verdict(&options(), chunk, answer) {
        Err(Rejection::Guard { guard, reason }) if guard == name => reason,
        other => panic!("expected the {name} guard to refuse it, got {other:?}"),
    }
}

/// What the guard `name`, as the loop builds it for `chunk`, says on its own.
fn alone(chunk: &Chunk, answer: &str, name: &str) -> GuardOutcome {
    let guards = options().guards_for(chunk);
    let guard = guards
        .iter()
        .find(|g| g.name() == name)
        .unwrap_or_else(|| panic!("no {name} guard for this chunk"));
    guard.check(&chunk.text, answer)
}

/// Article 19: the list's last item, given as context, opened the next
/// paragraph's answer verbatim — the item twice in the document, the second
/// time without its `- `, and a paragraph of its own.
#[test]
fn the_list_s_last_item_echoed_into_the_next_paragraph_is_refused() {
    let source = "- **A `list_` for every identifier a tool needs.** `capture_display` takes a display, so there is `list_displays`. `apply_preset` takes a preset, so there is `list_presets`. If an argument can only be learned by trial and error, the agent will learn it by trial and error.\n\
- **One job per tool.** Naming a capture is `assign_keyword`, a separate call, not a flag on four capture tools.\n\
\n\
The specification's own guidance is short: names of 1 to 128 characters from letters, digits, underscore, hyphen and dot, unique within a server. The 2026-07-28 revision adds that servers should return `tools/list` in a deterministic order, so clients can cache it and prompt caches keep hitting.\n";
    let chunk = chunk(
        source,
        2,
        "The specification's own guidance is short: names of 1 to 128 characters from letters, digits, underscore, hyphen and dot, unique within a server. The 2026-07-28 revision adds that servers should return \u{27E6}1\u{27E7} in a deterministic order, so clients can cache it and prompt caches keep hitting.",
    );
    assert_eq!(
        chunk.context.as_deref(),
        Some("**One job per tool.** Naming a capture is `assign_keyword`, a separate call, not a flag on four capture tools.")
    );
    let answer = "**One job per tool.** Naming a capture is `assign_keyword`, a separate call, not a flag on four capture tools.\n\nGuidelines from the specification are brief: a name must be unique on a server and consist of 1 to 128 characters using dots, underscores, hyphens, digits, and letters. It is noted in the 2026-07-28 revision that \u{27E6}1\u{27E7} should be returned by servers in a deterministic order to allow clients to cache it and ensure prompt caches keep hitting.";
    // The paragraph it added is the first finding…
    assert_eq!(
        refused_by(&chunk, answer, "paragraphs"),
        RejectReason::ParagraphsChanged {
            source: 0,
            candidate: 1
        }
    );
    // …and the echo on its own is refused too.
    assert!(
        matches!(
            alone(&chunk, answer, "context-echo"),
            GuardOutcome::Reject(RejectReason::ContextEchoed { words: 19, .. })
        ),
        "{:?}",
        alone(&chunk, answer, "context-echo")
    );
}

/// Article 12: a screenshot's caption, the chunk before, came back glued to
/// the paragraph after the heading — once rewritten in place, once verbatim
/// here. No paragraph was added: only the echo guard sees it.
#[test]
fn a_caption_glued_onto_the_next_paragraph_is_refused() {
    let source = "*[screenshot: the capture flash over a single app window, with the editor and terminal beside it untouched]*\n\
\n\
## Base64 or file paths\n\
\n\
Control loops pass screenshots inline, because the model has to look at every frame in order to act. Anthropic's documentation notes that long loops accumulate images quickly, each costing a thousand tokens or more. A directed capture doesn't have to be read at the moment it is taken, so Lazy Shot's tools return a file path: about 20 tokens <!-- verify --> of context, plus a file the agent opens only if it needs to. The agent can take a before and an after and then look at just one of them. The full argument, including the cases where base64 is the right choice, is in our [design patterns for MCP servers](/blog/mcp-server-design-patterns).\n";
    let chunk = chunk(
        source,
        1,
        "Control loops pass screenshots inline, because the model has to look at every frame in order to act. Anthropic's documentation notes that long loops accumulate images quickly, each costing a thousand tokens or more. A directed capture doesn't have to be read at the moment it is taken, so Lazy Shot's tools return a file path: about 20 tokens \u{27E6}1\u{27E7} of context, plus a file the agent opens only if it needs to. The agent can take a before and an after and then look at just one of them. The full argument, including the cases where base64 is the right choice, is in our \u{27E6}2\u{27E7}design patterns for MCP servers\u{27E6}3\u{27E7}.",
    );
    let answer = "*[screenshot: the capture flash over a single app window, with the editor and terminal beside it untouched]*\nSince the model requires viewing every frame to function, control loops provide screenshots inline. Documentation from Anthropic mentions that images in long loops build up rapidly, with each one requiring 1,000 tokens or more. Because a directed capture does not require immediate reading, the tools in Lazy Shot provide a file path: \u{27E6}1\u{27E7} of context (roughly 20 tokens) plus a file that the agent only accesses if necessary. It is possible for the agent to capture a \"before\" and an \"after\" and then examine only one of those. Our \u{27E6}2\u{27E7}design patterns for MCP servers\u{27E6}3\u{27E7} contain the complete argument, including instances where base64 is the correct option.";
    assert_eq!(
        refused_by(&chunk, answer, "context-echo"),
        RejectReason::ContextEchoed {
            words: 17,
            text: "screenshot: the capture flash over a single app window, with the editor and terminal beside it untouched".to_owned()
        }
    );
}

/// Article 15: the numbered callouts kept their numbers in order and lost
/// their items — 1 is now the pager.
#[test]
fn numbered_callouts_whose_items_turned_around_are_refused() {
    let source = "Annotate that page once. Capture it, drop numbered callouts on the regions that matter (1 on the filter rail, 2 on the date range, 3 on the export menu, 4 on the pager), and save the layout as a marker preset. That is a site map: a screenshot that says where things are, in numbers the agent can refer to.\n";
    let chunk = chunk(source, 0, source.trim_end());
    let answer = "Mark that specific page one time. Save it, placing numbered labels on the significant areas (1 on the pager, 2 on the export menu, 3 on the date range, 4 on the filter rail), then store the arrangement as a preset marker. This represents a site map: a captured image that identifies locations using numbers for the agent to use as references.";
    assert_eq!(
        refused_by(&chunk, answer, "order"),
        RejectReason::OrderReversed {
            first: "filter".to_owned(),
            last: "pager".to_owned()
        }
    );
}

/// Article 20: a series "there in order" came back from its end.
#[test]
fn a_series_in_order_that_comes_back_from_its_end_is_refused() {
    let source = "Three months later the bug is back, or something that looks like it. Search the library for `bug-482` and the series is there in order: the original, the annotated copy, each iteration, the after-shot.\n";
    let chunk = chunk(
        source,
        0,
        "Three months later the bug is back, or something that looks like it. Search the library for \u{27E6}1\u{27E7} and the series is there in order: the original, the annotated copy, each iteration, the after-shot.",
    );
    let answer = "After 3 months, the bug has reappeared, or at least a very similar issue. When you look through the library for \u{27E6}1\u{27E7}, you can find the series in sequence: the after-shot, every iteration, the annotated copy, and finally, the original.";
    assert_eq!(
        refused_by(&chunk, answer, "order"),
        RejectReason::OrderReversed {
            first: "original".to_owned(),
            last: "shot".to_owned()
        }
    );
}

/// Article 11: the parts of a command, in the order they are typed, came
/// back from the last.
#[test]
fn a_command_s_parts_named_backwards_are_refused() {
    let source = "That is the current syntax per the [Claude Code MCP documentation](https://code.claude.com/docs/en/mcp): transport flag, a name of your choice, the URL. The default scope is `local`, meaning this project on your machine only. Add `--scope user` to have it in every project, which fits a tool that lives on your machine anyway. Add `--scope project` to write a `.mcp.json` teammates can share:\n";
    let chunk = chunk(
        source,
        0,
        "That is the current syntax per the \u{27E6}1\u{27E7}Claude Code MCP documentation\u{27E6}2\u{27E7}: transport flag, a name of your choice, the URL. The default scope is \u{27E6}3\u{27E7}, meaning this project on your machine only. Add \u{27E6}4\u{27E7} to have it in every project, which fits a tool that lives on your machine anyway. Add \u{27E6}5\u{27E7} to write a \u{27E6}6\u{27E7} teammates can share:",
    );
    let answer = "According to the \u{27E6}1\u{27E7}Claude Code MCP documentation\u{27E6}2\u{27E7}, the existing format is: the URL, a name of your choice, and the transport flag. Because the default scope is \u{27E6}3\u{27E7}, this project will only be available on your machine. If you want it to be in every project, add \u{27E6}4\u{27E7}, as this is a tool that stays on your machine regardless. To create a \u{27E6}6\u{27E7} that your teammates can share, include \u{27E6}5\u{27E7}:";
    assert_eq!(
        refused_by(&chunk, answer, "order"),
        RejectReason::OrderReversed {
            first: "transport".to_owned(),
            last: "URL".to_owned()
        }
    );
}

/// Article 19: three groups of tools, named in the order the article then
/// walks through them, came back backwards. No word of the sentence says
/// the order is meant — it is the document's, three sections later — so
/// the order guard lets it through since D862, as it lets "a route, a
/// fixture, a dev flag" through. A line in the contracts asking the model
/// to keep every list's order was measured and left out (D863).
#[test]
fn tool_groups_listed_backwards_pass_the_order_guard() {
    let source = "We sort the tools into three groups: **Capture** (take a picture), **Manage** (find, name and remove pictures in the library) and **Markers** (draw on them). The protocol has no notion of a tool group. The grouping lives in the documentation and the naming, and it gives a model, and the human reading the tool list, a three-item map before a twenty-item one.\n";
    let chunk = chunk(source, 0, source.trim_end());
    let answer = "The tools are organized into 3 categories: **Markers** (draw on them), **Manage** (find, name and remove pictures in the library), and **Capture** (take a picture). Because the protocol lacks a concept for tool groups, this categorization exists solely within the naming and documentation. This provides both a human reading the tool list and a model with a 3-item map rather than a 20-item one.";
    assert_eq!(alone(&chunk, answer, "order"), GuardOutcome::Pass);
}

/// Article 15, under the guards of D856–D861: a quoted instruction of two
/// steps came back with its steps the other way round, and passed — a list
/// had to have three items. A sequence said in words has two (D862).
#[test]
fn a_two_step_instruction_turned_around_is_refused() {
    let source = "From then on the agent calls `list_presets`, finds `orders-admin-map`, and uses `apply_preset` on a fresh capture of the same page. Markers are applied as a fork, so the raw frame stays untouched and the annotated copy sits next to it. The instruction \"set the range in region 2, then export from region 3\" is now unambiguous.\n";
    let chunk = chunk(
        source,
        0,
        "From then on the agent calls \u{27E6}1\u{27E7}, finds \u{27E6}2\u{27E7}, and uses \u{27E6}3\u{27E7} on a fresh capture of the same page. Markers are applied as a fork, so the raw frame stays untouched and the annotated copy sits next to it. The instruction \"set the range in region 2, then export from region 3\" is now unambiguous.",
    );
    let answer = "Starting from that point, \u{27E6}3\u{27E7} is utilized by the agent on a new screenshot of the identical page, after they locate \u{27E6}2\u{27E7} and call \u{27E6}1\u{27E7}. Because markers function as a fork, the original frame remains unchanged while the copy with annotations is placed beside it. The command to \"export from region 3, then set the range in region 2\" is now perfectly clear.";
    assert_eq!(
        refused_by(&chunk, answer, "order"),
        RejectReason::OrderReversed {
            first: "set".to_owned(),
            last: "export".to_owned()
        }
    );
}

/// Article 17: a copy from the library to the destination came back as a
/// copy the other way.
#[test]
fn a_copy_turned_the_other_way_is_refused() {
    let source =
        "The last step is a file copy from the library path to the destination in the manifest:\n";
    let chunk = chunk(source, 0, source.trim_end());
    let answer = "A transfer of the file from the manifest's destination to the library path occurs as the final step:";
    assert_eq!(
        refused_by(&chunk, answer, "order"),
        RejectReason::OrderReversed {
            first: "library".to_owned(),
            last: "destination".to_owned()
        }
    );
}

/// Article 16: a link's opener and closer changed places — every
/// placeholder there once, so the placeholder guard passed it, and the
/// document had "cookbook](…) … at [".
#[test]
fn a_link_whose_brackets_change_places_is_refused() {
    let source = "The full recipe, with an importable workflow JSON <!-- verify -->, is [recipe 08 in the Lazy Shot cookbook](https://github.com/giglabo/lazy-shot-cookbook).\n";
    let chunk = chunk(
        source,
        0,
        "The full recipe, with an importable workflow JSON \u{27E6}1\u{27E7}, is \u{27E6}2\u{27E7}recipe 08 in the Lazy Shot cookbook\u{27E6}3\u{27E7}.",
    );
    assert_eq!(chunk.pairs, [(2, 3)]);
    let answer = "You can find recipe 08 in the Lazy Shot cookbook\u{27E6}3\u{27E7}, which includes the entire set of instructions along with a JSON \u{27E6}1\u{27E7} workflow that can be imported at \u{27E6}2\u{27E7}.";
    assert_eq!(
        refused_by(&chunk, answer, "pairs"),
        RejectReason::PairBroken { open: 2, close: 3 }
    );
}

/// Article 16: a FAQ's question and answer, one paragraph with a line break
/// between them, came back as two paragraphs.
#[test]
fn a_question_split_from_its_answer_is_refused() {
    let source = "**Should we use an AI Agent node with the MCP Client Tool instead?**\n\
For this flow, no. The steps never vary, so the standalone MCP Client node is cheaper and predictable. An agent with MCP Client Tool attached makes sense when the model has to choose what to capture, which a scheduled watch does not.\n";
    let chunk = chunk(source, 0, source.trim_end());
    let answer = "**Is utilizing an MCP Client Tool within an AI Agent node a better option?**\n\nNo, that is not the case for this specific workflow. Because the sequence of actions remains constant, opting for the independent MCP Client node provides greater cost-efficiency and reliability. Using an agent paired with an MCP Client Tool is appropriate only when the model is required to decide what information to collect; this is not necessary for a watch triggered by a schedule.";
    assert_eq!(
        refused_by(&chunk, answer, "paragraphs"),
        RejectReason::ParagraphsChanged {
            source: 0,
            candidate: 1
        }
    );
}

/// Article 11, shipped templates: a bold step heading came back with its
/// words shuffled — the same words and marks, no longer English.
#[test]
fn a_step_whose_words_only_moved_is_refused() {
    let source = "**2. Register it with Claude Code.**\n";
    let chunk = chunk(source, 0, source.trim_end());
    assert_eq!(
        refused_by(
            &chunk,
            "**2. Claude Code with it register.**",
            "permutation"
        ),
        RejectReason::WordsReordered { words: 6 }
    );
}

/// Every guard `guards_for` builds is named in the rules the queue's
/// fingerprint hashes, in its order (D861): a chunk with a construct and a
/// context faces all ten.
#[test]
fn every_guard_a_candidate_faces_is_in_the_rules() {
    let source = "A first paragraph, the context of the next.\n\nSee [the guide](https://example.com/guide) before you start.\n";
    let prepared = prepare(source, TextFormat::Markdown, Budget::DEFAULT);
    let chunk = &prepared.chunks()[1];
    assert!(chunk.context.is_some());
    assert_eq!(chunk.pairs, [(1, 2)]);
    let names: Vec<&str> = options()
        .guards_for(chunk)
        .iter()
        .map(|g| g.name())
        .collect();
    assert_eq!(names, wipemark_pipeline::select::RULES.guards);
    // Without them, the two are not built.
    let first: Vec<&str> = options()
        .guards_for(&prepared.chunks()[0])
        .iter()
        .map(|g| g.name())
        .collect();
    assert_eq!(first, &wipemark_pipeline::select::RULES.guards[..8]);
}
