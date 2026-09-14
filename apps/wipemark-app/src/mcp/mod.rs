//! The MCP server: its preferences, its protocol, and the socket it
//! answers on.
//!
//! Epic **E6 / S6.3**. `heretic-lazy-shot` exposes its capture tools to
//! an agent over the Model Context Protocol, and the pane it does that
//! from — a switch, an address, a port, and a block of JSON to paste
//! into a client — is the shape this module feeds. Wipemark wants the
//! same thing for a different reason: Layer A is deterministic and
//! verifiable, which makes it exactly the kind of step an agent should
//! be able to run on its own output.
//!
//! * [`server`] is the listener, the port scan and the supervisor that
//!   owns them.
//! * [`protocol`] is what it answers — JSON-RPC 2.0 and the MCP
//!   methods, as pure functions over values.
//! * This file is the vocabulary the settings window and the store
//!   share: an address, a port, and the snippet a client is configured
//!   with.
//!
//! # What answers, and what refuses
//!
//! The server is real: it binds, it speaks the protocol, it introduces
//! itself, and it lists its tools. Those tools then **refuse**, by
//! name, and say which epic implements them — Layer A is epic E1 and
//! this build does not have it. That is the same bargain
//! `wipemark-cli` makes when it exits 2 rather than 0, and for the same
//! reason: a tool that answered "nothing found" would be a scrubber
//! reporting a clean document it never read.
//!
//! # Everything a client reads is a format
//!
//! The addresses, the port, the JSON keys, the tool names and the
//! server's own name are read by a machine — an MCP client's
//! configuration file, and then the client itself — so none of them
//! comes from the catalogue, by the same rule that keeps `--json` field
//! names out of it.
//!
//! For the server that rule is sharper than convention. The application
//! initializes `wipemark-i18n` with [`Rendering::Ui`], which is what
//! keeps Fluent's U+2068/U+2069 isolates around interpolated values —
//! correct for a text renderer, and correct for nothing else. Those two
//! code points are `UnicodeClass::BidiControl`: a localized MCP
//! response would hand an agent the exact invisible characters this
//! product exists to remove. The one string here a person reads for its
//! meaning rather than its bytes is [`Client::Generic`]'s tab label,
//! which is drawn in a window and does come from the catalogue.
//!
//! [`Rendering::Ui`]: wipemark_i18n::Rendering::Ui

pub mod protocol;
pub mod server;

use std::fmt;
use std::net::{IpAddr, Ipv4Addr};

use serde_json::json;
use wipemark_i18n::{t, Message};

/// The name the server introduces itself by, and the key an `mcpServers`
/// object files it under. A format: renaming it breaks every
/// configuration file that already names it.
pub const SERVER_NAME: &str = "wipemark";

/// The path the streamable-HTTP transport is served from — the
/// convention every MCP client defaults to, and the one lazy-shot uses.
pub const PATH: &str = "/mcp";

/// The port nothing else on this desktop is expected to want.
///
/// Deliberately *not* 5055, which is heretic-lazy-shot's: the two
/// products are meant to be run together by the same agent, and a
/// default that collides means one of them starts a port along from
/// where its own snippet says it is.
pub const DEFAULT_PORT: u16 = 5056;

/// The lowest port a process can bind without privileges on every
/// platform this ships to. Below it the server would need root, which
/// is not a thing a preferences window should be able to ask for.
pub const LOWEST_PORT: u16 = 1024;

/// The highest port there is. Named rather than inlined because the
/// field's maximum length is derived from it.
pub const HIGHEST_PORT: u16 = u16::MAX;

/// The interface the server accepts connections on.
///
/// Any address this machine holds, not a choice of two. The two that
/// are offered as buttons — [`LOOPBACK`](Self::LOOPBACK) and
/// [`EVERYWHERE`](Self::EVERYWHERE) — are the answers for "an agent
/// running where I am" and "anything that can route to me", which is
/// what almost everyone means. What they are not is exhaustive: a
/// desktop with two interfaces, a machine answering only on its VPN
/// address, a container published on one bridge — each of those is a
/// specific address the user has and the product would otherwise have
/// no way to say. lazy-shot's field is free text with the two presets
/// beside it, and that is the right shape; this is that shape with a
/// type around it.
///
/// The choice still carries the consequence it always did — the
/// difference between loopback and anything else is the difference
/// between one machine and a network — which is why
/// [`on_the_network`](Self::on_the_network) exists and why the pane
/// says so out loud when the answer is yes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindAddress(IpAddr);

impl BindAddress {
    /// This machine and nothing else. The default.
    pub const LOOPBACK: Self = Self(IpAddr::V4(Ipv4Addr::LOCALHOST));

    /// Every interface, including the ones on a network.
    pub const EVERYWHERE: Self = Self(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

    /// The two the window offers as buttons, least exposed first.
    /// Everything else is typed.
    pub const PRESETS: [Self; 2] = [Self::LOOPBACK, Self::EVERYWHERE];

    /// The longest an address can be: an IPv4-mapped IPv6 address
    /// written out in full. The field's maximum length, so that no
    /// address this build would accept can be one keystroke too long to
    /// enter.
    pub const LONGEST: usize = "0000:0000:0000:0000:0000:ffff:255.255.255.255".len();

    /// Read a stored or typed value back.
    ///
    /// `None` for anything that is not an address — including a
    /// hostname. A hostname would have to be resolved before it could
    /// be bound, and the answer could change between the resolution and
    /// the bind; an address is a fact about this machine's interfaces.
    /// Surrounding space is tolerated because a paste carries it.
    pub fn parse(value: &str) -> Option<Self> {
        value.trim().parse::<IpAddr>().ok().map(Self)
    }

    /// The address, to bind.
    pub fn ip(self) -> IpAddr {
        self.0
    }

    /// Whether this is a wildcard — `0.0.0.0` or `::`, the addresses
    /// that mean "every interface" rather than naming one.
    pub fn is_wildcard(self) -> bool {
        self.0.is_unspecified()
    }

    /// Whether binding here puts the server on a network.
    ///
    /// Everything that is not loopback does: a wildcard by covering
    /// every interface, and a specific address by being the one the
    /// rest of the network reaches this machine at. The pane says so
    /// when the answer is yes — this server asks for no credentials,
    /// and that is a thing to know before the address is on a subnet
    /// with other people on it.
    pub fn on_the_network(self) -> bool {
        !self.0.is_loopback()
    }

    /// The host a *client* should dial to reach it.
    ///
    /// Not the same string as the address, and the difference is the
    /// one that makes a pasted snippet work. `0.0.0.0` is a wildcard to
    /// bind, not an address to connect to: a client handed
    /// `http://0.0.0.0:5056` reaches nothing on some stacks and the
    /// right thing by accident on others. An IPv6 address is bracketed,
    /// because a URL has no other way to tell the colons in an address
    /// from the colon before a port.
    pub fn host(self) -> String {
        match self.0 {
            _ if self.is_wildcard() => "localhost".to_owned(),
            IpAddr::V4(address) => address.to_string(),
            IpAddr::V6(address) => format!("[{address}]"),
        }
    }
}

impl Default for BindAddress {
    fn default() -> Self {
        Self::LOOPBACK
    }
}

impl fmt::Display for BindAddress {
    /// The address as it is bound, as it is stored and as the field
    /// shows it. A format.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Whether an address field should accept this text *while it is being
/// typed*.
///
/// The same bargain [`typeable`] makes for the port, and it has to be
/// made here too: `192.168.1.` is on the way to `192.168.1.101` and a
/// field that refused it could never be typed into. So this judges the
/// characters and the length, [`BindAddress::parse`] judges the address
/// once there is a whole one, and
/// `every_address_can_be_reached_one_keystroke_at_a_time` is what keeps
/// the two from drifting apart.
///
/// Hex digits and colons rather than digits and dots, because `::1` and
/// `fe80::1` are addresses a machine really has.
pub fn typeable_address(typed: &str) -> bool {
    typed.len() <= BindAddress::LONGEST
        && typed
            .chars()
            .all(|character| character.is_ascii_hexdigit() || character == '.' || character == ':')
}

/// Where a client connects: an address and a port, together, because
/// neither is a URL on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Endpoint {
    pub bind: BindAddress,
    pub port: u16,
}

impl Default for Endpoint {
    fn default() -> Self {
        Self {
            bind: BindAddress::default(),
            port: DEFAULT_PORT,
        }
    }
}

impl Endpoint {
    /// The URL to paste into a client.
    pub fn url(self) -> String {
        format!("http://{}:{}{PATH}", self.bind.host(), self.port)
    }
}

/// Whether a port field should accept this text *while it is being
/// typed*.
///
/// Deliberately looser than [`port`], and the pair has to be looser in
/// exactly the right way: "50" is on the way to "5056" and a field that
/// refused it could never be typed into, while "5056x" is not on the
/// way to anything. So this judges the characters and the length, and
/// [`port`] judges the number once there is one — and
/// `every_port_can_be_reached_one_keystroke_at_a_time` is what keeps
/// the two from drifting into a field that cannot reach a port it would
/// then accept.
pub fn typeable(typed: &str) -> bool {
    typed.len() <= HIGHEST_PORT.to_string().len()
        && typed.chars().all(|character| character.is_ascii_digit())
}

/// A port a user typed, if it is one this build could bind.
///
/// `None` for anything out of range rather than a clamp: silently
/// turning 80 into 1024 is a setting that lies about itself, and the
/// caller's answer to `None` is to keep the port it already had.
pub fn port(typed: &str) -> Option<u16> {
    typed
        .parse::<u16>()
        .ok()
        .filter(|port| *port >= LOWEST_PORT)
}

/// The clients the pane writes a snippet for.
///
/// The list is lazy-shot's, and for the same reason: these are the
/// three configuration shapes in circulation plus the one that says
/// what the other three are variations on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Client {
    ClaudeCode,
    ClaudeDesktop,
    N8n,
    Generic,
}

impl Client {
    /// Every client, in the order the tabs show them.
    pub const ALL: [Client; 4] = [
        Self::ClaudeCode,
        Self::ClaudeDesktop,
        Self::N8n,
        Self::Generic,
    ];

    /// The tab's label.
    ///
    /// Three product names and one word. The names are not translated —
    /// the same rule `wipemark_core::Vendor` states, and a user looking
    /// for the tab that matches the application they have installed
    /// needs to read the name that application is called. "Any MCP
    /// client" is a description rather than a name, so it comes from
    /// the catalogue like everything else a person reads for its
    /// meaning.
    pub fn label(self) -> String {
        match self {
            Self::ClaudeCode => "Claude Code".to_owned(),
            Self::ClaudeDesktop => "Claude Desktop".to_owned(),
            Self::N8n => "n8n".to_owned(),
            Self::Generic => t(Message::SettingsMcpClientGeneric),
        }
    }

    /// A stable element id for the tab. Not a label — this one is never
    /// shown, and it must not move when the language does.
    pub fn id(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::ClaudeDesktop => "claude-desktop",
            Self::N8n => "n8n",
            Self::Generic => "generic",
        }
    }

    /// The configuration to paste, for this client and this endpoint.
    ///
    /// Built through `serde_json` rather than `format!` so that a
    /// snippet is valid JSON by construction. A user pasting a
    /// half-escaped string into their configuration file finds out from
    /// their client, hours later, in a message about a file they did not
    /// hand-write.
    pub fn snippet(self, endpoint: Endpoint) -> String {
        let url = endpoint.url();
        let value = match self {
            // The `type` discriminator Claude Code wants for a server it
            // dials rather than spawns.
            Self::ClaudeCode => json!({
                "mcpServers": { SERVER_NAME: { "type": "http", "url": url } }
            }),
            Self::ClaudeDesktop => json!({
                "mcpServers": { SERVER_NAME: { "url": url } }
            }),
            Self::N8n => json!({ "serverUrl": url, "transport": "streamable-http" }),
            Self::Generic => json!({
                "url": url,
                "transport": "streamable-http",
                "name": SERVER_NAME,
            }),
        };
        // Pretty rather than compact: this is read by a person before it
        // is read by a machine, and it lands in a file they will have to
        // merge it into by hand.
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| url.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// The bug this catches has cost more afternoons than any other in
    /// this file: `0.0.0.0` is a wildcard to bind and not an address to
    /// dial, and a snippet carrying it fails on some network stacks and
    /// works by accident on others — which is worse, because it ships.
    #[test]
    fn a_snippet_never_tells_a_client_to_dial_the_wildcard() {
        for wildcard in ["0.0.0.0", "::"] {
            let endpoint = Endpoint {
                bind: BindAddress::parse(wildcard).expect("a wildcard is an address"),
                port: DEFAULT_PORT,
            };
            assert_eq!(endpoint.url(), "http://localhost:5056/mcp");
            for client in Client::ALL {
                assert!(
                    !client.snippet(endpoint).contains(wildcard),
                    "{client:?} tells the client to connect to {wildcard}"
                );
            }
        }
    }

    /// Every snippet names the endpoint the pane is showing. One that
    /// does not is a tab that quietly hands out a stale port.
    #[test]
    fn every_snippet_carries_the_endpoint_on_screen() {
        let endpoint = Endpoint {
            bind: BindAddress::LOOPBACK,
            port: 7777,
        };
        let url = endpoint.url();
        assert_eq!(url, "http://127.0.0.1:7777/mcp");
        for client in Client::ALL {
            assert!(
                client.snippet(endpoint).contains(&url),
                "{client:?}'s snippet does not contain {url}"
            );
        }
    }

    /// A snippet is pasted into a configuration file. Valid JSON is not
    /// a nice-to-have.
    #[test]
    fn every_snippet_is_valid_json() {
        for client in Client::ALL {
            let snippet = client.snippet(Endpoint::default());
            serde_json::from_str::<serde_json::Value>(&snippet)
                .unwrap_or_else(|error| panic!("{client:?}'s snippet is not JSON: {error}"));
        }
    }

    /// Four tabs that produce one snippet are one tab and three lies.
    #[test]
    fn no_two_clients_hand_out_the_same_snippet() {
        let snippets: BTreeSet<String> = Client::ALL
            .iter()
            .map(|client| client.snippet(Endpoint::default()))
            .collect();
        assert_eq!(snippets.len(), Client::ALL.len());
    }

    /// The ids are element ids and configuration-file keys. They must
    /// not move when the language does, and no two may collide.
    #[test]
    fn the_client_ids_are_distinct_and_not_labels() {
        let ids: BTreeSet<&str> = Client::ALL.iter().map(|client| client.id()).collect();
        assert_eq!(ids.len(), Client::ALL.len());
    }

    /// The whole point of widening the field: an address this machine
    /// actually holds is a bind address, and it survives being written
    /// down and read back.
    #[test]
    fn an_address_of_this_machine_is_one_of_the_answers() {
        for typed in ["192.168.1.101", "10.0.0.7", "::1", "fe80::1", "0.0.0.0"] {
            let parsed = BindAddress::parse(typed).unwrap_or_else(|| panic!("{typed} is a bind"));
            assert_eq!(
                BindAddress::parse(&parsed.to_string()),
                Some(parsed),
                "{typed} does not survive the round trip through the store"
            );
        }
        assert_eq!(
            BindAddress::parse("  127.0.0.1  "),
            Some(BindAddress::LOOPBACK)
        );
    }

    /// And a word is still not an address. A hostname would have to be
    /// resolved before it could be bound, and what it resolves to can
    /// change between the two — so it is refused here rather than
    /// guessed at, and the caller keeps the address it had.
    #[test]
    fn a_word_is_not_an_address_however_much_it_looks_like_one() {
        for typed in ["localhost", "", "wipemark.local", "192.168.1", "999.1.1.1"] {
            assert_eq!(
                BindAddress::parse(typed),
                None,
                "{typed} was read as an address"
            );
        }
    }

    /// A URL has no way to tell the colons inside an IPv6 address from
    /// the colon before the port. Without the brackets a client dials
    /// `http://::1:5056` and reaches nothing.
    #[test]
    fn a_v6_address_is_bracketed_before_it_reaches_a_url() {
        let endpoint = Endpoint {
            bind: BindAddress::parse("::1").expect("an address"),
            port: DEFAULT_PORT,
        };
        assert_eq!(endpoint.url(), "http://[::1]:5056/mcp");
        for client in Client::ALL {
            assert!(client.snippet(endpoint).contains("[::1]:5056"));
        }
    }

    /// Loopback is the one address that keeps this machine to itself.
    /// Everything else — a wildcard or a specific interface — is a
    /// server other people can reach, and the pane warns on exactly
    /// this answer.
    #[test]
    fn every_address_but_loopback_puts_the_server_on_a_network() {
        for kept_at_home in ["127.0.0.1", "127.0.0.53", "::1"] {
            let bind = BindAddress::parse(kept_at_home).expect("an address");
            assert!(!bind.on_the_network(), "{kept_at_home} warned for nothing");
        }
        for reachable in ["0.0.0.0", "192.168.1.101", "10.0.0.7", "::"] {
            let bind = BindAddress::parse(reachable).expect("an address");
            assert!(
                bind.on_the_network(),
                "{reachable} is on a network unwarned"
            );
        }
    }

    /// The bug this catches empties a field nobody can refill: a
    /// maximum length one character short, and an address the settings
    /// accept is one the keyboard cannot reach.
    #[test]
    fn every_address_can_be_reached_one_keystroke_at_a_time() {
        for target in [
            "127.0.0.1",
            "0.0.0.0",
            "192.168.1.101",
            "255.255.255.255",
            "fe80::1",
            "0000:0000:0000:0000:0000:ffff:255.255.255.255",
        ] {
            for length in 1..=target.len() {
                let prefix = &target[..length];
                assert!(
                    typeable_address(prefix),
                    "typing {target} stalls at {prefix:?}: the field refuses the next keystroke"
                );
            }
            assert!(
                BindAddress::parse(target).is_some(),
                "{target} is not an address"
            );
        }
    }

    /// And the field is not a free-text box: everything it lets through
    /// is on the way to an address, and a hostname is not.
    #[test]
    fn the_address_field_accepts_nothing_that_is_on_the_way_to_no_address() {
        assert!(
            typeable_address(""),
            "an empty field is how an address is replaced"
        );
        assert!(!typeable_address("localhost"));
        assert!(!typeable_address("192.168.1.101 "));
        assert!(!typeable_address("192.168.1.101/24"));
        assert!(!typeable_address(&"0".repeat(BindAddress::LONGEST + 1)));
    }

    /// A port below 1024 needs root to bind. Refused rather than
    /// clamped: a field that silently turns 80 into 1024 is a setting
    /// that lies about itself.
    #[test]
    fn a_port_that_could_not_be_bound_is_refused_rather_than_corrected() {
        assert_eq!(port("5056"), Some(5056));
        assert_eq!(port("1024"), Some(LOWEST_PORT));
        assert_eq!(port("80"), None);
        assert_eq!(port("0"), None);
        assert_eq!(port("65536"), None);
        assert_eq!(port(""), None);
        assert_eq!(port("5056 "), None);
    }

    /// The bug this catches empties a field nobody can refill: a
    /// maximum length one digit short, and 65535 is a port the settings
    /// accept and the keyboard cannot reach.
    #[test]
    fn every_port_can_be_reached_one_keystroke_at_a_time() {
        for target in [LOWEST_PORT, DEFAULT_PORT, 7777, HIGHEST_PORT] {
            let digits = target.to_string();
            for length in 1..=digits.len() {
                let prefix = &digits[..length];
                assert!(
                    typeable(prefix),
                    "typing {target} stalls at {prefix:?}: the field refuses the next keystroke"
                );
            }
            assert_eq!(port(&digits), Some(target));
        }
    }

    /// And the field is not a free-text box. Everything it lets through
    /// is on the way to a number; nothing else is.
    #[test]
    fn the_field_accepts_nothing_that_is_on_the_way_to_no_port() {
        assert!(typeable(""), "an empty field is how a port is replaced");
        assert!(!typeable("5056x"));
        assert!(!typeable("-1"));
        assert!(!typeable(" 5056"));
        assert!(!typeable("655350"), "one digit too many for any port");
    }

    /// The default has to be bindable by the code that will read it.
    #[test]
    fn the_default_port_is_one_this_build_would_accept() {
        assert_eq!(port(&DEFAULT_PORT.to_string()), Some(DEFAULT_PORT));
    }

    /// Two Heretic tools on one desktop, and neither of them asked the
    /// user to pick a port. They must not want the same one — with the
    /// port scan below, a collision is no longer a server that fails to
    /// start, it is a server that starts a port along from where its
    /// own snippet was copied.
    #[test]
    fn the_default_port_is_not_the_one_lazy_shot_takes() {
        /// heretic-lazy-shot's `SETTINGS_KEYS.MCP_PORT` default.
        const LAZY_SHOT: u16 = 5055;
        assert_ne!(DEFAULT_PORT, LAZY_SHOT);
    }
}
