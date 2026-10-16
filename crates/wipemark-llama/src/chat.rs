//! New in wipemark (E2-4); not carried over.
//!
//! The chat templates this crate renders itself, because llama.cpp's
//! built-in list either does not know them or renders them for a turn this
//! product never asks for.
//!
//! `llama_chat_apply_template` does not run Jinja: it recognises a template
//! by the markers in it (`llm_chat_detect_template`) and renders a
//! hand-written copy of that family. Two families the models of E2-4 need
//! are missing from that list, at the pin and on llama.cpp's master alike:
//!
//! * **Gemma 4** (`<|turn>` … `<turn|>`) is not in it at all, so every
//!   request was refused before a token (D96).
//! * **ChatML with a thinking switch** (Qwen3.8, Qwen3's hybrid models) is
//!   recognised as plain ChatML and rendered with a bare `assistant`
//!   opener — which is the template's *thinking on*. The model then writes
//!   a `<think>` block before its answer, and a rewrite comes back with its
//!   reasoning in it.
//!
//! Both are rendered here, from what the model's own Jinja template
//! produces for one optional system message and one user message with
//! `add_generation_prompt` and `enable_thinking = false` — the only
//! conversation this product ever sends. The expected strings in the tests
//! were checked against llama.cpp's own Jinja engine at the pin
//! (`llama-server --jinja`, `POST /apply-template`, E2-4's report), so the
//! tests pin both the recognition and the rendering.
//!
//! A family is recognised by the markers its rendering writes, the way
//! llama.cpp recognises its own, and nothing is guessed: a template that is
//! neither of these goes to `llama_chat_apply_template`, and one that is not
//! in llama.cpp's list either is refused. No BOS is written — the tokenizer
//! adds it, as it does for llama.cpp's renderings.
//!
//! # Which templates are written at all (E8-1)
//!
//! [`chat_support`] is the one verdict, and it is asked before anything is
//! rendered: one of the two families above; or a family llama.cpp's own
//! detection at the pin recognises — [`llama_cpp_family`], a port of
//! `llm_chat_detect_template` (`src/llama-chat.cpp` at `b10731`), marker
//! for marker and in its order; or **refused by name** — no template, or a
//! template neither knows. It is pure, so a model nobody has loaded can be
//! judged from its GGUF header (the Models page's dialog does, before a
//! model is added), and the load refuses what it refuses, before a request
//! (`wipemark_engine::LocalEngine`). The port decides only *whether*: a
//! recognised template still goes to llama.cpp as the model's own string,
//! so a rendering is llama.cpp's exactly as before. In a native build,
//! `the_port_agrees_with_llama_cpp` holds the port to llama.cpp's answer
//! over one template of every family.
//!
//! Pure string work, compiled and tested in every build.

/// A template family rendered here rather than by llama.cpp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Family {
    /// Gemma 4: `<|turn>role\n…<turn|>\n`. The 12B's template closes an
    /// empty thought channel after the model's opener when thinking is off;
    /// the E2B's and E4B's do not, and write the bare opener —
    /// `thought_closed` is which, read off the template itself.
    Gemma4 { thought_closed: bool },
    /// ChatML whose template has an `enable_thinking` switch: the
    /// assistant's opener followed by an empty `<think>` block.
    ChatmlThinkingOff,
}

/// The family `template` (a model's Jinja chat template) belongs to, when
/// it is one rendered here.
pub(crate) fn family_of(template: &str) -> Option<Family> {
    let has = |marker: &str| template.contains(marker);
    if has("<|turn>") && has("<turn|>") {
        return Some(Family::Gemma4 {
            thought_closed: has(r"<|channel>thought\n<channel|>"),
        });
    }
    // llama.cpp's own ChatML test, minus the two families it carves out of
    // it (Phi-4's `<|im_sep|>`, SmolVLM's `<end_of_utterance>`), plus the
    // switch and the empty block the template writes when it is off. The
    // block is matched as the template spells it: a Jinja string literal
    // with `\n` escapes, backslashes included.
    if has("<|im_start|>")
        && !has("<|im_sep|>")
        && !has("<end_of_utterance>")
        && has("enable_thinking")
        && has(r"<think>\n\n</think>\n\n")
    {
        return Some(Family::ChatmlThinkingOff);
    }
    None
}

/// Whether a conversation can be written for a model whose chat template is
/// `template` — and who writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatSupport {
    /// Rendered by this crate: the family's name.
    Here(&'static str),
    /// Rendered by `llama_chat_apply_template`, which recognises it as the
    /// family of this name.
    LlamaCpp(&'static str),
    /// The model carries no chat template, or an empty one.
    NoTemplate,
    /// A template neither this crate nor llama.cpp at the pin recognises.
    Unrecognised,
}

impl ChatSupport {
    /// Whether a conversation can be written at all.
    #[must_use]
    pub fn is_supported(self) -> bool {
        matches!(self, ChatSupport::Here(_) | ChatSupport::LlamaCpp(_))
    }

    /// The family's name, when there is one.
    #[must_use]
    pub fn family(self) -> Option<&'static str> {
        match self {
            ChatSupport::Here(family) | ChatSupport::LlamaCpp(family) => Some(family),
            ChatSupport::NoTemplate | ChatSupport::Unrecognised => None,
        }
    }
}

/// The verdict for a model whose chat template is `template` (its GGUF's
/// `tokenizer.chat_template`, or `None`): this crate's families first, then
/// llama.cpp's, and nothing guessed.
#[must_use]
pub fn chat_support(template: Option<&str>) -> ChatSupport {
    let Some(template) = template.filter(|t| !t.trim().is_empty()) else {
        return ChatSupport::NoTemplate;
    };
    // A NUL inside: llama.cpp hands the template on as a C string, so the
    // load would see only what stands before it — another template than the
    // one the file carries, which is a guess. Refused, as the load is left
    // to refuse it (the follow-ups of E8-1, B-L7).
    if template.contains('\0') {
        return ChatSupport::Unrecognised;
    }
    if let Some(family) = family_of(template) {
        return ChatSupport::Here(match family {
            Family::Gemma4 { .. } => "gemma4",
            Family::ChatmlThinkingOff => "chatml-thinking-off",
        });
    }
    match llama_cpp_family(template) {
        Some(family) => ChatSupport::LlamaCpp(family),
        None => ChatSupport::Unrecognised,
    }
}

/// The names `llama_chat_apply_template` takes for its built-in templates
/// at the pin (`LLM_CHAT_TEMPLATES`, `src/llama-chat.cpp`). A template that
/// *is* one of them is that family.
const LLAMA_CPP_NAMES: [&str; 54] = [
    "chatml",
    "llama2",
    "llama2-sys",
    "llama2-sys-bos",
    "llama2-sys-strip",
    "mistral-v1",
    "mistral-v3",
    "mistral-v3-tekken",
    "mistral-v7",
    "mistral-v7-tekken",
    "phi3",
    "phi4",
    "falcon3",
    "zephyr",
    "monarch",
    "gemma",
    "orion",
    "openchat",
    "vicuna",
    "vicuna-orca",
    "deepseek",
    "deepseek2",
    "deepseek3",
    "deepseek-ocr",
    "command-r",
    "llama3",
    "chatglm3",
    "chatglm4",
    "glmedge",
    "minicpm",
    "exaone3",
    "exaone4",
    "exaone-moe",
    "rwkv-world",
    "granite",
    "granite-4.0",
    "granite-4.1",
    "gigachat",
    "megrez",
    "yandex",
    "bailing",
    "bailing-think",
    "bailing2",
    "llama4",
    "smolvlm",
    "hunyuan-moe",
    "gpt-oss",
    "hunyuan-dense",
    "hunyuan-vl",
    "kimi-k2",
    "seed_oss",
    "grok-2",
    "pangu-embedded",
    "solar-open",
];

/// The family llama.cpp at the pin recognises `template` as, by its own
/// markers and in its own order — `llm_chat_detect_template`, ported line
/// for line. `None` is `LLM_CHAT_TEMPLATE_UNKNOWN`: llama.cpp would refuse
/// to render it.
#[must_use]
pub fn llama_cpp_family(template: &str) -> Option<&'static str> {
    if let Some(name) = LLAMA_CPP_NAMES.iter().find(|name| **name == template) {
        return Some(name);
    }
    let has = |marker: &str| template.contains(marker);
    if has("<|im_start|>") {
        return Some(if has("<|im_sep|>") {
            "phi4"
        } else if has("<end_of_utterance>") {
            "smolvlm"
        } else {
            "chatml"
        });
    }
    if template.starts_with("mistral") || has("[INST]") {
        if has("[SYSTEM_PROMPT]") {
            return Some("mistral-v7");
        }
        if has("' [INST] ' + system_message") || has("[AVAILABLE_TOOLS]") {
            if has(" [INST]") {
                return Some("mistral-v1");
            }
            if has(r#""[INST]""#) {
                return Some("mistral-v3-tekken");
            }
            return Some("mistral-v3");
        }
        return Some(if has("content.strip()") {
            "llama2-sys-strip"
        } else if has("bos_token + '[INST]") {
            "llama2-sys-bos"
        } else if has("<<SYS>>") {
            "llama2-sys"
        } else {
            "llama2"
        });
    }
    if has("<|assistant|>") && has("<|end|>") {
        return Some("phi3");
    }
    if has("[gMASK]<sop>") {
        return Some("chatglm4");
    }
    if has("<|assistant|>") && has("<|user|>") {
        if has("<|tool_declare|>") {
            return Some("exaone-moe");
        }
        return Some(if has("</s>") { "falcon3" } else { "glmedge" });
    }
    if has("<|{{ item['role'] }}|>") && has("<|begin_of_image|>") {
        return Some("glmedge");
    }
    if has("<|user|>") && has("<|endoftext|>") {
        return Some("zephyr");
    }
    if has("bos_token + message['role']") {
        return Some("monarch");
    }
    if has("<start_of_turn>") {
        return Some("gemma");
    }
    if has(r"'\n\nAssistant: ' + eos_token") {
        return Some("orion");
    }
    if has("GPT4 Correct ") {
        return Some("openchat");
    }
    if has("USER: ") && has("ASSISTANT: ") {
        return Some(if has("SYSTEM: ") {
            "vicuna-orca"
        } else {
            "vicuna"
        });
    }
    if has("### Instruction:") && has("<|EOT|>") {
        return Some("deepseek");
    }
    if has("<|START_OF_TURN_TOKEN|>") && has("<|USER_TOKEN|>") {
        return Some("command-r");
    }
    if has("<|start_header_id|>") && has("<|end_header_id|>") {
        return Some("llama3");
    }
    if has("[gMASK]sop") {
        return Some("chatglm3");
    }
    if has("<\u{7528}\u{6237}>") {
        return Some("minicpm");
    }
    if has("'Assistant: ' + message['content'] + eos_token") {
        return Some("deepseek2");
    }
    if has("<\u{FF5C}Assistant\u{FF5C}>")
        && has("<\u{FF5C}User\u{FF5C}>")
        && has("<\u{FF5C}end\u{2581}of\u{2581}sentence\u{FF5C}>")
    {
        return Some("deepseek3");
    }
    if has("[|system|]") && has("[|assistant|]") && has("[|endofturn|]") {
        return Some(if has("[|tool|]") {
            "exaone4"
        } else {
            "exaone3"
        });
    }
    if has("rwkv-world") || has(r"{{- 'User: ' + message['content']|trim + '\n\n' -}}") {
        return Some("rwkv-world");
    }
    if has("<|start_of_role|>") {
        if has("<tool_call>") || has("<tools>") {
            return Some(if has("g4_default_system_message") {
                "granite-4.0"
            } else {
                "granite-4.1"
            });
        }
        return Some("granite");
    }
    if has(
        "message['role'] + additional_special_tokens[0] + message['content'] + additional_special_tokens[1]",
    ) {
        return Some("gigachat");
    }
    if has("<|role_start|>") {
        return Some("megrez");
    }
    if has(" \u{410}\u{441}\u{441}\u{438}\u{441}\u{442}\u{435}\u{43D}\u{442}:") {
        return Some("yandex");
    }
    if has("<role>ASSISTANT</role>") && has("'HUMAN'") {
        return Some("bailing");
    }
    if has("<role>ASSISTANT</role>") && has(r#""HUMAN""#) && has("<think>") {
        return Some("bailing-think");
    }
    if has("<role>ASSISTANT</role>") && has("<role>HUMAN</role>") && has("<|role_end|>") {
        return Some("bailing2");
    }
    if has("<|header_start|>") && has("<|header_end|>") {
        return Some("llama4");
    }
    if has("<|endofuserprompt|>") {
        return Some("dots1");
    }
    if has("<|extra_0|>") && has("<|extra_4|>") {
        return Some("hunyuan-moe");
    }
    if has("<|start|>") && has("<|channel|>") {
        return Some("gpt-oss");
    }
    if has("<\u{FF5C}hy_Assistant\u{FF5C}>")
        && has("<\u{FF5C}hy_begin\u{2581}of\u{2581}sentence\u{FF5C}>")
    {
        return Some("hunyuan-vl");
    }
    if has("<\u{FF5C}hy_Assistant\u{FF5C}>")
        && has("<\u{FF5C}hy_place\u{2581}holder\u{2581}no\u{2581}3\u{FF5C}>")
    {
        return Some("hunyuan-dense");
    }
    if has("<|im_assistant|>assistant<|im_middle|>") {
        return Some("kimi-k2");
    }
    if has("<seed:bos>") {
        return Some("seed_oss");
    }
    if has("'Assistant: '  + message['content'] + '<|separator|>") {
        return Some("grok-2");
    }
    if has("[unused9]\u{7CFB}\u{7EDF}\u{FF1A}[unused10]") {
        return Some("pangu-embedded");
    }
    if has("<|begin|>") && has("<|end|>") && has("<|content|>") {
        return Some("solar-open");
    }
    None
}

/// `system` (when given) and `user`, rendered for `family`, ending with the
/// model's opener — thinking off. Each message is trimmed, as both
/// templates do (`| trim`).
pub(crate) fn render(family: Family, system: Option<&str>, user: &str) -> String {
    let system = system.map(str::trim);
    let user = user.trim();
    let mut out = String::with_capacity(system.map_or(0, str::len) + user.len() + 96);
    match family {
        Family::Gemma4 { thought_closed } => {
            // Written whenever the first message is a system message, even
            // one that trims to nothing.
            if let Some(system) = system {
                out.push_str("<|turn>system\n");
                out.push_str(system);
                out.push_str("<turn|>\n");
            }
            out.push_str("<|turn>user\n");
            out.push_str(user);
            out.push_str("<turn|>\n");
            out.push_str("<|turn>model\n");
            if thought_closed {
                out.push_str("<|channel>thought\n<channel|>");
            }
        }
        Family::ChatmlThinkingOff => {
            // Written only for a system message with something in it.
            if let Some(system) = system.filter(|s| !s.is_empty()) {
                out.push_str("<|im_start|>system\n");
                out.push_str(system);
                out.push_str("<|im_end|>\n");
            }
            out.push_str("<|im_start|>user\n");
            out.push_str(user);
            out.push_str("<|im_end|>\n");
            out.push_str("<|im_start|>assistant\n<think>\n\n</think>\n\n");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        chat_support, family_of, llama_cpp_family, render, ChatSupport, Family, LLAMA_CPP_NAMES,
    };

    /// One template per family llama.cpp's detection names at the pin, as
    /// small as its markers allow, and what it is — then three it does not
    /// know. The table the native test holds against llama.cpp itself.
    const FAMILIES: &[(&str, Option<&str>)] = &[
        ("{{ bos_token }}<|im_start|>user", Some("chatml")),
        ("<|im_start|>user<|im_sep|>", Some("phi4")),
        ("<|im_start|>User:<end_of_utterance>", Some("smolvlm")),
        ("[SYSTEM_PROMPT] [INST]", Some("mistral-v7")),
        ("{{ ' [INST] ' + system_message }}", Some("mistral-v1")),
        (r#"[AVAILABLE_TOOLS]{{ "[INST]" }}"#, Some("mistral-v3-tekken")),
        ("[AVAILABLE_TOOLS][INST]", Some("mistral-v3")),
        ("[INST] {{ content.strip() }}", Some("llama2-sys-strip")),
        ("{{ bos_token + '[INST] ' }}", Some("llama2-sys-bos")),
        ("[INST] <<SYS>>", Some("llama2-sys")),
        ("[INST]", Some("llama2")),
        ("<|assistant|><|end|>", Some("phi3")),
        ("[gMASK]<sop>", Some("chatglm4")),
        ("<|user|><|assistant|><|tool_declare|>", Some("exaone-moe")),
        ("<|user|><|assistant|></s>", Some("falcon3")),
        ("<|user|><|assistant|>", Some("glmedge")),
        ("<|{{ item['role'] }}|><|begin_of_image|>", Some("glmedge")),
        ("<|user|><|endoftext|>", Some("zephyr")),
        ("{{ bos_token + message['role'] }}", Some("monarch")),
        ("<start_of_turn>user", Some("gemma")),
        (r"{{ '\n\nAssistant: ' + eos_token }}", Some("orion")),
        ("GPT4 Correct User", Some("openchat")),
        ("USER: ASSISTANT: ", Some("vicuna")),
        ("SYSTEM: USER: ASSISTANT: ", Some("vicuna-orca")),
        ("### Instruction:<|EOT|>", Some("deepseek")),
        ("<|START_OF_TURN_TOKEN|><|USER_TOKEN|>", Some("command-r")),
        ("<|start_header_id|>user<|end_header_id|>", Some("llama3")),
        ("[gMASK]sop", Some("chatglm3")),
        ("<\u{7528}\u{6237}>", Some("minicpm")),
        (
            "{{ 'Assistant: ' + message['content'] + eos_token }}",
            Some("deepseek2"),
        ),
        (
            "<\u{FF5C}Assistant\u{FF5C}><\u{FF5C}User\u{FF5C}><\u{FF5C}end\u{2581}of\u{2581}sentence\u{FF5C}>",
            Some("deepseek3"),
        ),
        ("[|system|][|assistant|][|endofturn|]", Some("exaone3")),
        ("[|system|][|assistant|][|endofturn|][|tool|]", Some("exaone4")),
        (
            r"{{- 'User: ' + message['content']|trim + '\n\n' -}}",
            Some("rwkv-world"),
        ),
        ("<|start_of_role|>", Some("granite")),
        (
            "<|start_of_role|><tools>g4_default_system_message",
            Some("granite-4.0"),
        ),
        ("<|start_of_role|><tool_call>", Some("granite-4.1")),
        (
            "message['role'] + additional_special_tokens[0] + message['content'] + additional_special_tokens[1]",
            Some("gigachat"),
        ),
        ("<|role_start|>", Some("megrez")),
        (
            " \u{410}\u{441}\u{441}\u{438}\u{441}\u{442}\u{435}\u{43D}\u{442}:",
            Some("yandex"),
        ),
        ("<role>ASSISTANT</role>'HUMAN'", Some("bailing")),
        (
            r#"<role>ASSISTANT</role>"HUMAN"<think>"#,
            Some("bailing-think"),
        ),
        (
            "<role>ASSISTANT</role><role>HUMAN</role><|role_end|>",
            Some("bailing2"),
        ),
        ("<|header_start|><|header_end|>", Some("llama4")),
        ("<|endofuserprompt|>", Some("dots1")),
        ("<|extra_0|><|extra_4|>", Some("hunyuan-moe")),
        ("<|start|><|channel|>", Some("gpt-oss")),
        (
            "<\u{FF5C}hy_Assistant\u{FF5C}><\u{FF5C}hy_begin\u{2581}of\u{2581}sentence\u{FF5C}>",
            Some("hunyuan-vl"),
        ),
        (
            "<\u{FF5C}hy_Assistant\u{FF5C}><\u{FF5C}hy_place\u{2581}holder\u{2581}no\u{2581}3\u{FF5C}>",
            Some("hunyuan-dense"),
        ),
        ("<|im_assistant|>assistant<|im_middle|>", Some("kimi-k2")),
        ("<seed:bos>", Some("seed_oss")),
        (
            "'Assistant: '  + message['content'] + '<|separator|>",
            Some("grok-2"),
        ),
        (
            "[unused9]\u{7CFB}\u{7EDF}\u{FF1A}[unused10]",
            Some("pangu-embedded"),
        ),
        ("<|begin|><|end|><|content|>", Some("solar-open")),
        ("{{ messages }}", None),
        ("<|turn>user\n<turn|>", None),
        ("Hello", None),
    ];

    /// E8-1: every family llama.cpp detects is recognised by the markers it
    /// detects it by, and a template it does not know is not.
    #[test]
    fn every_family_is_recognised_by_llama_cpps_own_markers() {
        for (template, family) in FAMILIES {
            assert_eq!(llama_cpp_family(template), *family, "{template:?}");
        }
        // And a template that is one of llama.cpp's names is that family.
        for name in LLAMA_CPP_NAMES {
            assert_eq!(llama_cpp_family(name), Some(name), "{name}");
        }
    }

    /// E8-1: the verdict is this crate's families first, then llama.cpp's,
    /// and a refusal by name otherwise — never a guess.
    #[test]
    fn the_verdict_is_ours_then_llama_cpps_and_nothing_is_guessed() {
        assert_eq!(chat_support(Some(GEMMA_4)), ChatSupport::Here("gemma4"));
        assert_eq!(
            chat_support(Some(QWEN_3_8)),
            ChatSupport::Here("chatml-thinking-off"),
            "Qwen3.8 is ChatML to llama.cpp, and ours before it"
        );
        assert_eq!(
            chat_support(Some(QWEN_3_INSTRUCT)),
            ChatSupport::LlamaCpp("chatml")
        );
        assert_eq!(chat_support(Some(GEMMA_3)), ChatSupport::LlamaCpp("gemma"));
        assert_eq!(chat_support(None), ChatSupport::NoTemplate);
        // B-L7: a template with a NUL inside is refused, whatever stands on
        // either side of it — the load reads only up to the NUL.
        assert_eq!(
            chat_support(Some(&format!("{GEMMA_3}\0{GEMMA_4}"))),
            ChatSupport::Unrecognised
        );
        assert_eq!(
            chat_support(Some(&format!("x\0{QWEN_3_8}"))),
            ChatSupport::Unrecognised
        );
        assert_eq!(chat_support(Some("  \n")), ChatSupport::NoTemplate);
        assert_eq!(
            chat_support(Some("{% for m in messages %}{{ m.content }}{% endfor %}")),
            ChatSupport::Unrecognised
        );
        assert!(ChatSupport::Here("gemma4").is_supported());
        assert!(ChatSupport::LlamaCpp("gemma").is_supported());
        assert!(!ChatSupport::NoTemplate.is_supported());
        assert!(!ChatSupport::Unrecognised.is_supported());
        assert_eq!(ChatSupport::LlamaCpp("gemma").family(), Some("gemma"));
        assert_eq!(ChatSupport::Unrecognised.family(), None);
    }

    /// The port and llama.cpp at the pin agree on every template of the
    /// table: the one llama.cpp renders is the one the port recognises.
    /// Model-free — `llama_chat_apply_template` needs no model.
    #[cfg(feature = "native")]
    #[test]
    fn the_port_agrees_with_llama_cpp() {
        for (template, _) in FAMILIES {
            assert_eq!(
                llama_cpp_family(template).is_some(),
                crate::ffi::llama_cpp_renders(template),
                "{template:?}"
            );
        }
        for template in [QWEN_3_INSTRUCT, GEMMA_3, GEMMA_4, QWEN_3_8] {
            assert_eq!(
                llama_cpp_family(template).is_some(),
                crate::ffi::llama_cpp_renders(template),
                "{template:?}"
            );
        }
    }

    /// The parts of Gemma 4 12B it's template (`tokenizer.chat_template`
    /// of `gemma-4-12B-it-qat-UD-Q4_K_XL.gguf`) the recognition reads.
    const GEMMA_4: &str = "{{- bos_token -}}\n\
        {%- if (enable_thinking is defined and enable_thinking) or tools or messages[0]['role'] in ['system', 'developer'] -%}\n\
        {{- '<|turn>system\\n' -}}\n{{- '<turn|>\\n' -}}\n{%- endif %}\n\
        {{- '<|turn>' + role + '\\n' }}\n\
        {%- if add_generation_prompt -%}{{- '<|turn>model\\n' -}}\
        {%- if not enable_thinking | default(false) -%}{{- '<|channel>thought\\n<channel|>' -}}{%- endif -%}{%- endif -%}";

    /// The same generation prompt as Gemma 4 E2B's and E4B's write it: the
    /// bare opener, whatever `enable_thinking` says.
    const GEMMA_4_SMALL: &str = "{{- '<|turn>system\\n' -}}{{- '<turn|>\\n' -}}\n\
        {%- if add_generation_prompt -%}{{- '<|turn>model\\n' -}}{%- endif -%}";

    /// The parts of Qwen3.8 27B's template the recognition reads.
    const QWEN_3_8: &str = "{%- if enable_thinking is undefined or enable_thinking is true %}\n\
        {{- '<|im_start|>system\\n' }}\n\
        {%- if add_generation_prompt %}\n    {{- '<|im_start|>assistant\\n' }}\n\
        {%- if enable_thinking is defined and enable_thinking is false %}\n\
        {{- '<think>\\n\\n</think>\\n\\n' }}\n{%- else %}\n{{- '<think>\\n' }}\n{%- endif %}\n{%- endif %}";

    /// Qwen3 4B Instruct 2507's: ChatML, and no switch — it never thinks.
    const QWEN_3_INSTRUCT: &str = "{%- for message in messages %}\
        {{- '<|im_start|>' + message.role + '\\n' + content + '<|im_end|>' + '\\n' }}{%- endfor %}\
        {%- if add_generation_prompt %}{{- '<|im_start|>assistant\\n' }}{%- endif %}";

    /// Gemma 3's: llama.cpp's built-in `gemma`.
    const GEMMA_3: &str = "{{ bos_token }}{% for message in messages %}\
        {{ '<start_of_turn>' + role + '\\n' + message['content'] | trim + '<end_of_turn>\\n' }}{% endfor %}\
        {% if add_generation_prompt %}{{'<start_of_turn>model\\n'}}{% endif %}";

    #[test]
    fn the_two_families_are_recognised_by_their_own_markers() {
        assert_eq!(
            family_of(GEMMA_4),
            Some(Family::Gemma4 {
                thought_closed: true
            })
        );
        assert_eq!(
            family_of(GEMMA_4_SMALL),
            Some(Family::Gemma4 {
                thought_closed: false
            })
        );
        assert_eq!(family_of(QWEN_3_8), Some(Family::ChatmlThinkingOff));
    }

    #[test]
    fn a_template_llama_cpp_renders_is_left_to_it() {
        // Plain ChatML with no switch, Gemma 3, and nothing at all: each
        // goes to `llama_chat_apply_template`, which knows the first two
        // and refuses the third.
        assert_eq!(family_of(QWEN_3_INSTRUCT), None);
        assert_eq!(family_of(GEMMA_3), None);
        assert_eq!(family_of(""), None);
        // A switch without the empty block it writes is not this family:
        // the opener for "off" is not known, so it is not guessed.
        assert_eq!(
            family_of("<|im_start|>assistant\n{% if enable_thinking %}{% endif %}"),
            None
        );
        // Phi-4 is ChatML-shaped and is llama.cpp's.
        assert_eq!(
            family_of(&format!("{QWEN_3_8}<|im_sep|>")),
            None,
            "Phi-4's separator"
        );
    }

    // The two strings below are what llama.cpp's Jinja engine at the pin
    // renders from the real GGUFs' templates for the same messages, with
    // `enable_thinking: false` and `add_generation_prompt: true`, BOS
    // removed (`llama-server --jinja`, `POST /apply-template`; E2-4's
    // report records the call).

    const GEMMA_4_12B: Family = Family::Gemma4 {
        thought_closed: true,
    };
    const GEMMA_4_E4B: Family = Family::Gemma4 {
        thought_closed: false,
    };

    #[test]
    fn gemma_4_is_rendered_as_its_template_renders_it() {
        assert_eq!(
            render(
                GEMMA_4_12B,
                Some("  Rewrite the text.\n"),
                "The quick brown fox.\n\n"
            ),
            "<|turn>system\nRewrite the text.<turn|>\n\
             <|turn>user\nThe quick brown fox.<turn|>\n\
             <|turn>model\n<|channel>thought\n<channel|>"
        );
        assert_eq!(
            render(GEMMA_4_12B, None, "Привет."),
            "<|turn>user\nПривет.<turn|>\n<|turn>model\n<|channel>thought\n<channel|>"
        );
        // E2B and E4B: the same turns, and the bare opener.
        assert_eq!(
            render(GEMMA_4_E4B, Some("Rewrite the text."), "Привет."),
            "<|turn>system\nRewrite the text.<turn|>\n<|turn>user\nПривет.<turn|>\n<|turn>model\n"
        );
    }

    #[test]
    fn chatml_with_a_switch_is_rendered_with_thinking_off() {
        assert_eq!(
            render(
                Family::ChatmlThinkingOff,
                Some("Rewrite the text.\n"),
                " The quick brown fox."
            ),
            "<|im_start|>system\nRewrite the text.<|im_end|>\n\
             <|im_start|>user\nThe quick brown fox.<|im_end|>\n\
             <|im_start|>assistant\n<think>\n\n</think>\n\n"
        );
        assert_eq!(
            render(Family::ChatmlThinkingOff, None, "Привет."),
            "<|im_start|>user\nПривет.<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
        );
    }

    #[test]
    fn an_empty_system_message_is_what_each_template_makes_of_it() {
        // Qwen3.8's writes the system turn only for a message with
        // something in it once trimmed; Gemma 4's writes it for any.
        assert_eq!(
            render(Family::ChatmlThinkingOff, Some(" \n"), "x"),
            render(Family::ChatmlThinkingOff, None, "x"),
        );
        assert_eq!(
            render(GEMMA_4_12B, Some(" \n"), "x"),
            "<|turn>system\n<turn|>\n<|turn>user\nx<turn|>\n<|turn>model\n<|channel>thought\n<channel|>"
        );
    }
}
