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
    use super::{family_of, render, Family};

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
