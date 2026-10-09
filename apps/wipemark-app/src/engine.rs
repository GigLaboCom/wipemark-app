//! The Layer B engine settings: the endpoint, the model, the key, and
//! the knobs that ride with a request.
//!
//! Epic **E6 / S6.3** — the *settings*. The requests themselves are
//! `wipemark-engine`'s `HttpEngine` (E2-3), built from these settings by
//! `duty::engine_for`; the only one this build sends is the Engine page's
//! **Check**, a fixed sentence and never a document, and the pane says so
//! out loud rather than implying a rewrite is a click away. `docs/sdd/layer-b-rewrite-reference.md` is where the
//! wire format, the prompts and the security rules were read out of
//! upstream, and §8 of it is the table this module is the other half
//! of.
//!
//! # Two backends, and the difference is not cosmetic
//!
//! [`Provider::Ollama`] speaks Ollama's **native** `/api/chat` — not
//! `/api/generate`, and not the `/v1` OpenAI shim Ollama also serves —
//! which is why `temperature` travels inside an `options` object there
//! and at the top level everywhere else, and why that path carries no
//! `Authorization` header at all.
//! [`Provider::OpenAiCompatible`] is `/v1/chat/completions` and is the
//! one that takes a key: OpenAI, OpenRouter, LM Studio, vLLM, a
//! company's own gateway. One field with presets beside it rather than
//! a closed list of vendors — the same shape, and the same reasoning,
//! as the MCP bind address.
//!
//! # Where the key is, and where it is not
//!
//! Not here, and not in `wipemark.db`. It goes to the operating
//! system's credential store through [`wipemark_secret`], filed under
//! the endpoint's [origin](BaseUrl::origin) — so two providers are two
//! credentials, and pointing the field at a different host does not
//! quietly send the last host's key to it. What this module owns is the
//! *account name* ([`account_of`]) and the rules about when a key may
//! travel at all ([`Refusal`]).
//!
//! # The refusals are the product
//!
//! Three of them, and all three came from upstream's own source rather
//! than from a threat model written afterwards:
//!
//! * a non-loopback endpoint is **default-deny** until
//!   [`EngineSettings::allow_remote`] is set, because "the text leaves
//!   your machine" is a decision and not a side effect of typing a URL;
//! * a key never crosses a plaintext hop — `http://` to anywhere but
//!   this machine is a `Bearer` token on the wire in the clear;
//! * a base URL carrying credentials in its authority
//!   (`https://user:pass@host`) is refused as a URL, because the one
//!   place a credential is allowed to be is the credential store.
//!
//! Redirects are refused too, and that one belongs to the transport: the
//! engine never follows a 3xx (`wipemark_engine::http`) — urllib re-sends
//! `Authorization` on one, and the shape of that bug is identical in every
//! HTTP client. The engine also refuses a scheme other than `http` and
//! `https` itself, which [`BaseUrl::parse`] already cannot produce:
//! defence in depth, not a second rule.

use std::fmt;

use gpui::SharedString;
use gpui_component::select::SelectItem;
use wipemark_engine::http::KeyFault;
use wipemark_i18n::{t, Message};
use wipemark_secret::{Secret, Vault};

/// The default endpoint: Ollama, where it listens, on this machine.
///
/// A default the field can show even while the provider is
/// [`Provider::Off`] — an empty field would make "no engine" look like
/// "half-configured engine".
pub const DEFAULT_BASE_URL: &str = "http://127.0.0.1:11434";

/// Upstream's default and spec §4.4's. High for a paraphrase, and
/// deliberately so: the point of Layer B is that two runs do not
/// produce the same sentence.
pub const DEFAULT_TEMPERATURE: f32 = 0.9;

/// The widest a temperature may be. Above 2.0 most servers reject the
/// request, and the ones that do not produce noise.
pub const HIGHEST_TEMPERATURE: f32 = 2.0;

/// Upstream's `--timeout`, in seconds.
pub const DEFAULT_TIMEOUT: u32 = 120;

/// A minute short of an hour is already far past the point where a user
/// has concluded the application has hung.
pub const LONGEST_TIMEOUT: u32 = 3600;

/// Longer than any URL a browser will keep, which is the only limit
/// with any authority behind it.
pub const LONGEST_URL: usize = 2048;

/// Long enough for `registry.example.com/org/model:tag-with-detail`.
pub const LONGEST_MODEL: usize = 200;

/// Which shape of request the engine will send.
///
/// [`Off`](Self::Off) is a real choice and the default, not an empty
/// state: Layer A is deterministic, never licence-gated and complete on
/// its own, and the status bar already says "no engine configured ·
/// Layer A only" without apologising for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Provider {
    #[default]
    Off,
    Ollama,
    OpenAiCompatible,
}

impl Provider {
    /// Every choice, in the order the selector lists them.
    pub const ALL: [Provider; 3] = [Self::Off, Self::Ollama, Self::OpenAiCompatible];

    /// The stored value, and the word a future `--engine` flag would
    /// take. A format: never translated, and never derived from the
    /// label.
    pub fn id(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Ollama => "ollama",
            Self::OpenAiCompatible => "openai-compatible",
        }
    }

    /// Read a stored or typed value back. `None` for anything else —
    /// the caller's answer is to keep the default and leave the row
    /// alone, the bargain every other preference makes.
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim().to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|provider| provider.id() == value)
    }

    /// The selector's label.
    ///
    /// "Ollama" is a product name and is not translated — the rule
    /// `wipemark_core::Vendor` states and `mcp::Client` follows, and a
    /// user looking for the row that matches the thing they installed
    /// needs to read the name that thing is called. The other two are
    /// descriptions and come from the catalogue like everything else a
    /// person reads for its meaning.
    pub fn label(self) -> String {
        match self {
            Self::Off => t(Message::SettingsEngineProviderOff),
            Self::Ollama => "Ollama".to_owned(),
            Self::OpenAiCompatible => t(Message::SettingsEngineProviderOpenai),
        }
    }

    /// The path a request goes to, under the base URL.
    ///
    /// `None` for [`Off`](Self::Off), which sends nothing anywhere.
    pub fn path(self) -> Option<&'static str> {
        match self {
            Self::Off => None,
            // Native, not `/api/generate` and not the `/v1` shim.
            Self::Ollama => Some("/api/chat"),
            Self::OpenAiCompatible => Some("/v1/chat/completions"),
        }
    }

    /// Whether this provider has anywhere to put a key.
    ///
    /// Ollama's native endpoint has no `Authorization` header on it at
    /// all, so the row is not merely unused there — offering it would
    /// invite a user to store a credential that nothing will ever send.
    pub fn takes_a_key(self) -> bool {
        matches!(self, Self::OpenAiCompatible)
    }

    /// The endpoints worth offering as one click, for this provider.
    ///
    /// Filtered by provider because the wire formats are not
    /// interchangeable: an Ollama base URL under the OpenAI-compatible
    /// provider reaches a `/v1` shim that is not what the Ollama rows
    /// were measured against, and an OpenAI base URL under the Ollama
    /// provider reaches nothing at all.
    pub fn presets(self) -> &'static [Preset] {
        match self {
            Self::Off => &[],
            Self::Ollama => &[Preset::OLLAMA],
            // Ollama is in both lists and is the same base URL in both:
            // its OpenAI-compatible shim is a different *path* on the
            // same port, and the path is the provider's, not the
            // preset's. A user who has Ollama and wants the shim should
            // not have to know that to find the button.
            Self::OpenAiCompatible => &[
                Preset::OLLAMA,
                Preset::LM_STUDIO,
                Preset::OPENAI,
                Preset::OPENROUTER,
            ],
        }
    }
}

/// One endpoint worth a button.
///
/// Both halves are formats. The label is the product's own name, and
/// the URL is read by a machine — a preset that had been translated
/// would be a button that writes a hostname nothing resolves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preset {
    pub label: &'static str,
    pub url: &'static str,
}

impl Preset {
    pub const OLLAMA: Self = Self {
        label: "Ollama",
        url: DEFAULT_BASE_URL,
    };
    pub const LM_STUDIO: Self = Self {
        label: "LM Studio",
        url: "http://127.0.0.1:1234",
    };
    pub const OPENAI: Self = Self {
        label: "OpenAI",
        url: "https://api.openai.com",
    };
    pub const OPENROUTER: Self = Self {
        label: "OpenRouter",
        url: "https://openrouter.ai/api",
    };
}

/// How hard a reasoning model should think about a paraphrase.
///
/// Two spellings that are not interchangeable, which is the whole
/// reason this is five choices and not a slider.
/// [`None`](Self::None) sends `"none"` — a value some servers accept
/// and others reject — and [`Off`](Self::Off) omits the field
/// altogether, which is the only thing that works on the servers that
/// reject it. A single boolean would have stranded one class of
/// endpoint.
///
/// The default is [`None`](Self::None), and the number behind it is
/// upstream's: on a one-line rewrite, a reasoning model spent **9,894
/// completion tokens** on chain-of-thought against 12 without it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReasoningEffort {
    Off,
    #[default]
    None,
    Low,
    Medium,
    High,
}

impl ReasoningEffort {
    pub const ALL: [ReasoningEffort; 5] =
        [Self::Off, Self::None, Self::Low, Self::Medium, Self::High];

    /// The stored value. A format.
    pub fn id(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim().to_ascii_lowercase();
        Self::ALL.into_iter().find(|effort| effort.id() == value)
    }

    /// What goes in the request body, or `None` to leave the field out.
    ///
    /// The one method that distinguishes the two spellings, and the
    /// reason the enum has five variants for four levels.
    #[allow(
        dead_code,
        reason = "the request spells it through `wipemark_engine::Reasoning::wire`, \
                  which `duty::engine_for` maps this row onto; the distinction is \
                  settled here too because the *setting* is, and \
                  `off_omits_the_field_and_none_sends_a_value` holds it"
    )]
    pub fn wire(self) -> Option<&'static str> {
        match self {
            Self::Off => None,
            other => Some(other.id()),
        }
    }

    /// The selector's label. Every one of these is a word rather than a
    /// name, so every one comes from the catalogue.
    pub fn label(self) -> String {
        t(match self {
            Self::Off => Message::SettingsEngineReasoningOff,
            Self::None => Message::SettingsEngineReasoningNone,
            Self::Low => Message::SettingsEngineReasoningLow,
            Self::Medium => Message::SettingsEngineReasoningMedium,
            Self::High => Message::SettingsEngineReasoningHigh,
        })
    }
}

/// The scheme half of a base URL. Two, because upstream's
/// `_check_remote` rejects every other one before it looks at the host
/// and so does this: `file://` and `ftp://` are not endpoints, and a
/// scheme nobody validated is how an SSRF gets its foot in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Http,
    Https,
}

impl Scheme {
    fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
        }
    }

    fn default_port(self) -> u16 {
        match self {
            Self::Http => 80,
            Self::Https => 443,
        }
    }
}

/// Where the engine sends its requests.
///
/// A parsed value rather than a string, for the same reason
/// `mcp::BindAddress` is one: three separate decisions hang off it —
/// whether the text leaves this machine, whether a key would cross the
/// wire in the clear, and which credential belongs to it — and each of
/// those is a bug if it is re-derived from a string at the call site
/// that needs it.
///
/// No `url` crate. This is a base URL, which is a much smaller grammar
/// than a URL: a scheme we accept two of, an authority, an optional
/// port and a path prefix. Queries, fragments and userinfo are all
/// refused rather than parsed, so there is nothing here for a
/// full-grammar parser to be more correct about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseUrl {
    scheme: Scheme,
    /// Lowercased. Bracketless for IPv6 — [`BaseUrl::authority`] puts
    /// the brackets back where a URL needs them.
    host: String,
    /// `None` when the scheme's own port is meant, so that
    /// `https://api.openai.com` and `https://api.openai.com:443` are
    /// one origin and therefore one credential.
    port: Option<u16>,
    /// The prefix a path is appended to. Empty, or starting with `/`
    /// and never ending with one.
    path: String,
}

impl BaseUrl {
    /// Read a stored or typed value back.
    ///
    /// `None` for everything that is not a base URL this build would
    /// send to, and the list of those is the interesting half:
    ///
    /// * a scheme that is not `http` or `https`;
    /// * an authority carrying userinfo — `https://user:key@host` is a
    ///   credential in a settings row, which is the one thing this
    ///   whole design exists to prevent, and quietly dropping it would
    ///   be worse than refusing it;
    /// * a query or a fragment, neither of which a base URL has;
    /// * an empty host, or a port that is not a port.
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        if value.is_empty() || value.len() > LONGEST_URL {
            return None;
        }

        let (scheme, rest) = split_scheme(value)?;
        let rest = rest.trim_end_matches('/');
        if rest.contains(['?', '#']) {
            return None;
        }

        let (authority, path) = match rest.find('/') {
            Some(cut) => (&rest[..cut], &rest[cut..]),
            None => (rest, ""),
        };
        if authority.is_empty() || authority.contains('@') {
            return None;
        }

        let (host, port) = split_port(authority)?;
        if host.is_empty() || host.contains(char::is_whitespace) {
            return None;
        }

        Some(Self {
            scheme,
            host: host.to_ascii_lowercase(),
            port: port.filter(|port| *port != scheme.default_port()),
            path: path.to_owned(),
        })
    }

    /// The host, without the brackets an IPv6 literal wears in a URL.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Whether the bytes stay on this machine.
    ///
    /// Name-based rather than resolved, and deliberately: resolving
    /// would make the answer depend on a DNS lookup that could say
    /// something different a second later, and `localhost` is the
    /// answer a user typed when they meant this machine.
    pub fn is_loopback(&self) -> bool {
        if self.host.eq_ignore_ascii_case("localhost") {
            return true;
        }
        self.host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback())
    }

    /// Whether the hop is encrypted, or does not exist.
    ///
    /// Loopback counts. A `Bearer` token to `127.0.0.1` never reaches a
    /// wire, and demanding TLS from a local Ollama would be a
    /// requirement nobody can satisfy and everybody would work around.
    pub fn is_private(&self) -> bool {
        self.scheme == Scheme::Https || self.is_loopback()
    }

    /// Whether the document goes somewhere else. The banner's whole
    /// question.
    pub fn leaves_this_machine(&self) -> bool {
        !self.is_loopback()
    }

    /// The `host[:port]` a URL is written with, brackets included.
    pub fn authority(&self) -> String {
        let host = if self.host.contains(':') && !self.host.starts_with('[') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        match self.port {
            Some(port) => format!("{host}:{port}"),
            None => host,
        }
    }

    /// Scheme, host and port — and nothing else.
    ///
    /// The account a credential is filed under, which is why the path
    /// is not in it: OpenRouter under `/api` and under `/api/v1` is one
    /// key, and a user who edits the path should not find their key
    /// gone. It is the same boundary a browser draws, for the same
    /// reason.
    pub fn origin(&self) -> String {
        format!("{}://{}", self.scheme.as_str(), self.authority())
    }

    /// Where a request actually goes, for a provider that sends any.
    pub fn endpoint(&self, provider: Provider) -> Option<String> {
        provider
            .path()
            .map(|path| format!("{}{}{path}", self.origin(), self.path))
    }
}

impl Default for BaseUrl {
    fn default() -> Self {
        Self::parse(DEFAULT_BASE_URL).expect("the default base URL is one")
    }
}

impl fmt::Display for BaseUrl {
    /// What the field shows and what the row stores — normalised, so
    /// that a trailing slash or a capitalised host is not a second
    /// spelling of an endpoint that already has a credential.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{}", self.origin(), self.path)
    }
}

fn split_scheme(value: &str) -> Option<(Scheme, &str)> {
    let cut = value.find("://")?;
    let scheme = match &value[..cut] {
        scheme if scheme.eq_ignore_ascii_case("http") => Scheme::Http,
        scheme if scheme.eq_ignore_ascii_case("https") => Scheme::Https,
        _ => return None,
    };
    Some((scheme, &value[cut + 3..]))
}

/// `host`, `host:port`, `[::1]` or `[::1]:port`.
fn split_port(authority: &str) -> Option<(&str, Option<u16>)> {
    if let Some(rest) = authority.strip_prefix('[') {
        let close = rest.find(']')?;
        let host = &rest[..close];
        return match &rest[close + 1..] {
            "" => Some((host, None)),
            tail => tail
                .strip_prefix(':')?
                .parse()
                .ok()
                .map(|p| (host, Some(p))),
        };
    }
    match authority.rsplit_once(':') {
        // A bare IPv6 literal with no brackets. Refused rather than
        // guessed at: `::1:8080` is ambiguous and a URL is why brackets
        // exist.
        Some((host, _)) if host.contains(':') => None,
        Some((host, port)) => port.parse().ok().map(|port| (host, Some(port))),
        None => Some((authority, None)),
    }
}

/// Whether a URL field should accept this text *while it is being
/// typed*.
///
/// The same bargain `mcp::typeable` makes: `http://12` is on the way to
/// an endpoint and a field that refused it could not be typed into. So
/// this judges the characters and the length, [`BaseUrl::parse`] judges
/// the URL once there is a whole one, and
/// `every_endpoint_can_be_reached_one_keystroke_at_a_time` keeps the
/// two from drifting into a field that cannot reach a URL it would then
/// accept.
pub fn typeable_url(typed: &str) -> bool {
    typed.len() <= LONGEST_URL && !typed.chars().any(char::is_control)
}

/// Whether a model field should accept this text as it is typed.
///
/// Deliberately loose. `llama3.1:8b`, `gpt-4o-mini`,
/// `deepseek/deepseek-chat` and `hf.co/org/repo:Q4_K_M` are all model
/// ids somebody has, and this build has no list to check one against —
/// what it can say is that a model id is one token with no spaces in
/// it.
pub fn typeable_model(typed: &str) -> bool {
    typed.len() <= LONGEST_MODEL && !typed.chars().any(|c| c.is_control() || c.is_whitespace())
}

/// A model id, if the field holds one. `None` for an empty field, which
/// is the state a fresh install is in and not an error.
pub fn model(typed: &str) -> Option<String> {
    let typed = typed.trim();
    (!typed.is_empty() && typeable_model(typed)).then(|| typed.to_owned())
}

/// Whether a temperature field should accept this text as it is typed.
/// `0.` is on the way to `0.9`.
pub fn typeable_temperature(typed: &str) -> bool {
    typed.len() <= 4 && typed.chars().all(|c| c.is_ascii_digit() || c == '.')
}

/// A temperature a user typed, if it is one a server would take.
///
/// `None` rather than a clamp, the same refusal `mcp::port` makes: a
/// setting that silently turns 9 into 2 is a setting that lies about
/// itself, and the caller's answer is to keep the value it had.
pub fn temperature(typed: &str) -> Option<f32> {
    typed
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0 && *value <= HIGHEST_TEMPERATURE)
}

/// Whether a timeout field should accept this text as it is typed.
pub fn typeable_timeout(typed: &str) -> bool {
    typed.len() <= LONGEST_TIMEOUT.to_string().len()
        && typed.chars().all(|character| character.is_ascii_digit())
}

/// A timeout in seconds, if it is one worth waiting.
pub fn timeout(typed: &str) -> Option<u32> {
    typed
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|seconds| (1..=LONGEST_TIMEOUT).contains(seconds))
}

/// The credential store account one endpoint's key is filed under.
///
/// The origin, so that two providers are two credentials and editing
/// the path does not lose one. This is the single place that mapping
/// exists: a second spelling of it somewhere else is how a key gets
/// written under one name and looked up under another, which presents
/// as "the application forgot my key" and is not.
pub fn account_of(base: &BaseUrl) -> String {
    base.origin()
}

/// Whether a key could be sent at all — the transport's own rule
/// (`wipemark_engine::http::sendable`, the function that builds the
/// `Authorization` header), asked before the key is stored. A key the
/// page accepted and the first request then could not carry is what E2-3's
/// live check found; a key refused here never reaches the vault.
pub fn admit_key(secret: &Secret) -> Result<(), KeyFault> {
    wipemark_engine::http::sendable(secret)
}

/// What Save did with a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeySaved {
    /// In the credential store, under the endpoint's account.
    Stored,
    /// Refused before the credential store was touched: no request could
    /// carry it.
    Refused(KeyFault),
    /// The credential store said no, in its own words.
    Failed(String),
}

/// The whole of Save: the rule first, then the vault — so the one road
/// into the credential store refuses a key no request could carry.
///
/// Blocking (the vault write is), so it runs on the background executor;
/// the window also asks [`admit_key`] on the click, which keeps a refused
/// key from costing a trip there at all.
pub fn save_key(vault: &Vault, account: &str, secret: &Secret) -> KeySaved {
    if let Err(fault) = admit_key(secret) {
        return KeySaved::Refused(fault);
    }
    match vault.set(account, secret) {
        Ok(()) => KeySaved::Stored,
        Err(error) => {
            tracing::warn!(%error, "could not store a credential");
            KeySaved::Failed(error.to_string())
        }
    }
}

/// The sentence for a key Save refused. It names what is wrong and never
/// a character of the key.
pub fn key_refusal_message(fault: KeyFault) -> Message {
    match fault {
        KeyFault::Empty => Message::SettingsEngineKeyRefusedEmpty,
        KeyFault::NotAscii => Message::SettingsEngineKeyRefusedNotAscii,
        KeyFault::Control => Message::SettingsEngineKeyRefusedControl,
        KeyFault::Space => Message::SettingsEngineKeyRefusedSpace,
    }
}

/// Everything the engine settings are, as one value.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineSettings {
    pub provider: Provider,
    pub base_url: BaseUrl,
    /// Empty until the user names one. Not an `Option<String>`: the
    /// field it comes from is empty in exactly that case, and one
    /// representation of "nothing chosen" is fewer than two.
    pub model: String,
    /// Default-deny for anything that is not this machine. See
    /// [`Refusal::RemoteNotAllowed`].
    pub allow_remote: bool,
    pub temperature: f32,
    pub reasoning: ReasoningEffort,
    pub timeout: u32,
}

impl Default for EngineSettings {
    /// A fresh install rewrites nothing, talks to nobody, and says so.
    fn default() -> Self {
        Self {
            provider: Provider::default(),
            base_url: BaseUrl::default(),
            model: String::new(),
            allow_remote: false,
            temperature: DEFAULT_TEMPERATURE,
            reasoning: ReasoningEffort::default(),
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

/// Whether a key is stored for the endpoint on screen.
///
/// `Unknown` is a real state and not a placeholder: reading the
/// credential store blocks and, the first time after a rebuild, prompts
/// — so the lookup happens off the GPUI thread and the pane draws at
/// least one frame before it knows. Showing "no key stored" during that
/// frame would be a sentence that is not true yet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum KeyState {
    #[default]
    Unknown,
    Absent,
    Stored,
    /// The credential store said no, in its own words. Kept rather than
    /// collapsed into `Absent`, because "there is no key" and "I could
    /// not find out" have different fixes.
    Failed(String),
}

/// Why this configuration would not send a request.
///
/// Ordered by what a user has to fix first, and returned one at a time:
/// a pane that lists four problems at once is one a reader gives up on,
/// and three of these four stop mattering the moment the first is
/// fixed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// No engine chosen. Not a fault — Layer A is the product without
    /// one.
    NoEngine,
    /// An engine and nowhere to send it.
    NoModel,
    /// The endpoint is not this machine and nobody has said that is
    /// all right. Default-deny, from upstream's `_check_remote`.
    RemoteNotAllowed { host: String },
    /// A key exists for a plaintext endpoint that is not this machine.
    /// The request is refused rather than sent without the key: sending
    /// it unauthenticated would fail confusingly, and sending it
    /// authenticated would put the credential on the wire.
    KeyInTheClear { origin: String },
    /// The provider needs a key and none is stored for this endpoint.
    NoKey { origin: String },
    /// The credential store could not be read. Distinct from
    /// [`NoKey`](Self::NoKey) because it is a different problem with a
    /// different fix — a locked keychain, a denied prompt — and
    /// reporting it as a missing key sends the user off to find one
    /// they already have.
    KeyUnreadable { reason: String },
}

/// What this configuration would do, if E2 were here to do it.
///
/// A free function over values so the rules can be tested without a
/// window — the same shape as `settings::on_screen`. `key` is what the
/// credential store said, which is why it is a parameter and not
/// something this reads: the read blocks, and this runs on the frame.
pub fn refusal(settings: &EngineSettings, key: &KeyState) -> Option<Refusal> {
    if settings.provider == Provider::Off {
        return Some(Refusal::NoEngine);
    }
    if settings.model.trim().is_empty() {
        return Some(Refusal::NoModel);
    }
    if settings.base_url.leaves_this_machine() && !settings.allow_remote {
        return Some(Refusal::RemoteNotAllowed {
            host: settings.base_url.host().to_owned(),
        });
    }
    if !settings.provider.takes_a_key() {
        return None;
    }
    match key {
        // Not yet known is not a refusal. The pane says it is looking.
        KeyState::Unknown => None,
        KeyState::Stored if !settings.base_url.is_private() => Some(Refusal::KeyInTheClear {
            origin: settings.base_url.origin(),
        }),
        KeyState::Stored => None,
        KeyState::Absent => Some(Refusal::NoKey {
            origin: settings.base_url.origin(),
        }),
        KeyState::Failed(reason) => Some(Refusal::KeyUnreadable {
            reason: reason.clone(),
        }),
    }
}

/// Whether storing a key for this endpoint would be storing one that
/// can only travel in the clear.
///
/// Asked *before* the key is written rather than after, because the
/// honest moment to say "this would go over an unencrypted connection"
/// is while the user still has the key in the field.
pub fn key_would_travel_in_the_clear(base: &BaseUrl) -> bool {
    !base.is_private()
}

/// One row of a [`Select`](gpui_component::select::Select), for the two
/// closed lists this pane has.
///
/// The shape `language::LanguageChoice` established: the row carries
/// its own value, so a confirmed selection maps back through the list
/// that produced it and the dropdown cannot ask for something it never
/// offered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice<T: Clone> {
    item: T,
    label: SharedString,
    value: SharedString,
    /// Why the row is shown and cannot be chosen — greyed, the reason under
    /// its label — or `None` for a row that can.
    unavailable: Option<SharedString>,
}

impl<T: Clone> Choice<T> {
    /// The value behind a row. Only the tests reach for it — the two
    /// callers that matter go through [`row_of`] and [`from_value`],
    /// which is the point: a dropdown that could be asked for an item
    /// it never offered is one that can change a setting nobody chose.
    #[cfg(test)]
    pub fn item(&self) -> &T {
        &self.item
    }
}

impl<T: Clone> Choice<T> {
    /// Build one row of a selector.
    ///
    /// `value` is what the control hands back when a row is clicked, so
    /// it has to be unique within the list and it must **not** be the
    /// label: a translated value would change the identity of a row
    /// when the language did, which is how a selection survives a
    /// restart and not a language change.
    pub fn new(item: T, label: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self {
            item,
            label: label.into(),
            value: value.into(),
            unavailable: None,
        }
    }

    /// Shown, greyed, with `why` under its label, and never chosen — not by a
    /// click, which the list refuses, and not through [`from_value`].
    #[must_use]
    pub fn unavailable(mut self, why: Option<String>) -> Self {
        self.unavailable = why.map(SharedString::from);
        self
    }

    /// Why the row cannot be chosen, when it cannot.
    #[cfg(test)]
    pub fn why_unavailable(&self) -> Option<&SharedString> {
        self.unavailable.as_ref()
    }
}

impl<T: Clone> SelectItem for Choice<T> {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }

    fn disabled(&self) -> bool {
        self.unavailable.is_some()
    }

    fn render(&self, _: &mut gpui::Window, _: &mut gpui::App) -> impl gpui::IntoElement {
        use gpui::{div, ParentElement as _, Styled as _};
        let row = div().child(self.label.clone());
        match &self.unavailable {
            Some(why) => row.child(div().text_xs().child(why.clone())),
            None => row,
        }
    }
}

/// The provider selector's rows.
pub fn provider_choices() -> Vec<Choice<Provider>> {
    Provider::ALL
        .into_iter()
        .map(|provider| Choice::new(provider, provider.label(), provider.id()))
        .collect()
}

/// The reasoning-effort selector's rows.
pub fn reasoning_choices() -> Vec<Choice<ReasoningEffort>> {
    ReasoningEffort::ALL
        .into_iter()
        .map(|effort| Choice::new(effort, effort.label(), effort.id()))
        .collect()
}

/// Which row is ticked, or `None` for a stored value this build does
/// not offer — the same honest blank the language selector shows.
pub fn row_of<T: Clone + PartialEq>(choices: &[Choice<T>], item: &T) -> Option<usize> {
    choices.iter().position(|choice| choice.item == *item)
}

/// Resolve a confirmed selection back through the row that produced it.
pub fn from_value<T: Clone>(choices: &[Choice<T>], value: &SharedString) -> Option<T> {
    choices
        .iter()
        .find(|choice| choice.value == *value && choice.unavailable.is_none())
        .map(|choice| choice.item.clone())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use gpui_component::select::SelectItem as _;
    use wipemark_engine::http::KeyFault;
    use wipemark_secret::{Secret, Vault};

    use super::{
        account_of, admit_key, from_value, key_refusal_message, key_would_travel_in_the_clear,
        model, provider_choices, reasoning_choices, refusal, row_of, save_key, temperature,
        timeout, typeable_model, typeable_temperature, typeable_timeout, typeable_url, BaseUrl,
        EngineSettings, KeySaved, KeyState, Preset, Provider, ReasoningEffort, Refusal, Scheme,
        DEFAULT_BASE_URL,
    };

    fn url(value: &str) -> BaseUrl {
        BaseUrl::parse(value).unwrap_or_else(|| panic!("{value} is a base URL"))
    }

    /// Save refuses a key no request could carry, and the credential store
    /// is never touched: E2-3 stored Cyrillic letters typed on the wrong
    /// layout and learned so from the first request.
    #[test]
    fn a_key_that_could_not_be_sent_is_refused_at_save_and_stored_nowhere() {
        let vault = Vault::in_memory("com.GigLabo.wipemark.test");
        let account = account_of(&url("https://api.openai.com"));
        for (typed, fault) in [
            ("sk-ключ", KeyFault::NotAscii),
            ("sk-\u{200b}abc", KeyFault::NotAscii),
            ("sk-a\u{1b}b", KeyFault::Control),
            ("sk-a b", KeyFault::Space),
        ] {
            let secret = Secret::from(typed);
            assert_eq!(admit_key(&secret), Err(fault), "{}", typed.escape_debug());
            assert_eq!(
                save_key(&vault, &account, &secret),
                KeySaved::Refused(fault),
                "{}",
                typed.escape_debug()
            );
            assert!(
                !vault.has(&account).expect("the test vault answers"),
                "a refused key reached the vault: {}",
                typed.escape_debug()
            );
        }

        // The rule is not a wall: a key that can be sent is stored.
        let good = Secret::from("sk-proj-Abc_123.def");
        assert_eq!(admit_key(&good), Ok(()));
        assert_eq!(save_key(&vault, &account, &good), KeySaved::Stored);
        assert_eq!(vault.get(&account).expect("read"), Some(good));
    }

    /// Every refusal has its own sentence, in every shipped language, and
    /// none of them is the credential store's.
    #[test]
    fn every_refused_key_has_a_sentence_in_every_language() {
        let faults = [
            KeyFault::Empty,
            KeyFault::NotAscii,
            KeyFault::Control,
            KeyFault::Space,
        ];
        for language in wipemark_i18n::available_languages() {
            let localizer = wipemark_i18n::Localizer::for_languages(
                std::slice::from_ref(&language.id),
                wipemark_i18n::Rendering::PlainText,
            );
            let mut seen = BTreeSet::new();
            for fault in faults {
                let message = key_refusal_message(fault);
                assert!(localizer.defines(message), "{}: {fault:?}", language.id);
                assert!(
                    seen.insert(localizer.format(message)),
                    "{}: two faults read alike",
                    language.id
                );
            }
        }
    }

    /// A fresh install rewrites nothing and talks to nobody.
    #[test]
    fn a_first_launch_sends_nothing_anywhere() {
        let settings = EngineSettings::default();
        assert_eq!(settings.provider, Provider::Off);
        assert!(!settings.allow_remote);
        assert!(settings.model.is_empty());
        assert!(!settings.base_url.leaves_this_machine());
        assert_eq!(
            refusal(&settings, &KeyState::Unknown),
            Some(Refusal::NoEngine)
        );
    }

    #[test]
    fn a_stored_provider_round_trips() {
        for provider in Provider::ALL {
            assert_eq!(Provider::parse(provider.id()), Some(provider));
            assert_eq!(
                Provider::parse(&provider.id().to_uppercase()),
                Some(provider)
            );
        }
        assert_eq!(Provider::parse("anthropic"), None);
        assert_eq!(Provider::parse(""), None);
    }

    /// The distinction the enum exists for. Collapsing these two into a
    /// boolean strands every server that rejects `"none"` as a value.
    #[test]
    fn off_omits_the_field_and_none_sends_a_value() {
        assert_eq!(ReasoningEffort::Off.wire(), None);
        assert_eq!(ReasoningEffort::None.wire(), Some("none"));
        assert_eq!(ReasoningEffort::High.wire(), Some("high"));
        for effort in ReasoningEffort::ALL {
            assert_eq!(ReasoningEffort::parse(effort.id()), Some(effort));
        }
    }

    /// Ollama's native path carries no `Authorization` header, so
    /// offering the row would invite a credential nothing sends.
    #[test]
    fn only_the_provider_with_a_header_for_it_asks_for_a_key() {
        assert!(!Provider::Off.takes_a_key());
        assert!(!Provider::Ollama.takes_a_key());
        assert!(Provider::OpenAiCompatible.takes_a_key());
        assert_eq!(Provider::Off.path(), None);
        assert_eq!(Provider::Ollama.path(), Some("/api/chat"));
        assert_eq!(
            Provider::OpenAiCompatible.path(),
            Some("/v1/chat/completions")
        );
    }

    #[test]
    fn a_url_is_normalised_so_one_endpoint_has_one_spelling() {
        for (typed, expected) in [
            ("http://127.0.0.1:11434/", "http://127.0.0.1:11434"),
            ("HTTP://LocalHost:11434", "http://localhost:11434"),
            ("https://api.openai.com:443", "https://api.openai.com"),
            ("https://openrouter.ai/api/", "https://openrouter.ai/api"),
            ("  https://example.com  ", "https://example.com"),
            ("http://[::1]:11434", "http://[::1]:11434"),
        ] {
            assert_eq!(url(typed).to_string(), expected, "{typed}");
        }
    }

    /// The refusals that make this a parser rather than a string field.
    /// The second one is the one that matters: a URL with userinfo in
    /// it is a credential in a settings row, which is exactly what the
    /// credential store exists to prevent.
    #[test]
    fn a_base_url_that_is_not_one_is_refused_rather_than_repaired() {
        for wrong in [
            "",
            "api.openai.com",
            "ftp://example.com",
            "file:///etc/passwd",
            "https://",
            "https://example.com?key=sk-x",
            "https://example.com#fragment",
            "https://example.com:not-a-port",
            "http://::1:11434",
            "https://exa mple.com",
        ] {
            assert_eq!(BaseUrl::parse(wrong), None, "{wrong} was accepted");
        }
    }

    /// The one the whole design rests on: a key must not be typeable
    /// into the endpoint field.
    #[test]
    fn a_base_url_never_carries_a_credential() {
        for smuggled in [
            "https://user:sk-secret@api.openai.com",
            "https://sk-secret@api.openai.com",
            "http://token@127.0.0.1:11434",
        ] {
            assert_eq!(
                BaseUrl::parse(smuggled),
                None,
                "{smuggled} put a credential in a settings row"
            );
        }
    }

    #[test]
    fn what_is_on_this_machine_and_what_is_not() {
        for local in [
            "http://127.0.0.1:11434",
            "http://localhost:1234",
            "http://[::1]:11434",
            "https://127.0.0.1",
        ] {
            assert!(!url(local).leaves_this_machine(), "{local}");
            assert!(url(local).is_private(), "{local} is not a wire");
        }
        for remote in [
            "https://api.openai.com",
            "http://192.168.1.101:11434",
            "https://openrouter.ai/api",
        ] {
            assert!(url(remote).leaves_this_machine(), "{remote}");
        }
    }

    /// The plaintext rule, stated over the pair that decides it. Delete
    /// the `is_private` half of `refusal` and this is what goes red.
    #[test]
    fn a_key_never_travels_in_the_clear() {
        assert!(key_would_travel_in_the_clear(&url(
            "http://gateway.example.com:8000"
        )));
        assert!(key_would_travel_in_the_clear(&url("http://192.168.1.101")));
        assert!(!key_would_travel_in_the_clear(&url(
            "https://api.openai.com"
        )));
        // Never leaves the machine, so there is no wire to be in the
        // clear on — and demanding TLS from a local Ollama is a rule
        // everyone would route around.
        assert!(!key_would_travel_in_the_clear(&url(
            "http://127.0.0.1:11434"
        )));

        let settings = EngineSettings {
            provider: Provider::OpenAiCompatible,
            base_url: url("http://gateway.example.com:8000"),
            model: "some-model".to_owned(),
            allow_remote: true,
            ..EngineSettings::default()
        };
        assert_eq!(
            refusal(&settings, &KeyState::Stored),
            Some(Refusal::KeyInTheClear {
                origin: "http://gateway.example.com:8000".to_owned()
            })
        );
    }

    /// Default-deny, and the order the pane reports problems in.
    #[test]
    fn a_remote_endpoint_is_refused_until_it_is_allowed() {
        let mut settings = EngineSettings {
            provider: Provider::OpenAiCompatible,
            base_url: url("https://api.openai.com"),
            model: "gpt-4o-mini".to_owned(),
            ..EngineSettings::default()
        };
        assert_eq!(
            refusal(&settings, &KeyState::Stored),
            Some(Refusal::RemoteNotAllowed {
                host: "api.openai.com".to_owned()
            })
        );

        settings.allow_remote = true;
        assert_eq!(refusal(&settings, &KeyState::Stored), None);
        assert_eq!(
            refusal(&settings, &KeyState::Absent),
            Some(Refusal::NoKey {
                origin: "https://api.openai.com".to_owned()
            })
        );
        // Still being read is not yet a refusal.
        assert_eq!(refusal(&settings, &KeyState::Unknown), None);

        settings.model.clear();
        assert_eq!(
            refusal(&settings, &KeyState::Stored),
            Some(Refusal::NoModel)
        );
    }

    /// A local Ollama with a model is ready, and asks for nothing else.
    #[test]
    fn a_local_engine_needs_no_permission_and_no_key() {
        let settings = EngineSettings {
            provider: Provider::Ollama,
            base_url: url(DEFAULT_BASE_URL),
            model: "llama3.1:8b".to_owned(),
            ..EngineSettings::default()
        };
        assert_eq!(refusal(&settings, &KeyState::Absent), None);
        assert_eq!(
            settings.base_url.endpoint(Provider::Ollama).as_deref(),
            Some("http://127.0.0.1:11434/api/chat")
        );
    }

    /// A path on the base URL survives into the endpoint — OpenRouter
    /// is served under one — while staying out of the account name.
    #[test]
    fn the_path_reaches_the_endpoint_and_not_the_credential() {
        let base = url("https://openrouter.ai/api");
        assert_eq!(
            base.endpoint(Provider::OpenAiCompatible).as_deref(),
            Some("https://openrouter.ai/api/v1/chat/completions")
        );
        assert_eq!(account_of(&base), "https://openrouter.ai");
        assert_eq!(
            account_of(&url("https://openrouter.ai/api/v1")),
            account_of(&base),
            "editing the path lost the key"
        );
    }

    /// Two endpoints are two accounts, and one endpoint spelled two
    /// ways is one account. Both halves matter: the first is a key sent
    /// to a host it was not entered for, the second is a key the
    /// application appears to have forgotten.
    #[test]
    fn a_key_is_filed_under_the_host_it_was_entered_for() {
        assert_ne!(
            account_of(&url("https://api.openai.com")),
            account_of(&url("https://openrouter.ai/api"))
        );
        assert_ne!(
            account_of(&url("https://api.example.com")),
            account_of(&url("http://api.example.com")),
            "an encrypted and an unencrypted endpoint are not one origin"
        );
        for spelling in [
            "https://API.OpenAI.com",
            "https://api.openai.com:443/",
            "https://api.openai.com/v1",
        ] {
            assert_eq!(
                account_of(&url(spelling)),
                account_of(&url("https://api.openai.com")),
                "{spelling} was filed as a different endpoint"
            );
        }
    }

    /// The pair rule every field in this window keeps: anything the
    /// parser would accept has to be reachable one keystroke at a time,
    /// or the field cannot be typed into at all.
    #[test]
    fn every_endpoint_can_be_reached_one_keystroke_at_a_time() {
        for whole in [
            DEFAULT_BASE_URL,
            "https://api.openai.com",
            "https://openrouter.ai/api",
            "http://[::1]:11434",
        ] {
            for cut in 1..=whole.len() {
                assert!(
                    typeable_url(&whole[..cut]),
                    "{} is on the way to {whole} and the field refused it",
                    &whole[..cut]
                );
            }
        }
        for whole in ["0.9", "1.25", "2"] {
            for cut in 1..=whole.len() {
                assert!(typeable_temperature(&whole[..cut]), "{whole}");
            }
        }
        for whole in ["120", "3600"] {
            for cut in 1..=whole.len() {
                assert!(typeable_timeout(&whole[..cut]), "{whole}");
            }
        }
        for whole in ["llama3.1:8b", "deepseek/deepseek-chat"] {
            for cut in 1..=whole.len() {
                assert!(typeable_model(&whole[..cut]), "{whole}");
            }
        }
    }

    /// Out of range is refused, not clamped. A field that turns 9 into
    /// 2 is one that lies about what it is set to.
    #[test]
    fn a_number_out_of_range_is_refused_rather_than_clamped() {
        assert_eq!(temperature("0.9"), Some(0.9));
        assert_eq!(temperature("0"), Some(0.0));
        assert_eq!(temperature("2"), Some(2.0));
        assert_eq!(temperature("2.1"), None);
        assert_eq!(temperature(""), None);
        assert_eq!(temperature("."), None);

        assert_eq!(timeout("120"), Some(120));
        assert_eq!(timeout("0"), None, "a request with no time to answer");
        assert_eq!(timeout("3601"), None);
        assert_eq!(timeout(""), None);
    }

    #[test]
    fn an_empty_model_field_is_no_model_and_not_an_error() {
        assert_eq!(model(""), None);
        assert_eq!(model("   "), None);
        assert_eq!(model("  gpt-4o-mini "), Some("gpt-4o-mini".to_owned()));
    }

    /// A preset that does not parse is a button that writes a value the
    /// field then refuses — invisible until somebody clicks it.
    #[test]
    fn every_preset_is_an_endpoint_the_field_would_accept() {
        for provider in Provider::ALL {
            for preset in provider.presets() {
                let parsed = BaseUrl::parse(preset.url)
                    .unwrap_or_else(|| panic!("{} is not a base URL", preset.url));
                assert_eq!(
                    parsed.to_string(),
                    preset.url,
                    "{} is not normalised",
                    preset.url
                );
                assert!(!preset.label.is_empty());
                if provider == Provider::Ollama {
                    assert!(
                        !parsed.leaves_this_machine(),
                        "the Ollama presets are local endpoints"
                    );
                }
            }
        }
        assert!(
            Provider::Off.presets().is_empty(),
            "nothing to point at when there is no engine"
        );
        assert_eq!(Preset::OPENAI.url, "https://api.openai.com");
        assert_eq!(Scheme::Https.as_str(), "https");
    }

    /// The round-trip both dropdowns depend on. A row whose value does
    /// not come back as its own item silently changes a setting the
    /// user did not touch.
    #[test]
    fn a_click_maps_back_to_the_row_that_was_clicked() {
        let providers = provider_choices();
        for (row, choice) in providers.iter().enumerate() {
            assert_eq!(
                from_value(&providers, choice.value()).as_ref(),
                Some(choice.item())
            );
            assert_eq!(row_of(&providers, choice.item()), Some(row));
        }
        let efforts = reasoning_choices();
        for (row, choice) in efforts.iter().enumerate() {
            assert_eq!(
                from_value(&efforts, choice.value()).as_ref(),
                Some(choice.item())
            );
            assert_eq!(row_of(&efforts, choice.item()), Some(row));
        }
        assert_eq!(from_value(&providers, &"anthropic".into()), None);
    }

    /// Two rows reading the same word is a dropdown with no way to tell
    /// which one is selected.
    #[test]
    fn no_two_rows_of_a_selector_read_the_same() {
        let providers: BTreeSet<String> = Provider::ALL.iter().map(|p| p.label()).collect();
        assert_eq!(providers.len(), Provider::ALL.len(), "{providers:?}");
        let efforts: BTreeSet<String> = ReasoningEffort::ALL.iter().map(|e| e.label()).collect();
        assert_eq!(efforts.len(), ReasoningEffort::ALL.len(), "{efforts:?}");
        let ids: BTreeSet<&str> = Provider::ALL.iter().map(|p| p.id()).collect();
        assert_eq!(ids.len(), Provider::ALL.len());
    }
}
