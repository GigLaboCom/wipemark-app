//! The Settings window — the one place a preference is changed.
//!
//! Epic **E6 / S6.3**. The theme and the language used to live in the
//! status bar, one control each, because there were two of them. That
//! stops scaling at the third: engine, model directory, endpoint and
//! key are all coming in E6–E9, and none of them belongs in a strip
//! that is meant to say what the application is doing right now.
//!
//! # A window, not a drawing of one
//!
//! This is a real second window — `cx.open_window`, its own titlebar,
//! its own close button, its own entry in the window list — and not a
//! rectangle painted over the main one. The difference is everything
//! the platform does for free once it knows a window exists: the
//! traffic lights and their behaviours, moving it to another desktop or
//! another screen, remembering it in Mission Control, the shortcuts a
//! user already knows. A painted dialog has to reimplement each of
//! those badly or do without them.
//!
//! It also settles the modality question by removing it. There is
//! nothing to dismiss by clicking outside, because outside is another
//! window; there is no overlay to suppress. Every change is applied and
//! written the moment it is made, so closing the window — by its close
//! button, by ⌘W, by Escape, by Quit — leaves the same state behind
//! either way.
//!
//! # Sections down the left
//!
//! [`Section`] is the sidebar and [`Setting`] is a row inside one. Two
//! preferences did not need that structure; two preferences plus a
//! server's address do, and every desktop's preferences window has
//! settled on the same answer — a list of sections on the left, the
//! chosen one filling the rest. `General` is the name the platforms use
//! for the section that holds what does not belong to a named feature,
//! and using their word rather than a Wipemark coinage is the whole
//! point of following the convention.
//!
//! The sidebar is a `gpui_component::Sidebar` rather than a column of
//! buttons: it is already the component with the right hover, active
//! and keyboard behaviour, and it paints from the theme's own `sidebar`
//! colours, which is what makes this look like a preferences window
//! rather than a page with a nav bar drawn on it.
//!
//! # Where it opens
//!
//! Remembered geometry first, per screen ([`window_state`]). Failing
//! that, centred — over the main window when the request came from the
//! main window, on the screen under the pointer when it came from the
//! menu bar, because the menu bar is drawn on every display and a click
//! on it says nothing about which one the user is looking at. See
//! [`Origin`], and [`screen`] for the arithmetic.
//!
//! # Where the state lives
//!
//! [`Preferences`] is app-lifetime and holds no widgets. It has to be:
//! the observer that makes `System` a live promise must keep running
//! while this window is closed, the menu bar changes the theme whether
//! or not the window exists, and the MCP server — when there is one —
//! will read its address with no window in the call path at all. The
//! widgets live in [`SettingsView`], which is built with the window and
//! dies with it: a `SelectState` or an `InputState` is bound to the
//! window it was created in and cannot outlive it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    actions, deferred, div, point, px, size, AnyElement, AnyWindowHandle, App, Bounds,
    ClipboardItem, Context, Entity, FocusHandle, Focusable, Global, Hsla, KeyBinding,
    PathPromptOptions, Pixels, SharedString, Size, Subscription, TitlebarOptions, Window,
    WindowBounds, WindowOptions,
};
use gpui_component::button::{Button, ButtonGroup, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::radio::{Radio, RadioGroup};
use gpui_component::select::{Select, SelectEvent, SelectState};
use gpui_component::sidebar::{Sidebar, SidebarMenu, SidebarMenuItem};
use gpui_component::switch::Switch;
use gpui_component::tab::{Tab, TabBar};
use gpui_component::tag::Tag;
use gpui_component::{
    h_flex, v_flex, ActiveTheme, Disableable as _, IndexPath, Root, Selectable, Sizable,
    StyledExt as _,
};
use wipemark_engine::Unavailable;
use wipemark_i18n::{args, t, t_args, LanguagePreference, Message};
use wipemark_models::host::Host;
use wipemark_models::manifest::{Manifest, ModelEntry, Role};
use wipemark_models::store::{Cancel, Downloads, Event, Progress, State};
use wipemark_secret::{Secret, Vault};

use crate::compare::Comparison;
use crate::config::{self, SettingsStore};
use crate::dialog::{Answer, Chosen, Confirm, Naming};
use crate::diff::Grain;
use crate::duty::{self, Duty, LocalOptions, OnDisk, Performer, Roster, Serves, Vacancy};
use crate::engine::{
    self, BaseUrl, Choice, EngineSettings, KeyState, Provider, ReasoningEffort, Refusal,
};
use crate::engine_host::{
    self, Check, CheckOutcome, EngineHandle, EngineHost, Keeping, Loaded, LocalPolicy, IDLE_MINUTES,
};
use crate::hotkey::{self, Hotkey, Registration};
use crate::icon::{Icon, IconName};
use crate::language::{self, LanguageChoice};
use crate::mcp::server::{self, Supervisor};
use crate::mcp::{self, BindAddress, Client, Endpoint};
use crate::models::Folder;
// `Answer` is deliberately not imported: `dialog::Answer` is a
// dialog's yes or no, and two words that short would read as one.
use crate::placement::{self, Onto, Origin, Spot, Zone};
use crate::profile::{self, Profile, Standing};
use crate::recorder::{Recorder, RecorderEvent};
use crate::retention::{self, Destination, Homes, Period, Retention};
use crate::screen::{self, Connected, Screen};
use crate::theme::ThemePreference;
use crate::tray::Tray;
use crate::{display_watch, models, panel, window_state};

actions!(wipemark, [OpenSettings, CloseSettings]);

/// Key context of the Settings window, so that `⌘W` closes *it* and
/// means nothing anywhere else.
const CONTEXT: &str = "Settings";

/// The size the window gets the first time it is opened on a screen.
/// After that the screen's own remembered rectangle wins.
const DEFAULT_SIZE: Size<Pixels> = Size {
    width: px(820.0),
    height: px(560.0),
};

/// Small enough to be a nuisance, not small enough to hide a row: the
/// sidebar keeps its width, so everything a squeeze takes comes out of
/// the section beside it. The platform enforces this; nothing here has
/// to.
const MIN_SIZE: Size<Pixels> = Size {
    width: px(660.0),
    height: px(440.0),
};

/// Width of the section list.
///
/// Narrower than the component's own 255 px default, which is sized for
/// an application's main sidebar rather than a preferences window's.
const SIDEBAR_WIDTH: Pixels = px(180.0);

/// How long a change to the address or the port waits before the
/// server is restarted on it.
///
/// Typing a port is several valid ports on the way to the one that was
/// meant — 1024 is a port, and so is 10240 one keystroke later — and
/// each of them would otherwise be a server bound and thrown away. Long
/// enough to type through, short enough that nobody wonders whether it
/// took.
const SETTLE: Duration = Duration::from_millis(500);

/// How far above everything else a dialog is painted.
///
/// gpui-component defers its own overlays at 1 (popovers, selects,
/// context menus) and 2 (tooltips, date pickers). A modal dialog has to
/// be over all of them, and over the `Sidebar`, which defers too.
const DIALOG_PRIORITY: usize = 10;

/// How tall one display's grid of zones is drawn.
///
/// The width follows the display's own shape, so the card for a
/// portrait monitor is portrait — a grid that ignored the aspect ratio
/// would be six rectangles that are not the six rectangles the
/// preference means.
const GRID_HEIGHT: Pixels = px(96.0);

/// Width of the right-hand column every control sits in.
///
/// Wide enough for the longest language autonym this build ships plus
/// the caret, and the same for every row so the controls line up down
/// the section.
const CONTROL_COLUMN: Pixels = px(240.0);

/// One entry in the sidebar, and the page it shows.
///
/// The order is the order they are listed. `General` first because it
/// is where a user who does not yet know what the application can do
/// will look, and because it is where the platform puts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    General,
    Placement,
    Compare,
    Engine,
    Models,
    Retention,
    Mcp,
}

impl Section {
    /// Every section, in the order the sidebar lists them.
    ///
    /// Engine before MCP because it is the one a user came here for:
    /// it is where the product's second half is turned on, and the MCP
    /// page is an integration with something else. Models sits between
    /// them, next to the page it belongs to: Engine points Layer B at a
    /// server, and Models fills the machine that server runs on.
    ///
    /// Placement is second because it is General's other half: both
    /// pages are about the windows rather than about the work, and a
    /// user looking for "where does this thing open" reads down the top
    /// of the list rather than past the model catalogue.
    ///
    /// Compare is third, beside Placement, for the same reason: it is
    /// about a window — how the Compare window shows a result beside
    /// its original — and not about the work that made the result.
    ///
    /// Retention follows Models because it is the last page about the
    /// work: Engine says who rewrites, Models what is on the disk, and
    /// Retention what is written once they have — and what the product
    /// keeps of its own. After it only the integration is left.
    pub const ALL: [Section; 7] = [
        Self::General,
        Self::Placement,
        Self::Compare,
        Self::Engine,
        Self::Models,
        Self::Retention,
        Self::Mcp,
    ];

    /// The sidebar's label.
    pub fn title(self) -> Message {
        match self {
            Self::General => Message::SettingsSectionGeneral,
            Self::Placement => Message::SettingsSectionPlacement,
            Self::Compare => Message::SettingsSectionCompare,
            Self::Engine => Message::SettingsSectionEngine,
            Self::Models => Message::SettingsSectionModels,
            Self::Retention => Message::SettingsSectionRetention,
            Self::Mcp => Message::SettingsSectionMcp,
        }
    }

    /// The glyph beside the label in the sidebar.
    ///
    /// A section list of two entries does not need pictures to be
    /// navigable; it needs them to be *scannable*, which is what a
    /// window that grows a fourth and a fifth section will need
    /// badly. Sliders for the preferences that belong to no feature,
    /// a plug for the one that connects this application to another.
    pub fn glyph(self) -> IconName {
        match self {
            Self::General => IconName::Sliders,
            // A grid of cells, which is the control this page is: a
            // screen divided into sixths, one of them ticked.
            // Deliberately not the plain `Display` the System theme
            // button wears — two rows of this window carrying one
            // glyph is how a sidebar stops being scannable — and not
            // FA's `display-arrow-down`, a window arriving on a
            // screen, which is Pro-only artwork this repository does
            // not licence.
            Self::Placement => IconName::TableCellsLarge,
            // The glyph the Compare window's own footer wears, so the
            // page and the window are found by the same picture.
            Self::Compare => IconName::CodeCompare,
            Self::Engine => IconName::Microchip,
            // A disk, because that is what this page is about: weights
            // that take gigabytes of it and stay there until they are
            // removed.
            Self::Models => IconName::HardDrive,
            // An archive box: what is kept, and for how long. Not the
            // `HardDrive` above, which is the disk the weights take
            // — this page is about copies, and a copy is a thing you
            // put in a box and later throw out.
            Self::Retention => IconName::BoxArchive,
            Self::Mcp => IconName::Plug,
        }
    }

    /// A stable element id, and the name the command line knows this
    /// section by. Never shown, and deliberately not the label: an id
    /// that moves when the language does is an id that resets the
    /// component's state on a language change — and a `--settings=`
    /// value that only works in English.
    pub fn id(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Placement => "placement",
            Self::Compare => "compare",
            Self::Engine => "engine",
            Self::Models => "models",
            Self::Retention => "retention",
            Self::Mcp => "mcp",
        }
    }

    /// The section a command line named, or `None` for a word that is
    /// not one. Case and surrounding space are tolerated because a
    /// shell hands both through; nothing else is guessed.
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim().to_ascii_lowercase();
        Self::ALL.into_iter().find(|section| section.id() == value)
    }

    /// Every section's name, for a diagnostic that has to list them.
    pub fn names() -> String {
        Self::ALL
            .iter()
            .map(|section| section.id())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The rows this section shows, in [`Setting::ALL`] order.
    pub fn rows(self) -> impl Iterator<Item = Setting> {
        Setting::ALL
            .into_iter()
            .filter(move |setting| setting.section() == self)
    }
}

/// One row of one section.
///
/// The variants are the rows, in the order they are shown, and
/// [`SettingsView::control`] matches on them exhaustively — a new
/// preference is a compile error until it has been given a widget,
/// rather than a row that renders as a title with nothing beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    Appearance,
    Language,
    ShortcutShow,
    ShortcutPanel,
    Setup,
    WindowScreen,
    CloseAfterDrop,
    CompareGrain,
    CompareFollow,
    EngineServes,
    EngineKeep,
    EngineIdle,
    EngineLock,
    EngineProfile,
    EngineProvider,
    EngineEndpoint,
    EngineModel,
    EngineKey,
    EngineAllowRemote,
    EngineTemperature,
    EngineReasoning,
    EngineTimeout,
    ModelsFolder,
    ModelForRewrite,
    ResultsDestination,
    ResultsFolder,
    KeepOriginals,
    KeepResults,
    KeepFor,
    ServeOverMcp,
    McpBind,
    McpPort,
}

impl Setting {
    /// Every row, in the order the window shows them.
    ///
    /// The Engine rows are in the order a configuration is built:
    /// which shape of request, where it goes, which model answers it,
    /// what proves who is asking — and only then the knobs, which have
    /// working defaults and which nobody has to touch to get a first
    /// rewrite out of the product.
    ///
    /// The profile row comes before all of them because it is the one
    /// that sets every other: a user who has been here before picks a
    /// name and is done, and a user who has not scrolls past a control
    /// that says so.
    ///
    /// And "who rewrites" comes before *that*, because it decides which
    /// of the two halves of the product the rest of the page is even
    /// describing. It is deliberately not part of a profile: a profile
    /// names an endpoint, and whether an endpoint is asked at all is
    /// not the endpoint's business.
    ///
    /// Setup is the last General row: it is the one control on that
    /// page that opens something rather than setting it, and a reader
    /// who came here to change a theme should not meet it first.
    ///
    /// The Retention rows are in the order the questions are asked:
    /// where a result goes and which folder that means, then whether
    /// the product keeps anything of its own — the original, the
    /// result — and only then for how long, which means nothing until
    /// one of the two switches is on.
    ///
    /// The Compare rows put what is *marked* before how the two sides
    /// *move*: a reader opens the window for the marks.
    pub const ALL: [Setting; 32] = [
        Self::Appearance,
        Self::Language,
        Self::ShortcutShow,
        Self::ShortcutPanel,
        Self::Setup,
        Self::WindowScreen,
        Self::CloseAfterDrop,
        Self::CompareGrain,
        Self::CompareFollow,
        Self::EngineServes,
        Self::EngineKeep,
        Self::EngineIdle,
        Self::EngineLock,
        Self::EngineProfile,
        Self::EngineProvider,
        Self::EngineEndpoint,
        Self::EngineModel,
        Self::EngineKey,
        Self::EngineAllowRemote,
        Self::EngineTemperature,
        Self::EngineReasoning,
        Self::EngineTimeout,
        Self::ModelsFolder,
        Self::ModelForRewrite,
        Self::ResultsDestination,
        Self::ResultsFolder,
        Self::KeepOriginals,
        Self::KeepResults,
        Self::KeepFor,
        Self::ServeOverMcp,
        Self::McpBind,
        Self::McpPort,
    ];

    /// Which page this row appears on.
    pub fn section(self) -> Section {
        match self {
            Self::Appearance
            | Self::Language
            | Self::ShortcutShow
            | Self::ShortcutPanel
            | Self::Setup => Section::General,
            Self::WindowScreen | Self::CloseAfterDrop => Section::Placement,
            Self::CompareGrain | Self::CompareFollow => Section::Compare,
            Self::EngineServes
            | Self::EngineKeep
            | Self::EngineIdle
            | Self::EngineLock
            | Self::EngineProfile
            | Self::EngineProvider
            | Self::EngineEndpoint
            | Self::EngineModel
            | Self::EngineKey
            | Self::EngineAllowRemote
            | Self::EngineTemperature
            | Self::EngineReasoning
            | Self::EngineTimeout => Section::Engine,
            Self::ModelsFolder | Self::ModelForRewrite => Section::Models,
            Self::ResultsDestination
            | Self::ResultsFolder
            | Self::KeepOriginals
            | Self::KeepResults
            | Self::KeepFor => Section::Retention,
            Self::ServeOverMcp | Self::McpBind | Self::McpPort => Section::Mcp,
        }
    }

    /// The row's heading.
    pub fn title(self) -> Message {
        match self {
            Self::Appearance => Message::SettingsAppearanceTitle,
            Self::Language => Message::SettingsLanguageTitle,
            Self::ShortcutShow => hotkey::Action::Show.title(),
            Self::ShortcutPanel => hotkey::Action::Panel.title(),
            Self::Setup => Message::SettingsSetupTitle,
            Self::WindowScreen => Message::SettingsPlacementScreenTitle,
            Self::CloseAfterDrop => Message::SettingsPlacementCloseTitle,
            Self::CompareGrain => Message::SettingsCompareGrainTitle,
            Self::CompareFollow => Message::SettingsCompareFollowTitle,
            Self::EngineServes => Message::SettingsEngineServesTitle,
            Self::EngineKeep => Message::SettingsEngineKeepTitle,
            Self::EngineIdle => Message::SettingsEngineIdleTitle,
            Self::EngineLock => Message::SettingsEngineLockTitle,
            Self::EngineProfile => Message::SettingsEngineProfileTitle,
            Self::EngineProvider => Message::SettingsEngineProviderTitle,
            Self::EngineEndpoint => Message::SettingsEngineEndpointTitle,
            Self::EngineModel => Message::SettingsEngineModelTitle,
            Self::EngineKey => Message::SettingsEngineKeyTitle,
            Self::EngineAllowRemote => Message::SettingsEngineAllowRemoteTitle,
            Self::EngineTemperature => Message::SettingsEngineTemperatureTitle,
            Self::EngineReasoning => Message::SettingsEngineReasoningTitle,
            Self::EngineTimeout => Message::SettingsEngineTimeoutTitle,
            Self::ModelsFolder => Message::SettingsModelsFolderTitle,
            Self::ModelForRewrite => Message::SettingsModelsRewriteTitle,
            Self::ResultsDestination => Message::SettingsRetentionDestinationTitle,
            Self::ResultsFolder => Message::SettingsRetentionFolderTitle,
            Self::KeepOriginals => Message::SettingsRetentionOriginalsTitle,
            Self::KeepResults => Message::SettingsRetentionResultsTitle,
            Self::KeepFor => Message::SettingsRetentionPeriodTitle,
            Self::ServeOverMcp => Message::SettingsMcpEnabledTitle,
            Self::McpBind => Message::SettingsMcpBindTitle,
            Self::McpPort => Message::SettingsMcpPortTitle,
        }
    }

    /// The sentence under the heading.
    ///
    /// Not decoration. Appearance and Language both have a `System`
    /// choice and the two `System`s do not mean the same thing — one
    /// follows the desktop for as long as the app is open, the other is
    /// resolved once at startup — and the row title has nowhere to say
    /// so. The MCP rows carry a consequence rather than a definition:
    /// which addresses answer, and what happens to a port that is
    /// already taken.
    pub fn description(self) -> Message {
        match self {
            Self::Appearance => Message::SettingsAppearanceDescription,
            Self::Language => Message::SettingsLanguageDescription,
            Self::ShortcutShow => hotkey::Action::Show.description(),
            Self::ShortcutPanel => hotkey::Action::Panel.description(),
            Self::Setup => Message::SettingsSetupDescription,
            Self::WindowScreen => Message::SettingsPlacementScreenDescription,
            Self::CloseAfterDrop => Message::SettingsPlacementCloseDescription,
            Self::CompareGrain => Message::SettingsCompareGrainDescription,
            Self::CompareFollow => Message::SettingsCompareFollowDescription,
            Self::EngineServes => Message::SettingsEngineServesDescription,
            Self::EngineKeep => Message::SettingsEngineKeepDescription,
            Self::EngineIdle => Message::SettingsEngineIdleDescription,
            Self::EngineLock => Message::SettingsEngineLockDescription,
            Self::EngineProfile => Message::SettingsEngineProfileDescription,
            Self::EngineProvider => Message::SettingsEngineProviderDescription,
            Self::EngineEndpoint => Message::SettingsEngineEndpointDescription,
            Self::EngineModel => Message::SettingsEngineModelDescription,
            Self::EngineKey => Message::SettingsEngineKeyDescription,
            Self::EngineAllowRemote => Message::SettingsEngineAllowRemoteDescription,
            Self::EngineTemperature => Message::SettingsEngineTemperatureDescription,
            Self::EngineReasoning => Message::SettingsEngineReasoningDescription,
            Self::EngineTimeout => Message::SettingsEngineTimeoutDescription,
            Self::ModelsFolder => Message::SettingsModelsFolderDescription,
            Self::ModelForRewrite => Message::SettingsModelsRewriteDescription,
            Self::ResultsDestination => Message::SettingsRetentionDestinationDescription,
            Self::ResultsFolder => Message::SettingsRetentionFolderDescription,
            Self::KeepOriginals => Message::SettingsRetentionOriginalsDescription,
            Self::KeepResults => Message::SettingsRetentionResultsDescription,
            Self::KeepFor => Message::SettingsRetentionPeriodDescription,
            Self::ServeOverMcp => Message::SettingsMcpEnabledDescription,
            Self::McpBind => Message::SettingsMcpBindDescription,
            Self::McpPort => Message::SettingsMcpPortDescription,
        }
    }

    /// A caption over the row, for the one row that is set apart from
    /// the rows above it.
    pub fn caption(self) -> Option<Message> {
        match self {
            Self::EngineLock => Some(Message::SettingsEngineAdvanced),
            _ => None,
        }
    }

    /// Where this preference is kept.
    ///
    /// The link that `every_persisted_preference_has_a_row` walks in
    /// both directions: a key added to `config` with no row here, or a
    /// row here naming a key nothing persists, turns that test red.
    /// Nothing reads this at runtime — the rows write through
    /// `config::write_*`, which own their own spelling of the key.
    ///
    /// It is a two-variant answer and not a `&str` because one row is
    /// genuinely not a settings row, and an `Option` would have made
    /// that look like an oversight. See [`Storage::Credentials`].
    #[cfg(test)]
    pub fn storage(self) -> Storage {
        match self {
            Self::Appearance => Storage::Row(config::THEME_KEY),
            Self::Language => Storage::Row(config::LANGUAGE_KEY),
            Self::ShortcutShow => Storage::Row(config::hotkey_key(hotkey::Action::Show)),
            Self::ShortcutPanel => Storage::Row(config::hotkey_key(hotkey::Action::Panel)),
            // The row is "has the walk-through been through"; the
            // button beside it opens the walk-through, and it is the
            // walk-through's Finish and Skip that write the row.
            Self::Setup => Storage::Row(config::SETUP_DONE_KEY),
            Self::WindowScreen => Storage::Row(config::WINDOW_SCREEN_KEY),
            Self::CloseAfterDrop => Storage::Row(config::CLOSE_AFTER_DROP_KEY),
            Self::CompareGrain => Storage::Row(config::COMPARE_GRAIN_KEY),
            Self::CompareFollow => Storage::Row(config::COMPARE_FOLLOW_KEY),
            Self::EngineServes => Storage::Row(config::ENGINE_SERVES_KEY),
            Self::EngineKeep => Storage::Row(config::ENGINE_LOCAL_KEEP_KEY),
            Self::EngineIdle => Storage::Row(config::ENGINE_LOCAL_IDLE_KEY),
            Self::EngineLock => Storage::Row(config::ENGINE_LOCAL_MLOCK_KEY),
            Self::EngineProfile => Storage::Row(config::ENGINE_PROFILE_KEY),
            Self::EngineProvider => Storage::Row(config::ENGINE_PROVIDER_KEY),
            Self::EngineEndpoint => Storage::Row(config::ENGINE_BASE_URL_KEY),
            Self::EngineModel => Storage::Row(config::ENGINE_MODEL_KEY),
            Self::EngineKey => Storage::Credentials,
            Self::EngineAllowRemote => Storage::Row(config::ENGINE_ALLOW_REMOTE_KEY),
            Self::EngineTemperature => Storage::Row(config::ENGINE_TEMPERATURE_KEY),
            Self::EngineReasoning => Storage::Row(config::ENGINE_REASONING_KEY),
            Self::EngineTimeout => Storage::Row(config::ENGINE_TIMEOUT_KEY),
            Self::ModelsFolder => Storage::Row(config::MODELS_DIR_KEY),
            Self::ModelForRewrite => Storage::Row(config::MODEL_REWRITE_KEY),
            Self::ResultsDestination => Storage::Row(config::RESULTS_DESTINATION_KEY),
            Self::ResultsFolder => Storage::Row(config::RESULTS_FOLDER_KEY),
            Self::KeepOriginals => Storage::Row(config::KEEP_ORIGINALS_KEY),
            Self::KeepResults => Storage::Row(config::KEEP_RESULTS_KEY),
            Self::KeepFor => Storage::Row(config::KEEP_FOR_KEY),
            Self::ServeOverMcp => Storage::Row(config::MCP_ENABLED_KEY),
            Self::McpBind => Storage::Row(config::MCP_BIND_KEY),
            Self::McpPort => Storage::Row(config::MCP_PORT_KEY),
        }
    }
}

/// Where a row's value ends up when it is changed.
///
/// Two answers, and naming them is what keeps the second from looking
/// like a missing entry in the first. Everything the product knows
/// about a user's preferences is a row in `wipemark.db`; the one
/// exception is a credential, which goes to the operating system's own
/// store — see `crates/wipemark-secret` for why that distinction is not
/// a matter of taste.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Storage {
    /// A row in the `settings` table, under this key.
    Row(&'static str),
    /// The operating system's credential store, under an account
    /// derived from the endpoint. Never `wipemark.db`.
    Credentials,
}

/// Bind the shortcut that opens the window.
///
/// `secondary` is ⌘ on macOS and Ctrl everywhere else, which is what
/// every desktop expects of the comma. Called once, from `main`, after
/// `gpui_component::init` — a binding registered twice fires twice.
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("secondary-,", OpenSettings, None),
        // Both scoped to this window's context. Unscoped, either would
        // close the main window from anywhere — which on this
        // application means hiding it, a surprising thing for ⌘W to do
        // and a hostile thing for Escape.
        KeyBinding::new("secondary-w", CloseSettings, Some(CONTEXT)),
        // Escape is also the dropdown's own key, and the dropdown's
        // context is the deeper one: with the list open it closes the
        // list and stops there, and only propagates when there is
        // nothing of its own to close. So the window gets Escape when
        // the window is what Escape means.
        KeyBinding::new("escape", CloseSettings, Some(CONTEXT)),
    ]);
}

/// The Settings window, while there is one.
///
/// A global rather than a field on anything, because what it keeps is
/// the answer to "is one already open" — asked from the menu bar, from
/// the status bar and from the keyboard, none of which can see the
/// others.
struct Opened {
    window: AnyWindowHandle,
    /// The view inside it, so that a request naming a section can move
    /// an open window to it without going through `Root` — which is
    /// checked out for the length of a window update, and a second
    /// borrow of it is a panic. See the note at the top of `dialog`.
    view: Entity<SettingsView>,
}

impl Global for Opened {}

/// What the server is doing, as opposed to what it was asked to do.
///
/// The switch is the request and this is the answer, and keeping them
/// apart is the whole of the pane's honesty: a port that was taken, an
/// address this machine does not hold, a server still coming up — none
/// of those is visible from the preference alone, and all three are
/// things the user has to know before they paste a snippet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Status {
    /// Nothing is listening, and nothing was asked to.
    #[default]
    Off,
    /// Asked for, not yet answered. Also where a restart sits while it
    /// waits out the typing — see [`SETTLE`].
    Starting,
    /// Bound and answering. `endpoint` is where it actually is;
    /// `wanted` is the port that was asked for, and the two differ
    /// exactly when the scan had to step past something.
    Listening { endpoint: Endpoint, wanted: u16 },
    /// It did not come up, in the operating system's own words.
    Failed(String),
}

/// The preferences themselves. No widgets, and no window.
pub struct Preferences {
    /// What the user chose, not what is on screen: `System` resolves at
    /// apply time and can resolve differently a second later.
    theme: ThemePreference,
    /// Likewise for the language — except that `System` here resolves
    /// once, at startup. See the note in `wipemark_i18n::preference`:
    /// GPUI hands us an appearance change, and nothing hands us a
    /// language change.
    language: LanguagePreference,
    /// Whether the user wants Wipemark's work reachable over MCP.
    serving: bool,
    /// Where the server was asked to listen — not necessarily where it
    /// is. See [`Status::Listening`].
    endpoint: Endpoint,
    /// What actually happened, as the server itself reported it.
    status: Status,
    /// The one thing that owns a running server.
    ///
    /// It lives here, on the app-lifetime entity, and not on the
    /// Settings window: a server that ran only while its own
    /// preferences pane was open would be one no agent could rely on.
    supervisor: Supervisor,
    /// Which restart request is the current one.
    ///
    /// Every change of mind bumps it, and a debounced restart that
    /// wakes to find the number moved is a keystroke the user has
    /// already typed past. Without it, backspacing through a port
    /// leaves a queue of restarts that all come true half a second
    /// later.
    restarts: u64,
    /// What Layer B would send, and where. Nothing sends it in this
    /// build — E2 — which is what the banner at the top of the page
    /// says out loud.
    engine: EngineSettings,
    /// Every saved configuration, in [`profile::in_order`] order.
    ///
    /// Held here rather than read per frame because the page compares
    /// them against the live settings on every repaint — see
    /// [`profile::standing`] — and a table scan per frame is a table
    /// scan per frame.
    profiles: Vec<Profile>,
    /// Which profile the settings above were last applied from. A hint;
    /// [`profile::standing`] decides what the page says.
    active_profile: Option<String>,
    /// How many times a profile has been applied.
    ///
    /// The counter is the whole mechanism: applying a profile changes
    /// seven values at once, and the fields on the Settings window are
    /// the *source* of five of them. Something has to tell the window
    /// that this change came from here and not from a keystroke, and a
    /// number that only moves on an apply is that something. Without it
    /// the window either never updates its fields — so the next
    /// keystroke writes the old endpoint back over the new one — or
    /// rewrites them every frame, which takes the caret out of whatever
    /// is being typed.
    applied: u64,
    /// Whether a credential is stored for the endpoint above.
    ///
    /// Not the credential. This entity never holds one: the pane needs
    /// to know that a key exists, and a value nobody holds is a value
    /// nobody can spill into a log line or a panic message.
    key: KeyState,
    /// Which credential-store request is the current one.
    ///
    /// The same counter `restarts` is, for the same reason and one
    /// more: the endpoint field is typed into a character at a time and
    /// each keystroke that leaves a URL behind is a different account
    /// to look under, so a lookup that wakes to find the number moved
    /// is answering a question about an endpoint the user has already
    /// typed past. Without it, the last answer to arrive wins rather
    /// than the last one asked.
    key_lookups: u64,
    /// How many times a key was saved or forgotten — by anyone, whether
    /// or not the pane was still asking. The engine host rebuilds an
    /// endpoint's engine when it moves: the account is the same, the key
    /// under it is not.
    key_saves: u64,
    /// The operating system's credential store. `Arc` for the reason
    /// the store is one: every call to it happens on the background
    /// executor, because a keychain read blocks and can put a
    /// permission dialog on screen.
    vault: Arc<Vault>,
    /// The catalogue compiled into this binary. Read once — it is
    /// `include_str!`ed, so it cannot change while the process runs.
    catalogue: Manifest,
    /// What this machine can hold. `None` until the probe has answered,
    /// and it stays `None` if the probe could not read the machine —
    /// the page says "not judged" rather than "will not fit".
    host: Option<Host>,
    /// What is on disk, per entry id. Empty until the first scan, which
    /// is why the page can say it is still looking.
    installed: BTreeMap<String, State>,
    /// The weight file an engine would open, per entry id — filled by
    /// the same scan that fills `installed`, from
    /// `Downloads::weights_path`.
    ///
    /// Beside the states rather than derived on demand because deriving
    /// it means a `stat` per entry, and the frame is not where a disk
    /// is read. An id present here whose state is not `Present` is a
    /// pair `duty::on_duty` refuses: a path to a `.part` is a path to a
    /// file no runtime can open.
    weights: BTreeMap<String, PathBuf>,
    /// The one download in flight, if there is one.
    ///
    /// One at a time on purpose: two seven-gigabyte downloads over one
    /// connection finish later than the same two in sequence, and the
    /// page would have two progress bars and one disk-space check made
    /// before either started.
    download: Option<Running>,
    /// What the last download said when it ended, for as long as the
    /// page has not moved on. `None` once a new one starts.
    download_said: Option<Message>,
    /// The reason the last download failed, in the store's own words.
    /// Never localized — it names a URL, an HTTP status or an operating
    /// system error, and translating one of those strands whoever is
    /// being asked to debug it.
    download_error: Option<String>,
    /// Which downloaded model answers for rewriting. A manifest id, or
    /// `None`.
    rewrite_model: Option<String>,
    /// The folder the platform gives for the weights — `<data
    /// dir>/models` — which is where they go when the row says nothing.
    /// Kept beside the row so "Default" has something to put back and
    /// the field has something to show.
    models_default: PathBuf,
    /// The folder the row names instead, if it does. `None` is the
    /// default above — see `config::read_models_dir`.
    models_dir: Option<PathBuf>,
    /// The models directory and the client that fills it. `Arc` because
    /// the download runs on its own thread — see `Downloads::spawn`.
    /// Rebuilt over the new folder when the row moves; a download in
    /// flight keeps the old one, which is why the row cannot move while
    /// one runs.
    models: Arc<Downloads>,
    /// What the last scan found in the folder beyond the catalogue's
    /// own files — see `models::Folder`. `Unread` until the first scan
    /// answers, and again the moment the folder moves.
    folder: Folder,
    /// Which scan of the models directory is the current one.
    ///
    /// The same counter `key_lookups` is, for the same reason: a scan
    /// hashes files, so a slow one can outlive the download that
    /// invalidated it, and the last answer to arrive must not win over
    /// the last one asked for.
    scans: u64,
    /// Held as an `Arc` because the write happens on a background
    /// thread, and what crosses that boundary has to own itself. Never
    /// `None` — a store that could not be opened is an in-memory one
    /// that forgets, so the selectors work either way and there is no
    /// branch here that silently does nothing.
    store: SettingsStore,
    /// Which side serves, and in what order. The one preference that
    /// spans both pages, which is why it is not part of a profile: a
    /// profile names an endpoint, and whether an endpoint is asked at
    /// all is not the endpoint's business.
    serves: Serves,
    /// How long the model on this machine is kept, and whether it is
    /// locked in RAM — the three `engine.local.*` rows. Read by the
    /// `EngineHost`, which is where they take effect.
    local: LocalPolicy,
    /// Whether a ggml backend other than the CPU registered — `None` until
    /// the backends have been loaded with the scan, and always in a build
    /// without the local engine. See `duty::available_mb`.
    gpu: Option<bool>,
    /// Which screen a window opens on. One answer for every window,
    /// which is why it is a row and a radio button.
    onto: Onto,
    /// Whether dropping the window onto a zone closes the Settings
    /// window afterwards.
    ///
    /// One answer for the whole product and not one per display: it is
    /// about the gesture rather than about a monitor, which is why it
    /// is a row above the cards instead of a control on one.
    close_after_drop: bool,
    /// What each display was told, by uuid — a cell from the
    /// drag-and-drop grid, or the rectangle the window was put at by
    /// hand. A display that is **missing** from here has not been
    /// answered for and takes `Spot::default`; see
    /// [`placement::spot_for`], which is the one place that rule
    /// lives.
    ///
    /// Held rather than read per frame for the reason the profiles are:
    /// the page redraws whenever the pointer moves over it, and a query
    /// per card per frame is a query per card per frame. This entity is
    /// the only writer, so the map and the rows cannot disagree.
    spots: BTreeMap<String, Spot>,
    /// The displays attached right now, refreshed by
    /// [`display_watch`] — a monitor plugged in, unplugged, rearranged
    /// or woken at another resolution while this application runs.
    ///
    /// App-lifetime and not view-state on purpose: the displays keep
    /// changing while the Settings window is closed, and the window
    /// that opens afterwards has to be placed against what is attached
    /// *then*.
    screens: Vec<Connected>,
    /// Whether that list has ever been filled in.
    ///
    /// Its own fact rather than `!screens.is_empty()`, because the two
    /// are not the same sentence: an empty list after a reading is a
    /// machine with no displays, which the page would be wrong to
    /// describe as "still looking".
    screens_read: bool,
    /// The panel, while there is one.
    ///
    /// Held for one job: a zone chosen on the Placement page moves the
    /// panel while the user watches, and the page has no other way to
    /// reach it. `None` whenever the panel is not on screen, which is
    /// most of the time — it is a window you summon.
    panel: Option<AnyWindowHandle>,
    /// Which display the panel is on, as the panel itself last reported
    /// it.
    ///
    /// Reported rather than asked for: reading it needs a window
    /// update, and the page would be doing one per card per frame.
    panel_screen: Option<Screen>,
    /// The rectangle this application last asked the panel to be at, if
    /// it is still waiting to see it. See [`Preferences::we_placed_it`].
    expected: Option<Bounds<Pixels>>,
    /// The profile `--profile=<name>` pinned for this session, if one
    /// was named.
    ///
    /// A pin and not an application: it decides who answers for the
    /// HTTP side while this process runs and writes no row, so a flag
    /// on one launch cannot edit preferences set on another. It is
    /// deliberately not a setting — there is no row for it and no
    /// widget, because a preference that outlived the command line
    /// that asked for it would be a surprise waiting on the next
    /// launch.
    pinned: Option<String>,
    /// The system-wide shortcuts, per action. Absent is unset.
    hotkeys: BTreeMap<hotkey::Action, Hotkey>,
    /// What the desktop said about each of them — filled by `main`,
    /// which owns the registrar, for the reason [`Status`] is filled by
    /// the server: the row is the request and this is the answer.
    registrations: BTreeMap<hotkey::Action, Registration>,
    /// Whether the setup walk-through has been finished or skipped
    /// once. `main` reads the stored value to decide whether the main
    /// window opens it; this copy is what keeps [`setup_through`]
    /// from notifying every observer for a row that did not move.
    ///
    /// [`setup_through`]: Preferences::setup_through
    setup_done: bool,
    /// How many times the walk-through has been asked for since launch.
    ///
    /// The same mechanism [`applied`](Self::applied) is, for the same
    /// reason: the Settings window asks and the main window answers,
    /// and a number that only moves on a request is how the main window
    /// tells a request from a repaint.
    setup_asked: u64,
    /// Where results go and what is kept — the Retention page's rows,
    /// as values. Read by the page, and by the panel, which says what
    /// would happen to the thing just dropped on it.
    retention: Retention,
    /// How a result is shown beside its original — the Compare page's
    /// rows, as values. Read by the page, and by whoever opens a
    /// Compare window, which keeps what it was opened with.
    comparison: Comparison,
    /// The two folders those rows cannot name from the database alone:
    /// the platform's Downloads folder, and `<data dir>/kept`. Gathered
    /// by `main` once, for the reason `models_default` is.
    homes: Homes,
}

/// A download in flight.
///
/// The cancel flag lives here rather than beside the thread, because
/// the button that trips it is on a window that the thread knows
/// nothing about.
struct Running {
    /// The manifest id being fetched — which card draws the bar.
    id: String,
    cancel: Cancel,
    /// `None` between the thread starting and its first report.
    progress: Option<Progress>,
}

impl Preferences {
    /// Build the preferences, and start the server if the last session
    /// left it on.
    ///
    /// Takes a `Context` because of that last clause: the server has to
    /// come up with the application rather than when somebody first
    /// visits the page that describes it, and the events it sends back
    /// have to reach this entity from a plain thread. `flume` is the
    /// channel that spans the two executors — the same arrangement
    /// every long operation in this product uses.
    #[allow(
        clippy::too_many_arguments,
        reason = "each is gathered by `main` before the window exists; a struct of them would be a second `Stored`"
    )]
    pub fn new(
        stored: config::Stored,
        store: SettingsStore,
        vault: Arc<Vault>,
        models_default: PathBuf,
        homes: Homes,
        pinned: Option<String>,
        engine_handle: EngineHandle,
        cx: &Context<Self>,
    ) -> Self {
        let config::Stored {
            theme,
            language,
            serving,
            endpoint,
            engine,
            profiles,
            active_profile,
            rewrite_model,
            models_dir,
            serves,
            local,
            hotkeys,
            onto,
            close_after_drop,
            spots,
            setup_done,
            retention,
            comparison,
        } = stored;
        let (events, heard) = flume::unbounded();
        // The server holds a way to the engine from the start (D56): no
        // tool calls it until the pipeline puts Layer A and the guards
        // around a model, but the road is built and tested now.
        let supervisor = Supervisor::spawn(events, engine_handle);

        cx.spawn(async move |preferences, cx| {
            while let Ok(event) = heard.recv_async().await {
                // The entity is gone, so the application is on its way
                // out and there is nobody left to tell.
                if preferences
                    .update(cx, |preferences, cx| preferences.server_said(event, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();

        if serving {
            supervisor.serve(endpoint);
        }

        // The displays, now and whenever they change. Started here and
        // not when the Placement page is first drawn, for the reason
        // the server is started here: a page that learns about the
        // second monitor only because somebody visited it is a page
        // that is wrong until they do, and the window that opens
        // meanwhile is placed against a desktop nobody checked.
        display_watch::watch(cx, |preferences, screens, cx| {
            preferences.screens_changed(screens, cx);
        });

        Self {
            theme,
            language,
            serving,
            endpoint,
            status: if serving {
                Status::Starting
            } else {
                Status::Off
            },
            supervisor,
            restarts: 0,
            engine,
            profiles,
            active_profile,
            applied: 0,
            // Deliberately not looked up here. A keychain read blocks
            // and, for an unsigned build, prompts — a permission dialog
            // over a window that has not been drawn yet. The Settings
            // window asks when it opens, which is the first moment
            // anything needs the answer.
            key: KeyState::Unknown,
            key_lookups: 0,
            key_saves: 0,
            vault,
            catalogue: models::catalogue(),
            // Deliberately not probed or scanned here, for the reason
            // the key is not read here: the probe can spawn a process
            // and the scan can hash seven gigabytes, and neither belongs
            // between the user double-clicking and a window appearing.
            // `Shell::new` asks for both once the window exists, on the
            // background executor — the status bar cannot say who
            // rewrites without the answer.
            host: None,
            installed: BTreeMap::new(),
            weights: BTreeMap::new(),
            download: None,
            download_said: None,
            download_error: None,
            rewrite_model,
            // Where the weights go. A path and a client, and nothing
            // else: constructing this touches no disk, so a machine
            // that never downloads a model never gets a `models/`
            // directory.
            models: Arc::new(Downloads::new(
                models_dir.clone().unwrap_or_else(|| models_default.clone()),
            )),
            models_default,
            models_dir,
            folder: Folder::Unread,
            scans: 0,
            store,
            serves,
            local,
            // Asked with the scan, off this thread: registering the
            // backends loads every one of their libraries.
            gpu: None,
            onto,
            close_after_drop,
            spots,
            // Filled by the watch above, which takes its first reading
            // as soon as this task runs. Empty for those few frames,
            // and the page says "still looking" rather than "no
            // displays" — `screens_read` is the difference.
            screens: Vec::new(),
            screens_read: false,
            panel: None,
            panel_screen: None,
            expected: None,
            pinned,
            hotkeys,
            // Nothing has been asked for yet. `main::install_hotkeys`
            // registers what was read and writes the answer back,
            // once the registrar exists — which is after the window,
            // for the reason the tray is.
            registrations: BTreeMap::new(),
            setup_done,
            setup_asked: 0,
            retention,
            comparison,
            homes,
        }
    }

    /// The Retention page's rows.
    pub fn retention(&self) -> &Retention {
        &self.retention
    }

    /// The Compare page's rows.
    pub fn comparison(&self) -> Comparison {
        self.comparison
    }

    /// Record how finely a changed passage is marked.
    pub fn select_grain(&mut self, grain: Grain, cx: &mut Context<Self>) {
        if self.comparison.grain == grain {
            return;
        }
        self.comparison.grain = grain;
        cx.notify();
        self.persist(cx, move |store| config::write_compare_grain(store, grain));
    }

    /// Record whether the original follows the result's cursor.
    pub fn follow_cursor(&mut self, follow: bool, cx: &mut Context<Self>) {
        if self.comparison.follow == follow {
            return;
        }
        self.comparison.follow = follow;
        cx.notify();
        self.persist(cx, move |store| config::write_compare_follow(store, follow));
    }

    /// The two default folders — see [`Homes`].
    pub fn homes(&self) -> &Homes {
        &self.homes
    }

    /// The results folder in effect: the row's, or the platform's.
    pub fn results_folder(&self) -> &Path {
        self.retention
            .folder
            .as_deref()
            .unwrap_or(&self.homes.results)
    }

    /// Whether no row names a results folder.
    pub fn results_folder_is_default(&self) -> bool {
        self.retention.folder.is_none()
    }

    /// What would happen to `intake`, under the rows as they stand.
    pub fn plan_for(&self, intake: &wipemark_intake::Intake) -> retention::Plan {
        retention::plan(&retention::Source::of(intake), &self.retention, &self.homes)
    }

    /// Record where results go.
    pub fn select_destination(&mut self, destination: Destination, cx: &mut Context<Self>) {
        if self.retention.destination == destination {
            return;
        }
        self.retention.destination = destination;
        cx.notify();
        self.persist(cx, move |store| {
            config::write_results_destination(store, destination)
        });
    }

    /// Record the results folder. `None` puts the platform's back.
    pub fn select_results_folder(&mut self, dir: Option<PathBuf>, cx: &mut Context<Self>) {
        if self.retention.folder == dir {
            return;
        }
        self.retention.folder = dir.clone();
        cx.notify();
        self.persist(cx, move |store| {
            config::write_results_folder(store, dir.as_deref())
        });
    }

    /// Record whether originals that arrived without a file are kept.
    pub fn keep_originals(&mut self, keep: bool, cx: &mut Context<Self>) {
        if self.retention.keep_originals == keep {
            return;
        }
        self.retention.keep_originals = keep;
        cx.notify();
        self.persist(cx, move |store| config::write_keep_originals(store, keep));
    }

    /// Record whether their results are.
    pub fn keep_results(&mut self, keep: bool, cx: &mut Context<Self>) {
        if self.retention.keep_results == keep {
            return;
        }
        self.retention.keep_results = keep;
        cx.notify();
        self.persist(cx, move |store| config::write_keep_results(store, keep));
    }

    /// Record for how long.
    pub fn select_period(&mut self, period: Period, cx: &mut Context<Self>) {
        if self.retention.keep_for == period {
            return;
        }
        self.retention.keep_for = period;
        cx.notify();
        self.persist(cx, move |store| config::write_keep_for(store, period));
    }

    /// How many times the walk-through has been asked for. The main
    /// window keeps the number it last answered and opens the overlay
    /// when this one moves past it.
    pub fn setup_asked(&self) -> u64 {
        self.setup_asked
    }

    /// Ask the main window to open the walk-through.
    ///
    /// Writes nothing — see [`Setting::Setup`]'s storage note.
    pub fn ask_for_setup(&mut self, cx: &mut Context<Self>) {
        self.setup_asked += 1;
        cx.notify();
    }

    /// Record that the walk-through has been finished or skipped.
    ///
    /// Both write the same answer: what the row means is "has this been
    /// shown", and a walk-through somebody skipped past has been shown.
    /// It is never written `false` by this application — "Run again"
    /// opens the overlay without touching it, and a debug build's
    /// [`forget_setup`](Self::forget_setup) deletes it instead.
    pub fn setup_through(&mut self, cx: &mut Context<Self>) {
        if !self.setup_done {
            self.setup_done = true;
            cx.notify();
        }
        self.persist(cx, |store| config::write_setup_done(store, true));
    }

    /// Forget that the walk-through was shown, so the next launch is a
    /// first launch again. Debug builds only — see
    /// `config::forget_setup` for why a released build has no such
    /// control. Nothing opens now: the point is the *next* launch, and
    /// the path it takes by itself.
    #[cfg(debug_assertions)]
    pub fn forget_setup(&mut self, cx: &mut Context<Self>) {
        if self.setup_done {
            self.setup_done = false;
            cx.notify();
        }
        self.persist(cx, config::forget_setup);
    }

    /// The chord bound to `action`, if one is.
    pub fn shortcut(&self, action: hotkey::Action) -> Option<Hotkey> {
        self.hotkeys.get(&action).copied()
    }

    /// What the desktop said about the chord bound to `action`.
    pub fn registration(&self, action: hotkey::Action) -> &Registration {
        self.registrations
            .get(&action)
            .unwrap_or(&Registration::Unset)
    }

    /// Bind `chord` to `action` — or refuse, naming the action that
    /// already holds it.
    ///
    /// Refused *before* the row is written and not after the desktop
    /// complains: two rows with one chord would be two things asked of
    /// one key, and the second registration is the one the OS turns
    /// down. Nothing else moves; the registrar follows the change from
    /// `main` and reports back through [`Preferences::shortcut_registered`].
    pub fn assign_shortcut(
        &mut self,
        action: hotkey::Action,
        chord: Option<Hotkey>,
        cx: &mut Context<Self>,
    ) -> Result<(), hotkey::Action> {
        if let Some(chord) = chord {
            let assigned: Vec<(hotkey::Action, Hotkey)> = self
                .hotkeys
                .iter()
                .map(|(action, chord)| (*action, *chord))
                .collect();
            if let Some(holder) = hotkey::taken_by(&assigned, chord, action) {
                return Err(holder);
            }
        }
        if self.shortcut(action) == chord {
            return Ok(());
        }
        match chord {
            Some(chord) => self.hotkeys.insert(action, chord),
            None => self.hotkeys.remove(&action),
        };
        cx.notify();
        self.persist(cx, move |store| config::write_hotkey(store, action, chord));
        Ok(())
    }

    /// The desktop's answer to the chord bound to `action`.
    pub fn shortcut_registered(
        &mut self,
        action: hotkey::Action,
        registration: Registration,
        cx: &mut Context<Self>,
    ) {
        if self.registration(action) == &registration {
            return;
        }
        self.registrations.insert(action, registration);
        cx.notify();
    }

    /// Which screen a window opens on.
    pub fn onto(&self) -> Onto {
        self.onto
    }

    /// Record a new answer to "which screen".
    ///
    /// Nothing moves on screen: this decides where the *next* window
    /// goes, and a preferences window that jumped to another display
    /// while its own radio button was being read would be answering a
    /// question nobody asked. The zones below are the ones that apply
    /// while you watch, and only to the display they belong to.
    pub fn select_onto(&mut self, onto: Onto, cx: &mut Context<Self>) {
        if self.onto == onto {
            return;
        }
        self.onto = onto;
        cx.notify();
        self.persist(cx, move |store| config::write_window_screen(store, onto));
    }

    /// Whether a drop onto a zone closes the Settings window.
    pub fn closes_after_drop(&self) -> bool {
        self.close_after_drop
    }

    /// Record a new answer to that.
    pub fn close_after_drop(&mut self, close: bool, cx: &mut Context<Self>) {
        if self.close_after_drop == close {
            return;
        }
        self.close_after_drop = close;
        cx.notify();
        self.persist(cx, move |store| {
            config::write_close_after_drop(store, close)
        });
    }

    /// What this display was told, and the rectangle behind it. A
    /// display nobody has answered for takes the default, which is a
    /// place — see [`placement::spot_for`].
    pub fn spot_of(&self, screen: &Screen) -> Spot {
        placement::spot_for(&self.spots, screen)
    }

    /// Every answer this machine has been given, by display uuid —
    /// including for the displays that are not attached today, which is
    /// what makes unplugging a monitor cost nothing.
    pub fn spots(&self) -> &BTreeMap<String, Spot> {
        &self.spots
    }

    /// Put a cell in force on this display.
    ///
    /// One display: nothing about this touches another screen's row,
    /// which is the whole reason the answer is filed per display rather
    /// than as one preference with a screen attached to it. The
    /// hand-placed rectangle is kept — a cell says *where* a window
    /// goes and never how big it is, and the size the user dragged the
    /// window to is still the size they chose.
    pub fn choose_zone(&mut self, screen: &Screen, zone: Zone, cx: &mut Context<Self>) {
        self.answer(screen, placement::Answer::Zone(zone), None, cx);
    }

    /// The window was moved or resized by hand on this display, and
    /// that is now the answer.
    ///
    /// A hand beats a cell: dragging a window somewhere is a more
    /// specific instruction than pointing at a sixth of a screen, and
    /// the page stops ticking a cell until one is clicked again.
    pub fn put_by_hand(&mut self, screen: &Screen, rect: Bounds<Pixels>, cx: &mut Context<Self>) {
        self.expected = None;
        self.answer(screen, placement::Answer::Manual, Some(rect), cx);
    }

    /// Put the hand-placed rectangle back in force, without moving
    /// anything: the way back from a cell for somebody who chose one by
    /// mistake.
    pub fn back_to_hand(&mut self, screen: &Screen, cx: &mut Context<Self>) {
        self.answer(screen, placement::Answer::Manual, None, cx);
    }

    /// Forget everything this display was told.
    ///
    /// The one control that throws away the hand-placed rectangle as
    /// well as the cell, because it is the only thing a "restore
    /// default" can honestly mean: the size goes back too, and a
    /// rectangle kept behind the scenes would come back the next time
    /// somebody pressed "where I put it" on a display they had just
    /// reset.
    pub fn restore_default(&mut self, screen: &Screen, cx: &mut Context<Self>) {
        let Some(key) = screen.key.clone() else {
            return;
        };
        if self.spots.remove(&key).is_none() {
            return;
        }
        cx.notify();

        let screen = screen.clone();
        self.persist(cx, move |store| placement::forget(store, &screen));
    }

    /// Record one display's answer, and write it down.
    fn answer(
        &mut self,
        screen: &Screen,
        at: placement::Answer,
        rect: Option<Bounds<Pixels>>,
        cx: &mut Context<Self>,
    ) {
        let Some(key) = screen.key.clone() else {
            // A display the platform will not name stably cannot be
            // remembered — see `Screen::key`. Saying so is better than
            // a grid that appears to work until the next reboot.
            tracing::warn!("this display has no stable name; its windows open where they land");
            return;
        };

        let mut spot = self.spot_of(screen);
        spot.at = at;
        if rect.is_some() {
            spot.rect = rect;
        }
        if self.spots.get(&key) == Some(&spot) {
            return;
        }
        self.spots.insert(key, spot);
        cx.notify();

        let screen = screen.clone();
        self.persist(cx, move |store| placement::save(store, &screen, spot));
    }

    /// Remember that *we* asked the panel to be at this rectangle.
    ///
    /// The one thing that tells a placement from a person: the panel
    /// reports every bounds change the same way, and a move this
    /// application made must not be recorded as the user's answer. Held
    /// rather than consumed, because opening a window reports its
    /// rectangle more than once and all of those are still ours.
    pub fn we_placed_it(&mut self, bounds: Option<Bounds<Pixels>>) {
        self.expected = bounds;
    }

    /// Whether a rectangle the panel just reported is the one we asked
    /// for. A point of tolerance, because AppKit rounds a frame to
    /// whole points and our arithmetic does not.
    pub fn ours(&self, bounds: Bounds<Pixels>) -> bool {
        self.expected.is_some_and(|expected| {
            let near = |a: Pixels, b: Pixels| (a.as_f32() - b.as_f32()).abs() <= 1.0;
            near(expected.origin.x, bounds.origin.x)
                && near(expected.origin.y, bounds.origin.y)
                && near(expected.size.width, bounds.size.width)
                && near(expected.size.height, bounds.size.height)
        })
    }

    /// The panel has been summoned, and this is it.
    pub fn panel_opened(&mut self, panel: AnyWindowHandle, cx: &mut Context<Self>) {
        self.panel = Some(panel);
        cx.notify();
    }

    /// The panel has been dismissed.
    pub fn panel_closed(&mut self, cx: &mut Context<Self>) {
        self.panel = None;
        self.panel_screen = None;
        cx.notify();
    }

    /// The panel, for the one thing that has to move it.
    pub fn panel_window(&self) -> Option<AnyWindowHandle> {
        self.panel
    }

    /// Which display the panel is on, or `None` while there is no
    /// panel.
    pub fn panel_screen(&self) -> Option<&Screen> {
        self.panel_screen.as_ref()
    }

    /// The panel has opened, or has been moved onto another display.
    pub fn panel_moved(&mut self, screen: Option<Screen>, cx: &mut Context<Self>) {
        if self.panel_screen == screen {
            return;
        }
        self.panel_screen = screen;
        cx.notify();
    }

    /// The displays attached right now.
    pub fn screens(&self) -> &[Connected] {
        &self.screens
    }

    /// Whether the displays have been read at all yet.
    ///
    /// The same distinction the Models page makes between "nothing
    /// installed" and "not looked yet": an empty list is a machine with
    /// no displays, which is not a state a desktop is in, so the page
    /// has to be able to tell the two apart.
    pub fn screens_scanned(&self) -> bool {
        self.screens_read
    }

    /// A reading from [`display_watch`].
    ///
    /// The comparison lives here, beside the state, so the watcher can
    /// hand over a reading on a timer without knowing whether anything
    /// has moved — and so a repaint is a repaint rather than a wake-up
    /// for every view in the application three times a second.
    fn screens_changed(&mut self, screens: Vec<Connected>, cx: &mut Context<Self>) {
        // The first reading is always news, even when it is the empty
        // list it started as: what changed is that somebody looked.
        let first = !std::mem::replace(&mut self.screens_read, true);
        if !first && !display_watch::moved(&self.screens, &screens) {
            return;
        }
        tracing::debug!(displays = screens.len(), "the displays changed");
        self.screens = screens;
        cx.notify();
    }

    /// Record a new answer to "who rewrites".
    ///
    /// Nothing else moves: the endpoint fields, the model choice and
    /// the credential all stay exactly where they are, because this
    /// setting says which of them is *read*, not what any of them is.
    pub fn select_serves(&mut self, serves: Serves, cx: &mut Context<Self>) {
        if self.serves == serves {
            return;
        }
        self.serves = serves;
        cx.notify();
        self.persist(cx, move |store| config::write_engine_serves(store, serves));
    }

    /// Which side serves, and in what order.
    pub fn serves(&self) -> Serves {
        self.serves
    }

    /// The local model's three rows.
    pub fn local_policy(&self) -> LocalPolicy {
        self.local
    }

    /// How the machine's engine would be built now.
    pub fn local_options(&self) -> LocalOptions {
        LocalOptions {
            lock: self.local.lock,
            host: self.host,
            gpu: self.gpu,
        }
    }

    /// Load when needed, or keep loaded.
    pub fn select_keeping(&mut self, keeping: Keeping, cx: &mut Context<Self>) {
        if self.local.keeping == keeping {
            return;
        }
        self.local.keeping = keeping;
        cx.notify();
        self.persist(cx, move |store| config::write_local_keep(store, keeping));
    }

    /// How many idle minutes unload an on-demand model.
    pub fn select_idle(&mut self, minutes: u32, cx: &mut Context<Self>) {
        if self.local.idle_minutes == minutes || !IDLE_MINUTES.contains(&minutes) {
            return;
        }
        self.local.idle_minutes = minutes;
        cx.notify();
        self.persist(cx, move |store| config::write_local_idle(store, minutes));
    }

    /// Lock the model in RAM, or not. Takes effect at the next load: the
    /// engine is rebuilt with it, and a loaded model is unloaded first.
    pub fn select_lock(&mut self, lock: bool, cx: &mut Context<Self>) {
        if self.local.lock == lock {
            return;
        }
        self.local.lock = lock;
        cx.notify();
        self.persist(cx, move |store| config::write_local_mlock(store, lock));
    }

    /// The catalogue this build ships.
    pub fn catalogue(&self) -> &Manifest {
        &self.catalogue
    }

    /// What this machine can hold, once it has been read.
    pub fn host(&self) -> Option<Host> {
        self.host
    }

    /// What is on disk for `id`. An entry nothing has scanned yet reads
    /// as absent, which is what the page should offer to fix.
    pub fn model_state(&self, id: &str) -> State {
        self.installed.get(id).cloned().unwrap_or(State::Absent)
    }

    /// The download in flight, if it is `id`'s.
    pub fn downloading(&self, id: &str) -> Option<&Progress> {
        let running = self.download.as_ref()?;
        (running.id == id).then_some(running.progress.as_ref())?
    }

    /// True while any download is running — the other cards' buttons are
    /// disabled while one is, because the disk-space check was made for
    /// this one alone.
    pub fn any_download_running(&self) -> bool {
        self.download.is_some()
    }

    /// Whether the first scan has answered.
    ///
    /// The probe and the scan land together, so the machine's memory
    /// being known is the same fact as the directory having been read.
    /// Before that the page says it is still looking, rather than that
    /// nothing is installed — a card that offered a Download for a
    /// model already on the disk would start a scan's worth of hashing
    /// all over again.
    pub fn models_scanned(&self) -> bool {
        self.host.is_some()
    }

    /// Where the weights live: the row's folder, or the platform's.
    pub fn models_dir(&self) -> &Path {
        self.models.models_dir()
    }

    /// True while no row names a folder.
    pub fn models_dir_is_default(&self) -> bool {
        self.models_dir.is_none()
    }

    /// What the last scan found in the folder beyond the catalogue's
    /// own files.
    pub fn folder(&self) -> &Folder {
        &self.folder
    }

    /// How many catalogue entries are whole on this machine.
    pub fn installed_count(&self) -> usize {
        self.installed
            .values()
            .filter(|state| matches!(state, State::Present { .. }))
            .count()
    }

    /// Move the models folder, or put the default back with `None`.
    ///
    /// Refused — `false`, and nothing moves — while a download runs:
    /// the bytes are landing in the old folder through a client the
    /// download thread holds, and a page that showed the new folder
    /// over a bar filling the old one would be lying about where the
    /// file will be. The buttons are disabled for the same span, so
    /// this is the second wall and not the first.
    ///
    /// Everything the old scan said is forgotten before the new one is
    /// asked for, rather than left on screen until it answers: an
    /// "installed" tick over a folder that has not been read is the
    /// kind of provisional answer `models_scanned` exists to avoid. The
    /// chosen rewrite model is **kept**. It names a catalogue entry, not
    /// a path; if the new folder has the same entry downloaded, the
    /// choice is still right, and if it does not, `duty` says so and
    /// switching back restores it. Clearing it here would make trying
    /// a second folder cost the choice made in the first.
    pub fn select_models_dir(&mut self, dir: Option<PathBuf>, cx: &mut Context<Self>) -> bool {
        if self.download.is_some() {
            tracing::info!("the models folder cannot move while a download runs");
            return false;
        }
        if dir == self.models_dir {
            return true;
        }
        let effective = dir.clone().unwrap_or_else(|| self.models_default.clone());
        tracing::info!(folder = %effective.display(), "models folder moved");
        self.models = Arc::new(Downloads::new(effective));
        self.models_dir = dir;
        self.installed.clear();
        self.weights.clear();
        self.folder = Folder::Unread;
        self.download_said = None;
        self.download_error = None;
        let row = self.models_dir.clone();
        self.persist(cx, move |store| {
            config::write_models_dir(store, row.as_deref())
        });
        self.look_at_models(cx);
        cx.notify();
        true
    }

    /// How the last download ended, and why — the second one in the
    /// store's own words, never translated.
    pub fn download_outcome(&self) -> (Option<Message>, Option<&str>) {
        (self.download_said, self.download_error.as_deref())
    }

    /// Which model was chosen for rewriting.
    pub fn rewrite_model(&self) -> Option<&str> {
        self.rewrite_model.as_deref()
    }

    /// Every catalogue entry that is on this machine and serves `role`.
    pub fn installed_for(&self, role: Role) -> Vec<&ModelEntry> {
        models::installed_for(&self.catalogue, role, &self.installed)
    }

    /// Who serves `role` right now, and how.
    ///
    /// Assembled from state this entity is already holding; the rule
    /// itself lives in [`crate::duty`], where it is a pure function
    /// over values and can be read without a window. Nothing here
    /// touches the disk or the credential store — the scan and the
    /// lookup have already happened, and this is what they add up to.
    pub fn duty(&self, role: Role) -> Duty {
        let mut chosen = BTreeMap::new();
        if let Some(id) = &self.rewrite_model {
            chosen.insert(Role::Rewrite, id.clone());
        }
        let on_disk: BTreeMap<String, OnDisk> = self
            .installed
            .iter()
            .map(|(id, state)| {
                (
                    id.clone(),
                    OnDisk {
                        state: state.clone(),
                        weights: self.weights.get(id).cloned(),
                    },
                )
            })
            .collect();
        // The one account this window has asked about. An endpoint it
        // has not looked under is missing rather than empty, which
        // `duty` reads as "not yet known" and not as "no key".
        let keys = BTreeMap::from([(engine::account_of(&self.engine.base_url), self.key.clone())]);

        duty::on_duty(
            &Roster {
                live: &self.engine,
                profiles: &self.profiles,
                active: self.active_profile.as_deref(),
                keys: &keys,
                catalogue: &self.catalogue,
                chosen: &chosen,
                on_disk: &on_disk,
                host: self.host,
                serves: self.serves,
            },
            role,
            // The pin, when the command line named one. Nothing else
            // can set it: a profile chosen in the window is *applied*,
            // which moves the fields, and the live settings are then
            // the answer already.
            match self.pinned.as_deref() {
                Some(name) => duty::Pick::Named(name),
                None => duty::Pick::Live,
            },
        )
    }

    /// Read the machine and the models directory.
    ///
    /// Both on the background executor, and for the same reason: the
    /// probe may spawn the graphics driver's own tool, and the scan
    /// re-hashes any file whose size or mtime moved — seven gigabytes
    /// of reading, on the thread that draws the window.
    ///
    /// Called once the main window exists, again when the Settings
    /// window opens, and after anything that changes what is on disk.
    pub fn look_at_models(&mut self, cx: &Context<Self>) {
        self.scans += 1;
        let mine = self.scans;
        let models = self.models.clone();
        let entries: Vec<ModelEntry> = self.catalogue.models.clone();

        cx.spawn(async move |preferences, cx| {
            let found = cx
                .background_executor()
                .spawn(async move {
                    let host = Host::probe();
                    let gpu = gpu_backend();
                    let states: BTreeMap<String, State> = entries
                        .iter()
                        .map(|entry| (entry.id.clone(), models.state(entry)))
                        .collect();
                    // The path an engine is handed, gathered here so
                    // that answering "who rewrites" later costs no I/O.
                    let weights: BTreeMap<String, PathBuf> = entries
                        .iter()
                        .filter_map(|entry| {
                            models.weights_path(entry).map(|at| (entry.id.clone(), at))
                        })
                        .collect();
                    // Everything else in the folder, however deep. The
                    // catalogue's own files are the ones above,
                    // whatever state they are in — a damaged download
                    // is still the catalogue's, and belongs on its card
                    // rather than in the list of strangers.
                    let folder = Folder::from_listing(
                        wipemark_models::scan::weights_under(models.models_dir()),
                        |path| weights.values().any(|ours| ours == path),
                    );
                    (host, gpu, states, weights, folder)
                })
                .await;

            preferences
                .update(cx, |preferences, cx| {
                    // A newer scan was asked for while this one hashed.
                    // Its answer is the one that describes the disk.
                    if preferences.scans != mine {
                        return;
                    }
                    let (host, gpu, states, weights, folder) = found;
                    preferences.host = Some(host);
                    preferences.gpu = gpu;
                    preferences.installed = states;
                    preferences.weights = weights;
                    preferences.folder = folder;
                    // The moment the answer stops being provisional:
                    // before the scan lands, "that model is not here"
                    // is a sentence about a directory nobody has read.
                    preferences.log_the_duty();
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    /// Write down who would rewrite, and what stands in the way.
    ///
    /// To the log and not to the screen. The first question a support
    /// answer asks is which engine was on duty, and a screenshot of a
    /// status bar is not evidence — this is the line that is. Never
    /// localized, for the reason no log line is: it is read by whoever
    /// is debugging, not by whoever ran the program. It names a model
    /// id, an origin and a refusal, and no document text ever reaches
    /// it.
    fn log_the_duty(&self) {
        let duty = self.duty(Role::Rewrite);
        let Some(performer) = duty.performer() else {
            tracing::info!(
                role = Role::Rewrite.id(),
                vacancy = ?duty.vacancy(),
                "nothing is on duty to rewrite"
            );
            return;
        };
        let about = performer.info();
        // Not `engine_for` here: building an engine starts a worker
        // thread, and a log line is not a reason to. The `EngineHost`
        // logs what it built, or why it could not, when it builds it.
        tracing::info!(
            role = Role::Rewrite.id(),
            vendor = about.vendor.as_str(),
            model = about.model_id,
            ctx = ?about.ctx_len,
            stays_here = performer.stays_on_this_machine(),
            local = ?self.local,
            "on duty to rewrite"
        );
    }

    /// Fetch one catalogue entry, or carry on fetching it.
    ///
    /// Refuses while another download is running: the free-space check
    /// that ran before this one started was made for one file.
    pub fn download_model(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.download.is_some() {
            return;
        }
        let Some(entry) = self.catalogue.get(id).cloned() else {
            return;
        };
        let cancel = Cancel::new();
        self.download = Some(Running {
            id: entry.id.clone(),
            cancel: cancel.clone(),
            progress: None,
        });
        self.download_said = None;
        self.download_error = None;
        cx.notify();

        let events = self.models.clone().spawn(entry, cancel);
        cx.spawn(async move |preferences, cx| {
            while let Ok(event) = events.recv_async().await {
                if preferences
                    .update(cx, |preferences, cx| preferences.download_said(event, cx))
                    .is_err()
                {
                    // The window is gone and the application is on its
                    // way out. The thread's own cancel flag is dropped
                    // with it.
                    break;
                }
            }
        })
        .detach();
    }

    /// Stop the download in flight. What is on disk is kept: the next
    /// attempt carries on from it.
    pub fn stop_download(&self, cx: &mut Context<Self>) {
        if let Some(running) = &self.download {
            running.cancel.cancel();
            cx.notify();
        }
    }

    /// Delete everything downloaded for `id`.
    ///
    /// On the background executor, because removing seven gigabytes is
    /// not instant on every filesystem. Clears the selection if the
    /// removed model was the chosen one — a preference naming a file
    /// that is not there is worse than no preference.
    pub fn remove_model(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(entry) = self.catalogue.get(id).cloned() else {
            return;
        };
        if self.rewrite_model.as_deref() == Some(id) {
            self.select_rewrite_model(None, cx);
        }
        let models = self.models.clone();
        cx.spawn(async move |preferences, cx| {
            cx.background_executor()
                .spawn(async move {
                    if let Err(error) = models.remove(&entry) {
                        tracing::warn!(%error, model = %entry.id, "could not remove a model");
                    }
                })
                .await;
            preferences
                .update(cx, |preferences, cx| preferences.look_at_models(cx))
                .ok();
        })
        .detach();
    }

    /// Put a model that has just finished downloading to work, if
    /// nothing else holds the role it serves.
    ///
    /// The expensive half of choosing a model is fetching it. Somebody
    /// who has waited out two and a half gigabytes and is then told
    /// that nothing is on duty has been asked the same question twice,
    /// and the second time in a dropdown three rows above the button
    /// they just pressed.
    ///
    /// **Only the first one.** A second download is a comparison, not a
    /// replacement: someone who has chosen a rewriter and then fetches
    /// another to try it has not asked for the switch, and a selection
    /// that moved on its own is one they would have to notice before
    /// they could undo it. The mirror is already here —
    /// `remove_model` clears the choice when the chosen model is
    /// deleted, because a preference naming a file that is not there is
    /// worse than no preference.
    fn adopt(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(entry) = models::adopted(
            &self.catalogue,
            Role::Rewrite,
            self.rewrite_model.as_deref(),
            id,
        ) else {
            return;
        };
        let adopted = entry.id.clone();
        tracing::info!(
            role = Role::Rewrite.id(),
            model = adopted,
            "put a downloaded model to work"
        );
        self.select_rewrite_model(Some(adopted), cx);
    }

    /// Choose the model that answers for rewriting.
    pub fn select_rewrite_model(&mut self, id: Option<String>, cx: &mut Context<Self>) {
        self.rewrite_model = id;
        let chosen = self.rewrite_model.clone();
        self.persist(cx, move |store| {
            config::write_model(store, Role::Rewrite, chosen.as_deref())
        });
        cx.notify();
    }

    /// One event from the download thread.
    fn download_said(&mut self, event: Event, cx: &mut Context<Self>) {
        match event {
            Event::Started { .. } => {}
            Event::Progress(progress) => {
                if let Some(running) = self.download.as_mut() {
                    running.progress = Some(progress);
                }
            }
            Event::Finished(_) => {
                let finished = self.download.take().map(|running| running.id);
                self.download_said = None;
                if let Some(id) = finished {
                    self.adopt(&id, cx);
                }
                self.look_at_models(cx);
            }
            Event::Failed(reason) => {
                self.download = None;
                self.download_said = Some(match reason {
                    Some(_) => Message::SettingsModelsFailed,
                    // Cancelled. Not a failure, and the sentence says
                    // what survived rather than what stopped.
                    None => Message::SettingsModelsStopped,
                });
                self.download_error = reason;
                // A cancelled download leaves a `.part`, and a failed
                // one may have left nothing — either way the card has
                // to be told what is actually there now.
                self.look_at_models(cx);
            }
        }
        cx.notify();
    }

    pub fn store(&self) -> SettingsStore {
        self.store.clone()
    }

    /// Re-apply the theme after the OS changed appearance under us.
    ///
    /// Light and Dark are the user overriding the OS, so they survive
    /// the flip untouched; `System` is a standing promise to follow it,
    /// and this is where the promise is kept. Called from the main
    /// window's appearance observer, which outlives the Settings window
    /// by design.
    pub fn follow_the_system(&self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.theme.follows_system() {
            return;
        }
        self.theme.apply(Some(window), cx);
        cx.refresh_windows();
        cx.notify();
    }

    /// Apply a theme and remember it, wherever the choice came from —
    /// the Settings window, or the menu bar's Appearance submenu.
    pub fn select_theme(
        &mut self,
        choice: ThemePreference,
        window: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
        if self.theme == choice {
            return;
        }
        self.theme = choice;
        choice.apply(window, cx);
        // The palette is a global and `Theme::change` refreshes only
        // the window it was handed. With a Settings window open there
        // are two, and repainting one of them is worse than repainting
        // neither: half the application changes colour.
        cx.refresh_windows();
        cx.notify();

        // Every theme change passes through here, including the ones the
        // menu bar itself sent, so this is the one place the tick has to
        // be moved. A submenu showing last minute's state is worse than
        // no submenu.
        if let Some(tray) = cx.try_global::<Tray>() {
            tray.show_theme(choice);
        }

        self.persist(cx, move |store| config::write_theme(store, choice));
    }

    /// Change the language of every string in the process.
    ///
    /// Not on the background executor, unlike the write below it: this
    /// reparses a few kilobytes of embedded text and touches no
    /// filesystem at all, and it has to be done before the `notify`
    /// that repaints the windows. The rule is about blocking calls, not
    /// about work.
    pub fn select_language(&mut self, choice: LanguagePreference, cx: &mut Context<Self>) {
        if self.language == choice {
            return;
        }
        self.language = choice.clone();
        let resolved = wipemark_i18n::select(&choice);
        tracing::info!(%resolved, "language changed");

        // The menu bar is built once and never repainted, so this is the
        // only thing that carries the change up into it.
        if let Some(tray) = cx.try_global::<Tray>() {
            tray.relabel();
        }
        // Both windows are now showing last language's words, and
        // neither of them is this entity. The observers put the titles
        // and the rows right; this is what wakes them.
        cx.refresh_windows();
        cx.notify();

        self.persist(cx, move |store| config::write_language(store, &choice));
    }

    /// Whether the user has asked for the MCP server, and where they
    /// asked for it.
    pub fn mcp(&self) -> (bool, Endpoint) {
        (self.serving, self.endpoint)
    }

    /// What the server is actually doing.
    pub fn status(&self) -> &Status {
        &self.status
    }

    /// Start or stop the server, and remember which was asked for.
    pub fn serve_over_mcp(&mut self, serving: bool, cx: &mut Context<Self>) {
        if self.serving == serving {
            return;
        }
        self.serving = serving;
        // A restart still waiting out the last keystroke is no longer
        // what the user wants, whichever way this switch just went.
        self.restarts += 1;

        if serving {
            self.status = Status::Starting;
            self.supervisor.serve(self.endpoint);
        } else {
            // Said here rather than waited for: the supervisor will
            // confirm it a moment later, and a switch that stays on
            // until a thread gets round to answering is a switch that
            // looks broken.
            self.status = Status::Off;
            self.supervisor.stop();
        }
        cx.notify();
        self.persist(cx, move |store| config::write_mcp_enabled(store, serving));
    }

    /// Record which interface the server should answer on, and move it
    /// there if it is running.
    pub fn select_bind(&mut self, bind: BindAddress, cx: &mut Context<Self>) {
        if self.endpoint.bind == bind {
            return;
        }
        self.endpoint.bind = bind;
        self.restart_soon(cx);
        cx.notify();
        self.persist(cx, move |store| config::write_mcp_bind(store, bind));
    }

    /// Record the port, and move the server to it if it is running. The
    /// caller has already refused anything this build could not bind —
    /// see [`mcp::port`].
    pub fn select_port(&mut self, port: u16, cx: &mut Context<Self>) {
        if self.endpoint.port == port {
            return;
        }
        self.endpoint.port = port;
        self.restart_soon(cx);
        cx.notify();
        self.persist(cx, move |store| config::write_mcp_port(store, port));
    }

    /// What Layer B is configured to do.
    pub fn engine(&self) -> &EngineSettings {
        &self.engine
    }

    /// Whether a credential is stored for the endpoint on screen.
    pub fn key(&self) -> &KeyState {
        &self.key
    }

    /// The credential store, for the engine host: it reads an endpoint's
    /// key when it builds the engine, on the background executor.
    pub fn vault(&self) -> Arc<Vault> {
        Arc::clone(&self.vault)
    }

    /// How many times a key was saved or forgotten.
    pub fn key_saves(&self) -> u64 {
        self.key_saves
    }

    /// Whether what is put in the credential store outlives the
    /// process. The pane says so when the answer is no, because a key
    /// that lasts until the window closes is a thing to know before
    /// typing one in.
    pub fn credentials_persist(&self) -> bool {
        self.vault.is_persistent()
    }

    /// Every saved configuration.
    pub fn profiles(&self) -> &[Profile] {
        &self.profiles
    }

    /// How many times a profile has been applied. The window watches it
    /// so that it can tell an apply from a keystroke — see
    /// [`Preferences::applied`].
    pub fn applied(&self) -> u64 {
        self.applied
    }

    /// Where the settings on screen stand relative to what is saved.
    pub fn standing(&self) -> Standing {
        profile::standing(&self.profiles, self.active_profile.as_deref(), &self.engine)
    }

    /// Put a saved configuration on screen.
    ///
    /// Every field at once, which is the point of the control: the
    /// alternative is retyping a URL, a model name and three numbers to
    /// move between two endpoints somebody uses every day.
    ///
    /// The key is not part of it and does not need to be. A profile
    /// carries an endpoint, the credential store is keyed by that
    /// endpoint's origin, and so switching profiles switches which
    /// account is looked under — which is why the lookup below is
    /// unconditional rather than guarded on the account having moved.
    /// The account has moved, or the profile is the one already on
    /// screen.
    pub fn apply_profile(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(chosen) = profile::find(&self.profiles, id) else {
            // A row the window offered and this entity no longer has.
            // Nothing to do but say so: the list is rebuilt from the
            // same place the click came from, so the next frame is
            // already right.
            tracing::warn!(id, "no saved profile by that name");
            return;
        };

        let settings = chosen.settings.clone();
        let id = chosen.id.clone();
        self.engine = settings.clone();
        self.active_profile = Some(id.clone());
        self.applied += 1;
        self.look_up_key(Duration::ZERO, cx);
        cx.notify();

        let pointer = id.clone();
        self.persist(cx, move |store| {
            // Through the same single-key writers a keystroke goes
            // through, so that a profile applied and the same values
            // typed by hand leave the database in exactly one state.
            config::write_engine(store, &settings)?;
            config::write_active_profile(store, Some(&pointer))
        });
    }

    /// Save what is on screen under `name`.
    ///
    /// A name already in the list updates that profile rather than
    /// growing a second one beside it — see [`profile::id_of`], which is
    /// where “already in the list” is decided. That is what a user means
    /// by typing a name they have used before, and it is what makes
    /// renaming `work gw` to `Work GW` one write rather than a delete
    /// and an insert.
    ///
    /// Nothing here touches the credential store. A profile records
    /// where a request goes; what proves who is asking stays filed under
    /// that endpoint's origin, shared by every profile pointing at it.
    pub fn save_profile(&mut self, name: &str, cx: &mut Context<Self>) {
        let Some(mut saved) = Profile::new(name, self.engine.clone()) else {
            // The field is what stops this being reachable — Save is
            // disabled while the name is one nothing can be filed under.
            return;
        };

        // Under the id of the profile this name already refers to, not
        // under a freshly derived one. The two are the same for every
        // row this build wrote; they differ for a row that arrived by
        // hand, and forking that into a second copy is exactly what a
        // user pressing Save on a profile they are looking at does not
        // mean. See `profile::by_name`.
        if let Some(existing) = profile::by_name(&self.profiles, name) {
            saved.id.clone_from(&existing.id);
        }

        match self
            .profiles
            .iter_mut()
            .find(|existing| existing.id == saved.id)
        {
            Some(existing) => existing.clone_from(&saved),
            None => self.profiles.push(saved.clone()),
        }
        profile::in_order(&mut self.profiles);
        self.active_profile = Some(saved.id.clone());
        cx.notify();

        self.persist(cx, move |store| {
            config::write_profile(store, &saved)?;
            config::write_active_profile(store, Some(&saved.id))
        });
    }

    /// Forget one saved configuration.
    ///
    /// The settings on screen are deliberately left alone. Deleting the
    /// profile the page is sitting on removes the saved copy and nothing
    /// else, so the most this can cost is a name — the values that were
    /// under it are still in the fields, and saving them again is one
    /// click. That is what makes this the one destructive control on
    /// these pages that asks nothing before it acts.
    pub fn forget_profile(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(at) = self.profiles.iter().position(|saved| saved.id == id) else {
            return;
        };
        self.profiles.remove(at);
        let was_active = self.active_profile.as_deref() == Some(id);
        if was_active {
            self.active_profile = None;
        }
        cx.notify();

        let id = id.to_owned();
        self.persist(cx, move |store| {
            config::forget_profile(store, &id)?;
            if was_active {
                config::write_active_profile(store, None)?;
            }
            Ok(())
        });
    }

    /// Choose which shape of request Layer B sends.
    ///
    /// The key state goes with it: the two providers do not share one —
    /// Ollama's native API has no `Authorization` header at all — so
    /// leaving the last provider's answer on screen would be a sentence
    /// about a question nobody asked.
    pub fn select_provider(&mut self, provider: Provider, cx: &mut Context<Self>) {
        if self.engine.provider == provider {
            return;
        }
        self.engine.provider = provider;
        self.look_up_key(Duration::ZERO, cx);
        cx.notify();
        self.persist(cx, move |store| {
            config::write_engine_provider(store, provider)
        });
    }

    /// Record the endpoint, and look under its name for a key.
    ///
    /// Debounced, because this is called from a field that is typed
    /// into: `https://a`, `https://ap`, `https://api` are three
    /// endpoints on the way to one, and each is a different credential
    /// account. Looking each of them up would be three keychain reads
    /// nobody asked for and, on a first run, three permission prompts.
    pub fn select_endpoint(&mut self, base_url: BaseUrl, cx: &mut Context<Self>) {
        if self.engine.base_url == base_url {
            return;
        }
        let account_moved =
            engine::account_of(&self.engine.base_url) != engine::account_of(&base_url);
        self.engine.base_url = base_url.clone();
        if account_moved {
            self.look_up_key(SETTLE, cx);
        }
        cx.notify();
        self.persist(cx, move |store| {
            config::write_engine_base_url(store, &base_url)
        });
    }

    /// Record the model name, exactly as the endpoint spells it.
    pub fn select_model(&mut self, model: String, cx: &mut Context<Self>) {
        if self.engine.model == model {
            return;
        }
        self.engine.model = model.clone();
        cx.notify();
        self.persist(cx, move |store| config::write_engine_model(store, &model));
    }

    /// Record whether the document may leave this machine.
    pub fn allow_remote(&mut self, allowed: bool, cx: &mut Context<Self>) {
        if self.engine.allow_remote == allowed {
            return;
        }
        self.engine.allow_remote = allowed;
        cx.notify();
        self.persist(cx, move |store| {
            config::write_engine_allow_remote(store, allowed)
        });
    }

    /// Record the temperature. The caller has already refused anything
    /// out of range — see `engine::temperature`.
    pub fn select_temperature(&mut self, temperature: f32, cx: &mut Context<Self>) {
        if self.engine.temperature == temperature {
            return;
        }
        self.engine.temperature = temperature;
        cx.notify();
        self.persist(cx, move |store| {
            config::write_engine_temperature(store, temperature)
        });
    }

    /// Record how hard a reasoning model should think about a
    /// paraphrase.
    pub fn select_reasoning(&mut self, reasoning: ReasoningEffort, cx: &mut Context<Self>) {
        if self.engine.reasoning == reasoning {
            return;
        }
        self.engine.reasoning = reasoning;
        cx.notify();
        self.persist(cx, move |store| {
            config::write_engine_reasoning(store, reasoning)
        });
    }

    /// Record how long to wait for one response.
    pub fn select_timeout(&mut self, seconds: u32, cx: &mut Context<Self>) {
        if self.engine.timeout == seconds {
            return;
        }
        self.engine.timeout = seconds;
        cx.notify();
        self.persist(cx, move |store| {
            config::write_engine_timeout(store, seconds)
        });
    }

    /// Ask the credential store whether this endpoint has a key.
    ///
    /// `after` is zero for a window opening and [`SETTLE`] for a field
    /// being typed into. Either way the read itself is on the
    /// background executor: it blocks, and on macOS the first one after
    /// a rebuild puts a permission dialog on screen — neither of which
    /// a frame can wait for.
    ///
    /// A provider that sends no key is not asked about one. The state
    /// goes back to `Unknown`, which is what the pane renders as "this
    /// provider sends no key" rather than as an absence.
    pub fn look_up_key(&mut self, after: Duration, cx: &mut Context<Self>) {
        self.key_lookups += 1;
        self.key = KeyState::Unknown;
        cx.notify();

        if !self.engine.provider.takes_a_key() {
            return;
        }

        let mine = self.key_lookups;
        let vault = self.vault.clone();
        let account = engine::account_of(&self.engine.base_url);
        cx.spawn(async move |preferences, cx| {
            if after > Duration::ZERO {
                cx.background_executor().timer(after).await;
                // Another keystroke landed while this was waiting. The
                // request behind it is the one the user means.
                if preferences
                    .read_with(cx, |preferences, _| preferences.key_lookups != mine)
                    .unwrap_or(true)
                {
                    return;
                }
            }

            let found = cx
                .background_executor()
                .spawn(async move {
                    match vault.has(&account) {
                        Ok(true) => KeyState::Stored,
                        Ok(false) => KeyState::Absent,
                        Err(error) => {
                            // The error, never the account's contents:
                            // the credential store's own wording is
                            // what the user needs and all they get.
                            tracing::warn!(%error, "could not read the credential store");
                            KeyState::Failed(error.to_string())
                        }
                    }
                })
                .await;

            preferences
                .update(cx, |preferences, cx| {
                    if preferences.key_lookups == mine {
                        preferences.key = found;
                        cx.notify();
                    }
                })
                .ok();
        })
        .detach();
    }

    /// Put a key in the credential store, under the endpoint it was
    /// entered for.
    ///
    /// The one method in this type that takes a [`Secret`], and it does
    /// not keep it: the value goes to the background executor, is
    /// written, and is dropped there. What comes back is a
    /// [`KeyState`] — whether there is one, never what it is.
    pub fn store_key(&mut self, secret: Secret, cx: &Context<Self>) {
        self.key_lookups += 1;
        let mine = self.key_lookups;
        let vault = self.vault.clone();
        let account = engine::account_of(&self.engine.base_url);

        cx.spawn(async move |preferences, cx| {
            let outcome = cx
                .background_executor()
                .spawn(async move {
                    match vault.set(&account, &secret) {
                        Ok(()) => KeyState::Stored,
                        Err(error) => {
                            tracing::warn!(%error, "could not store a credential");
                            KeyState::Failed(error.to_string())
                        }
                    }
                })
                .await;
            preferences
                .update(cx, |preferences, cx| {
                    if outcome == KeyState::Stored {
                        preferences.key_saves += 1;
                        cx.notify();
                    }
                    if preferences.key_lookups == mine {
                        preferences.key = outcome;
                        cx.notify();
                    }
                })
                .ok();
        })
        .detach();
    }

    /// Forget this endpoint's key. Only this endpoint's — the account
    /// is the origin, so a key for another provider is untouched.
    pub fn forget_key(&mut self, cx: &Context<Self>) {
        self.key_lookups += 1;
        let mine = self.key_lookups;
        let vault = self.vault.clone();
        let account = engine::account_of(&self.engine.base_url);

        cx.spawn(async move |preferences, cx| {
            let outcome = cx
                .background_executor()
                .spawn(async move {
                    match vault.delete(&account) {
                        Ok(_) => KeyState::Absent,
                        Err(error) => {
                            tracing::warn!(%error, "could not remove a credential");
                            KeyState::Failed(error.to_string())
                        }
                    }
                })
                .await;
            preferences
                .update(cx, |preferences, cx| {
                    if outcome == KeyState::Absent {
                        preferences.key_saves += 1;
                        cx.notify();
                    }
                    if preferences.key_lookups == mine {
                        preferences.key = outcome;
                        cx.notify();
                    }
                })
                .ok();
        })
        .detach();
    }

    /// Take the server to wherever the endpoint now says, once the
    /// typing has stopped.
    ///
    /// Debounced rather than immediate, and the reason is the port
    /// field: every keystroke that leaves a bindable number behind is a
    /// preference change, so typing `10240` asks for `1024` on the way
    /// past. Binding each of those in turn would be four servers
    /// nobody asked for and a fifth that is the one they meant.
    fn restart_soon(&mut self, cx: &Context<Self>) {
        self.restarts += 1;
        if !self.serving {
            return;
        }
        self.status = Status::Starting;

        let mine = self.restarts;
        cx.spawn(async move |preferences, cx| {
            cx.background_executor().timer(SETTLE).await;
            preferences
                .update(cx, |preferences, cx| {
                    // Another keystroke landed while this was waiting,
                    // or the switch went off. Either way this request
                    // is stale and the one behind it is not.
                    if preferences.restarts != mine || !preferences.serving {
                        return;
                    }
                    preferences.supervisor.serve(preferences.endpoint);
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    /// Take what the server said and show it.
    ///
    /// The one place [`Status`] is written from the server's side. The
    /// switch writes the *request*; this writes what came of it.
    fn server_said(&mut self, event: server::Event, cx: &mut Context<Self>) {
        self.status = match event {
            server::Event::Listening { endpoint, wanted } => Status::Listening { endpoint, wanted },
            server::Event::Stopped => Status::Off,
            server::Event::Failed(reason) => Status::Failed(reason),
        };
        cx.notify();
    }

    /// Write one preference down, off the foreground thread.
    ///
    /// Every writer in this type goes through here, and they all make
    /// the same bargain: the window already shows the change, so losing
    /// the persistence is a next-launch problem rather than a reason to
    /// undo what the user just saw happen. `Store` is `Send + Sync` and
    /// the closure owns its value, so nothing about clicking a control
    /// waits on SQLite.
    fn persist(
        &self,
        cx: &Context<Self>,
        write: impl FnOnce(&wipemark_store::Store) -> anyhow::Result<()> + Send + 'static,
    ) {
        let store = self.store.clone();
        cx.background_executor()
            .spawn(async move {
                if let Err(error) = write(&store) {
                    tracing::warn!(%error, "could not persist a preference");
                }
            })
            .detach();
    }
}

/// Whether a ggml backend other than the CPU registered.
///
/// Blocks — it registers the backends, which loads every one of their
/// libraries — so it runs with the scan on the background executor. A
/// build without the local engine has no backends to ask about.
#[cfg_attr(
    feature = "local-llama",
    allow(
        clippy::unnecessary_wraps,
        reason = "one signature for both builds; the one without the local engine has no answer"
    )
)]
fn gpu_backend() -> Option<bool> {
    #[cfg(feature = "local-llama")]
    {
        Some(wipemark_engine::has_gpu_backend())
    }
    #[cfg(not(feature = "local-llama"))]
    {
        None
    }
}

/// The one dialog a Settings window can have open.
///
/// Two variants rather than a boxed `AnyView` so that closing one is a
/// `None` and not a downcast, and so a third dialog is a compile error
/// in `SettingsView::dialog` rather than an overlay nobody renders.
enum Overlay {
    Naming(Entity<Naming>),
    Confirm(Entity<Confirm>),
}

/// Everything writing the geometry down needs, owned.
type Remembered = (SettingsStore, Screen, Bounds<Pixels>);

/// A window being dragged across a display's grid of zones.
///
/// It carries nothing, and that is the design: where it lands decides
/// everything, and the cell that catches it already knows which display
/// it belongs to. What the type is *for* is the filter — `on_drop` and
/// `drag_over` are keyed by type, so a cell lights up for this and for
/// nothing else that may be dragged across this window later.
#[derive(Debug, Clone, Copy)]
struct DraggedWindow;

/// What the pointer carries while [`DraggedWindow`] is in the air.
///
/// A window has to look like a window while it is being moved, or the
/// gesture reads as dragging a coloured square onto a grid of squares.
struct WindowChip;

impl Render for WindowChip {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        v_flex()
            .w(px(64.0))
            .h(px(44.0))
            .rounded_sm()
            .border_1()
            .border_color(theme.border)
            .bg(theme.background)
            .child(
                // The titlebar, which is the whole reason this reads as
                // a window at four times the size of a favicon.
                div().h(px(8.0)).w_full().bg(theme.primary).rounded_t_sm(),
            )
    }
}

/// The view inside the Settings window: the sections, the rows, and the
/// widgets that change them.
struct SettingsView {
    preferences: Entity<Preferences>,
    /// Where the keyboard lands.
    ///
    /// Without one, `Window::focused` stays `None` for the life of the
    /// window and every key event is dispatched to the root node alone:
    /// Tab moves nothing, `⌘W` reaches nothing, and the dropdown cannot
    /// be opened without a mouse. macOS agrees — with no focus handle
    /// the accessibility tree reports the *window* as the focused
    /// element, which is what "the window has no focus" looks like from
    /// the outside.
    focus: FocusHandle,
    /// A copy, not a borrow: the geometry is written from a closure
    /// that runs while this window is being torn down, and from one
    /// that runs at process exit. Neither has an entity to read.
    store: SettingsStore,
    /// The language the widgets were last built in. `Preferences`
    /// notifies for a theme change too, and rebuilding the dropdown on
    /// every click of Light/Dark would be work for nothing.
    language: LanguagePreference,
    /// The dropdown's own state. A `Select` is a view over one of these
    /// rather than a value, so the entity is the widget — and it is
    /// bound to this window, which is why it is built here and not in
    /// [`Preferences`].
    language_select: Entity<SelectState<Vec<LanguageChoice>>>,
    /// The shortcut field for each action, in `Action::ALL` order.
    /// Entities for the reason the dropdowns are: each holds a focus
    /// handle and, while it listens, the keyboard, and both are bound
    /// to this window.
    shortcuts: BTreeMap<hotkey::Action, Entity<Recorder>>,
    /// The address field's state, bound to this window.
    ///
    /// A field and not a choice of two, because the address is now
    /// anything this machine holds. The two presets beside it write
    /// into this — they are shortcuts to a value, not a separate
    /// control with a separate opinion.
    bind: Entity<InputState>,
    /// The port field's state, likewise bound to this window.
    port: Entity<InputState>,
    /// The saved configurations. Its rows are names the user typed, so
    /// unlike every other dropdown here it is *not* rebuilt on a
    /// language change — there is nothing in it to translate.
    profile_select: Entity<SelectState<Vec<Choice<String>>>>,
    /// The ids currently in [`profile_select`], so the observer can tell
    /// a changed list from a repaint. `set_items` closes an open
    /// dropdown, which is a rude thing to do to whoever is choosing from
    /// it.
    ///
    /// [`profile_select`]: SettingsView::profile_select
    listed: Vec<String>,
    /// Which id is ticked in it, guarded for the same reason.
    ticked: Option<String>,
    /// The dialog on top of the page, if one is open.
    ///
    /// One at a time and owned here, not by `Root`: it is an element in
    /// this view's own tree, painted over the section. See
    /// `crate::dialog` on why that is the arrangement rather than
    /// gpui-component's dialog layer.
    overlay: Option<Overlay>,
    /// The open dialog's event subscription. Held rather than detached,
    /// and cleared with the overlay: a subscription that outlived its
    /// dialog would answer for the next one.
    answered: Option<Subscription>,
    /// The number of profile applications this view has caught up with.
    ///
    /// The fields below are the *source* of the engine settings, so an
    /// apply has to push its values into them — and it must not do that
    /// on any other frame, or it takes the caret out of whatever is
    /// being typed. See [`Preferences::applied`].
    applied: u64,
    /// Which shape of request Layer B sends. A dropdown rather than a
    /// button group because two of the three labels are sentences
    /// rather than words, and the control column is 240 px wide.
    serves_select: Entity<SelectState<Vec<Choice<Serves>>>>,
    provider_select: Entity<SelectState<Vec<Choice<Provider>>>>,
    /// Which downloaded model rewrites. Its rows are the models on this
    /// machine, so they change as downloads finish — [`offered`] is what
    /// says when the list has actually moved.
    ///
    /// [`offered`]: SettingsView::offered
    model_select: Entity<SelectState<Vec<Choice<Option<String>>>>>,
    /// The ids currently in `model_select`, so the observer can tell a
    /// changed list from a repaint. Rebuilding a dropdown every frame
    /// closes it under the pointer of whoever is using it.
    offered: Vec<String>,
    /// The models folder, as a path somebody can paste into or read
    /// out of. Committed on Enter and on blur — see the subscription.
    folder: Entity<InputState>,
    /// The folder the field was last set *to* by this window, so the
    /// observer can tell "the preference moved" from "the user is
    /// typing" and only overwrite the field for the first.
    folder_shown: PathBuf,
    /// The endpoint field. Free text with presets under it, the shape
    /// the bind address already uses and for the same reason: the list
    /// of endpoints somebody has is not one this product can close.
    endpoint: Entity<InputState>,
    /// The model name, exactly as the endpoint spells it.
    model: Entity<InputState>,
    /// The API key field. Masked, and **write-only**: a key that has
    /// been saved is never put back into it. See [`SettingsView::key_control`].
    api_key: Entity<InputState>,
    temperature: Entity<InputState>,
    reasoning_select: Entity<SelectState<Vec<Choice<ReasoningEffort>>>>,
    timeout: Entity<InputState>,
    /// The results folder, the shape the models folder is: a path
    /// somebody can paste into or read out of, committed on Enter and
    /// on blur.
    results_folder: Entity<InputState>,
    /// The folder that field was last set *to* by this window — see
    /// [`folder_shown`](SettingsView::folder_shown).
    results_shown: PathBuf,
    /// How long a kept copy stays. A dropdown of five fixed spans.
    period_select: Entity<SelectState<Vec<Choice<Period>>>>,
    /// How many idle minutes unload an on-demand model. Five fixed spans.
    idle_select: Entity<SelectState<Vec<Choice<u32>>>>,
    /// The engine host, for the Engine page's local-model block — `None`
    /// only where nothing installed one.
    host: Option<Entity<EngineHost>>,
    /// Which section the sidebar has selected. View state and not a
    /// preference: it is where the user is, not what they chose, and a
    /// window that reopens on the page you last visited rather than the
    /// one you asked for is the surprise this avoids.
    section: Section,
    /// Which client's snippet is on screen. View state for the same
    /// reason.
    client: Client,
    /// Whether the snippet on screen has just been copied, which the
    /// button says for two seconds and then stops saying.
    copied: bool,
    /// The screen this window is on *now*, which is not necessarily the
    /// one it opened on: dragging it to the other monitor moves where
    /// it will be remembered, which is the behaviour the per-screen
    /// rows exist to give.
    screen: Option<Screen>,
    /// The latest rectangle, kept so that closing can write it without
    /// asking the platform for a window that is on its way out.
    bounds: Bounds<Pixels>,
    _selection: Subscription,
    _shortcuts: Vec<Subscription>,
    _bind: Subscription,
    _port: Subscription,
    _profile_choice: Subscription,
    _serves: Subscription,
    _provider: Subscription,
    _model_choice: Subscription,
    _folder: Subscription,
    _endpoint: Subscription,
    _model: Subscription,
    _api_key: Subscription,
    _temperature: Subscription,
    _reasoning: Subscription,
    _timeout: Subscription,
    _results_folder: Subscription,
    _period: Subscription,
    _idle: Subscription,
    /// Repaints the page when the model loads, unloads or is checked.
    _host: Option<Subscription>,
    _preferences: Subscription,
    _geometry: Subscription,
    _activation: Subscription,
    _quit: Subscription,
}

impl SettingsView {
    fn new(
        preferences: Entity<Preferences>,
        screen: Option<Screen>,
        at: Option<Section>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (store, language, endpoint, engine, profiles, standing, serves) = {
            let preferences = preferences.read(cx);
            (
                preferences.store(),
                preferences.language.clone(),
                preferences.endpoint,
                preferences.engine.clone(),
                preferences.profiles().to_vec(),
                preferences.standing(),
                preferences.serves,
            )
        };

        // Focused before the element that tracks it has ever been
        // painted, which is the established GPUI order — `Root` does the
        // same thing when it opens a dialog. The handle exists as soon
        // as it is made; the dispatch tree picks it up on the next
        // frame.
        let focus = cx.focus_handle();
        focus.focus(window, cx);

        // The safety net for a window that comes back with its focus
        // nowhere, and the `is_none` is the whole of it.
        //
        // Measured rather than assumed, because it decides which way
        // round the bug is: GPUI clears `Window::focus` only in
        // `blur()`, never on deactivation, and logging the handle
        // across a trip to another application confirms it — the port
        // field is still the focused element when the window comes
        // back. So an *unconditional* re-focus here would take the
        // caret out of a half-typed port every time the user alt-tabbed
        // to read the number off something else. The condition is what
        // makes this a net rather than a thief.
        let activation = cx.observe_window_activation(window, |view, window, cx| {
            if window.is_window_active() && window.focused(cx).is_none() {
                view.focus.focus(window, cx);
            }
        });

        let language_select = cx.new(|cx| {
            let choices = language::choices();
            // `None` when the config names a language this build has no
            // catalogue for. Nothing is ticked, which is the honest
            // answer: none of these rows is what was asked for, and
            // moving the user onto one would overwrite a choice that a
            // later catalogue makes come true.
            let row = language::row_of(&choices, &language).map(IndexPath::new);
            SelectState::new(choices, row, window, cx)
        });

        let selection = cx.subscribe_in(
            &language_select,
            window,
            |view, _, event: &SelectEvent<Vec<LanguageChoice>>, _, cx| {
                let SelectEvent::Confirm(value) = event;
                let Some(value) = value else {
                    return;
                };
                // Back through the row's own value, so the dropdown can
                // only ever ask for a language it actually offered.
                if let Some(choice) = language::from_value(&language::choices(), value) {
                    view.preferences.update(cx, |preferences, cx| {
                        preferences.select_language(choice, cx);
                    });
                }
            },
        );

        // One field per action, built from the same three lines: a row
        // added to `Action::ALL` gets a working recorder without a
        // second place to remember.
        let mut shortcuts = BTreeMap::new();
        let mut recorded = Vec::new();
        for action in hotkey::Action::ALL {
            let chord = preferences.read(cx).shortcut(action);
            let recorder = cx.new(|cx| Recorder::new(chord, window, cx));
            recorded.push(cx.subscribe_in(
                &recorder,
                window,
                move |view, recorder, event: &RecorderEvent, _, cx| {
                    let RecorderEvent::Changed(chord) = *event;
                    let previous = view.preferences.read(cx).shortcut(action);
                    let refused = view.preferences.update(cx, |preferences, cx| {
                        preferences.assign_shortcut(action, chord, cx)
                    });
                    // The field showed the chord as it was recorded; a
                    // refusal puts the old one back and says who holds
                    // it.
                    if let Err(holder) = refused {
                        recorder.update(cx, |recorder, cx| recorder.refuse(holder, previous, cx));
                    }
                },
            ));
            shortcuts.insert(action, recorder);
        }

        let bind = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(endpoint.bind.to_string())
                // The same bargain the port field makes below:
                // `192.168.1.` is on the way to an address and a field
                // that refused it could not be typed into.
                .validate(|typed, _| mcp::typeable_address(typed))
        });

        let addressed = cx.subscribe_in(&bind, window, |view, bind, event: &InputEvent, _, cx| {
            if !matches!(event, InputEvent::Change) {
                return;
            }
            // A half-typed address is not a choice yet. The stored one
            // is left alone until the field says something that is an
            // address, so backspacing through an octet never binds a
            // nonsense interface on the way past.
            if let Some(chosen) = BindAddress::parse(&bind.read(cx).value()) {
                view.preferences.update(cx, |preferences, cx| {
                    preferences.select_bind(chosen, cx);
                });
            }
        });

        let port = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(endpoint.port.to_string())
                // What can be *typed*, which is not what is valid:
                // "50" is on the way to "5056" and a field that refused
                // it could not be typed into at all. What is valid is
                // decided below, once there is a whole number to judge.
                // The two are kept in step by
                // `every_port_can_be_reached_one_keystroke_at_a_time`.
                .validate(|typed, _| mcp::typeable(typed))
        });

        let typed = cx.subscribe_in(&port, window, |view, port, event: &InputEvent, _, cx| {
            if !matches!(event, InputEvent::Change) {
                return;
            }
            let typed = port.read(cx).value();
            // A half-typed port — "50", or nothing at all — is not a
            // choice yet, and `mcp::port` says so. The stored port is
            // left where it was until the field says something this
            // build could actually bind, so backspacing through a
            // number never writes a nonsense one on the way past.
            if let Some(chosen) = mcp::port(&typed) {
                view.preferences.update(cx, |preferences, cx| {
                    preferences.select_port(chosen, cx);
                });
            }
        });

        // Ticked from the standing rather than from the stored pointer,
        // so a page that is sitting on a profile's settings without ever
        // having applied one still opens with that profile named — and
        // one that is sitting on nothing opens blank rather than on a
        // name that is no longer true.
        let ticked = standing.id().map(str::to_owned);

        let profile_select = cx.new(|cx| {
            let choices = profile::choices(&profiles);
            let row = ticked
                .as_ref()
                .and_then(|id| engine::row_of(&choices, id))
                .map(IndexPath::new);
            SelectState::new(choices, row, window, cx)
        });

        let chose_profile = cx.subscribe_in(
            &profile_select,
            window,
            |view, _, event: &SelectEvent<Vec<Choice<String>>>, _, cx| {
                let SelectEvent::Confirm(value) = event;
                let Some(value) = value else {
                    return;
                };
                let choices = profile::choices(view.preferences.read(cx).profiles());
                // Back through the row's own value, so the dropdown can
                // only ask for a profile it actually offered.
                if let Some(id) = engine::from_value(&choices, value) {
                    view.preferences.update(cx, |preferences, cx| {
                        preferences.apply_profile(&id, cx);
                    });
                }
            },
        );

        let serves_select = cx.new(|cx| {
            let choices = duty::serves_choices();
            let row = engine::row_of(&choices, &serves).map(IndexPath::new);
            SelectState::new(choices, row, window, cx)
        });

        let chose_serves = cx.subscribe_in(
            &serves_select,
            window,
            |view, _, event: &SelectEvent<Vec<Choice<Serves>>>, _, cx| {
                let SelectEvent::Confirm(value) = event;
                let Some(value) = value else {
                    return;
                };
                if let Some(serves) = engine::from_value(&duty::serves_choices(), value) {
                    view.preferences.update(cx, |preferences, cx| {
                        preferences.select_serves(serves, cx);
                    });
                }
            },
        );

        let provider_select = cx.new(|cx| {
            let choices = engine::provider_choices();
            let row = engine::row_of(&choices, &engine.provider).map(IndexPath::new);
            SelectState::new(choices, row, window, cx)
        });

        let chose_provider = cx.subscribe_in(
            &provider_select,
            window,
            |view, _, event: &SelectEvent<Vec<Choice<Provider>>>, _, cx| {
                let SelectEvent::Confirm(value) = event;
                let Some(value) = value else {
                    return;
                };
                // Back through the row's own value, so the dropdown can
                // only ask for a provider it actually offered.
                if let Some(provider) = engine::from_value(&engine::provider_choices(), value) {
                    view.preferences.update(cx, |preferences, cx| {
                        preferences.select_provider(provider, cx);
                    });
                }
            },
        );

        let model_select = cx.new(|cx| {
            // Empty until the first scan answers. The page says it is
            // still looking rather than that nothing is installed.
            SelectState::new(models::model_choices(Vec::new()), None, window, cx)
        });

        let chose_model = cx.subscribe_in(
            &model_select,
            window,
            |view, _, event: &SelectEvent<Vec<Choice<Option<String>>>>, _, cx| {
                let SelectEvent::Confirm(value) = event;
                let Some(value) = value else {
                    return;
                };
                let installed = view.preferences.read(cx).installed_for(Role::Rewrite);
                let choices = models::model_choices(installed);
                // Back through the row's own value, so the dropdown can
                // only ask for a model it actually offered — which is
                // to say, one that is on this machine.
                if let Some(chosen) = engine::from_value(&choices, value) {
                    view.preferences.update(cx, |preferences, cx| {
                        preferences.select_rewrite_model(chosen, cx);
                    });
                }
            },
        );

        // The folder field shows the folder in effect — the platform's
        // when no row names one — so "Default" has something to show
        // and an empty field is an instruction rather than a state.
        let folder_shown = preferences.read(cx).models_dir().to_path_buf();
        let folder_field = cx.new(|cx| {
            InputState::new(window, cx).default_value(folder_shown.display().to_string())
        });

        // Committed on Enter and on leaving the field, and **not** on
        // change, unlike every other field here. A folder is read
        // recursively the moment it is chosen, and `/Users/` is an
        // absolute path on the way to `/Users/me/models`: committing per
        // keystroke would walk a home directory once per letter typed.
        let typed_folder = cx.subscribe_in(
            &folder_field,
            window,
            |view, field, event: &InputEvent, window, cx| {
                if !matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                    return;
                }
                view.commit_folder(field.read(cx).value().to_string(), window, cx);
            },
        );

        let endpoint_field = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(engine.base_url.to_string())
                // `https://ap` is on the way to an endpoint. What is a
                // *valid* endpoint is decided below, once there is a
                // whole one — the same pair the port and the address
                // fields keep.
                .validate(|typed, _| engine::typeable_url(typed))
        });

        let typed_endpoint = cx.subscribe_in(
            &endpoint_field,
            window,
            |view, field, event: &InputEvent, _, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                // A half-typed URL is not a choice yet, and neither is
                // one carrying a user name and a password — that one is
                // refused rather than half-accepted, because the stored
                // endpoint is the one place a credential must not land.
                if let Some(chosen) = BaseUrl::parse(&field.read(cx).value()) {
                    view.preferences.update(cx, |preferences, cx| {
                        preferences.select_endpoint(chosen, cx);
                    });
                }
            },
        );

        let model_field = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(engine.model.clone())
                .validate(|typed, _| engine::typeable_model(typed))
        });

        let typed_model = cx.subscribe_in(
            &model_field,
            window,
            |view, field, event: &InputEvent, _, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                // Unlike every other field here, an empty one *is* a
                // choice: it is how a user says they have not picked a
                // model, and the banner says so rather than silently
                // keeping the last name they typed.
                let typed = field.read(cx).value();
                let chosen = engine::model(&typed).unwrap_or_default();
                view.preferences.update(cx, |preferences, cx| {
                    preferences.select_model(chosen, cx);
                });
            },
        );

        // No `default_value`. A key that has been saved is never read
        // back out of the credential store into this field — see
        // `key_control` on why the control is write-only.
        let api_key = cx.new(|cx| {
            InputState::new(window, cx)
                .masked(true)
                .placeholder(t(Message::SettingsEngineKeyPlaceholder))
        });

        // The one subscription here that changes no preference. Save is
        // disabled while the field is empty, which makes it the only
        // control on this page whose appearance depends on what is *in*
        // a field rather than on what has been chosen — and nothing
        // else in this window would repaint for that. Writing the key
        // on change instead is what this deliberately does not do: the
        // credential store blocks and can prompt, and a key typed a
        // character at a time would be forty writes.
        let typed_key = cx.subscribe_in(&api_key, window, |_, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });

        let temperature = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(engine.temperature.to_string())
                .validate(|typed, _| engine::typeable_temperature(typed))
        });

        let typed_temperature = cx.subscribe_in(
            &temperature,
            window,
            |view, field, event: &InputEvent, _, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                if let Some(chosen) = engine::temperature(&field.read(cx).value()) {
                    view.preferences.update(cx, |preferences, cx| {
                        preferences.select_temperature(chosen, cx);
                    });
                }
            },
        );

        let reasoning_select = cx.new(|cx| {
            let choices = engine::reasoning_choices();
            let row = engine::row_of(&choices, &engine.reasoning).map(IndexPath::new);
            SelectState::new(choices, row, window, cx)
        });

        let chose_reasoning = cx.subscribe_in(
            &reasoning_select,
            window,
            |view, _, event: &SelectEvent<Vec<Choice<ReasoningEffort>>>, _, cx| {
                let SelectEvent::Confirm(value) = event;
                let Some(value) = value else {
                    return;
                };
                if let Some(effort) = engine::from_value(&engine::reasoning_choices(), value) {
                    view.preferences.update(cx, |preferences, cx| {
                        preferences.select_reasoning(effort, cx);
                    });
                }
            },
        );

        let timeout = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(engine.timeout.to_string())
                .validate(|typed, _| engine::typeable_timeout(typed))
        });

        let typed_timeout = cx.subscribe_in(
            &timeout,
            window,
            |view, field, event: &InputEvent, _, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                if let Some(chosen) = engine::timeout(&field.read(cx).value()) {
                    view.preferences.update(cx, |preferences, cx| {
                        preferences.select_timeout(chosen, cx);
                    });
                }
            },
        );

        // The results folder field, built and committed the way the
        // models folder is — on Enter and on blur, never on change —
        // and for the same reason: `/Users/` is an absolute path on the
        // way to `/Users/me/cleaned`, and a folder half-typed is not a
        // folder chosen.
        let results_shown = preferences.read(cx).results_folder().to_path_buf();
        let results_field = cx.new(|cx| {
            InputState::new(window, cx).default_value(results_shown.display().to_string())
        });

        let typed_results = cx.subscribe_in(
            &results_field,
            window,
            |view, field, event: &InputEvent, window, cx| {
                if !matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                    return;
                }
                view.commit_results_folder(field.read(cx).value().to_string(), window, cx);
            },
        );

        let period_select = cx.new(|cx| {
            let choices = retention::period_choices();
            let period = preferences.read(cx).retention().keep_for;
            let row = engine::row_of(&choices, &period).map(IndexPath::new);
            SelectState::new(choices, row, window, cx)
        });

        let chose_period = cx.subscribe_in(
            &period_select,
            window,
            |view, _, event: &SelectEvent<Vec<Choice<Period>>>, _, cx| {
                let SelectEvent::Confirm(value) = event;
                let Some(value) = value else {
                    return;
                };
                if let Some(period) = engine::from_value(&retention::period_choices(), value) {
                    view.preferences.update(cx, |preferences, cx| {
                        preferences.select_period(period, cx);
                    });
                }
            },
        );

        let idle_select = cx.new(|cx| {
            let choices = idle_choices();
            let minutes = preferences.read(cx).local_policy().idle_minutes;
            let row = engine::row_of(&choices, &minutes).map(IndexPath::new);
            SelectState::new(choices, row, window, cx)
        });

        let chose_idle = cx.subscribe_in(
            &idle_select,
            window,
            |view, _, event: &SelectEvent<Vec<Choice<u32>>>, _, cx| {
                let SelectEvent::Confirm(value) = event;
                let Some(value) = value else {
                    return;
                };
                if let Some(minutes) = engine::from_value(&idle_choices(), value) {
                    view.preferences.update(cx, |preferences, cx| {
                        preferences.select_idle(minutes, cx);
                    });
                }
            },
        );

        let host = engine_host::hosted(cx);
        let watched_host = host
            .as_ref()
            .map(|host| cx.observe(host, |_, _, cx| cx.notify()));

        // The first moment anything needs to know whether a key is
        // stored, and the reason `Preferences::new` deliberately did
        // not ask: the read blocks and, for an unsigned build, prompts.
        // A permission dialog belongs over a window somebody just
        // opened, not over one that has not been drawn yet.
        preferences.update(cx, |preferences, cx| {
            preferences.look_up_key(Duration::ZERO, cx);
            // And the same bargain for the models directory: the probe
            // can spawn the graphics driver's own tool and the scan can
            // re-hash gigabytes, so it happens when the window that
            // shows the answer opens rather than at startup.
            preferences.look_at_models(cx);
        });

        // A change made anywhere — the menu bar's Appearance submenu,
        // an OS appearance flip — has to move the controls in here too.
        // The button groups and the switch are rebuilt every frame and
        // need only the notify; the dropdown holds state and is rebuilt
        // only when the thing it shows has actually moved. The port
        // field is deliberately left alone: it is the one control whose
        // value the user is *mid-way through* editing.
        let observed = cx.observe_in(&preferences, window, |view, preferences, window, cx| {
            let language = preferences.read(cx).language.clone();
            if view.language != language {
                view.language = language;
                view.retranslate(window, cx);
            }
            // An apply changed seven values at once, and five of them
            // live in fields this view owns. Caught here rather than in
            // the click handler because a profile can be applied from
            // anywhere that holds the entity.
            let applied = preferences.read(cx).applied();
            if view.applied != applied {
                view.applied = applied;
                view.apply_engine_fields(window, cx);
            }
            // A recorder is the source of its own value while it is
            // being used; this only matters when the row moved from
            // somewhere else, and `show` is a no-op otherwise.
            for (action, recorder) in &view.shortcuts {
                let chord = preferences.read(cx).shortcut(*action);
                recorder.update(cx, |recorder, cx| recorder.show(chord, cx));
            }
            view.refresh_profile_choices(window, cx);
            view.refresh_model_choices(window, cx);
            view.show_the_folder(window, cx);
            view.show_the_results_folder(window, cx);
            cx.notify();
        });

        let geometry = cx.observe_window_bounds(window, |view, window, cx| {
            view.bounds = geometry_of(window);
            // Recomputed rather than kept: a window dragged onto the
            // other monitor is remembered *there* next time, which is
            // the whole point of a row per screen.
            view.screen = screen::of_window(window, cx);
        });

        // The two ways this window stops existing. The close button is
        // the ordinary one; Quit with the window still open fires no
        // close at all, and that is the case that loses the position
        // every time if it is not handled separately.
        let quit = cx.on_app_quit(|view: &mut Self, _| {
            if let Some((store, screen, bounds)) = view.remembered() {
                // Synchronous, unlike every other write in this file.
                // Handing it to the background executor here would be
                // the *less* honest choice: `on_app_quit` is the last
                // thing that runs, and a detached task queued at that
                // point has nothing left to run on. There is no frame
                // to miss either — this is the process leaving.
                window_state::save(&store, &screen, bounds);
            }
            async {}
        });

        let this = cx.weak_entity();
        window.on_window_should_close(cx, move |_, cx| {
            if let Some(view) = this.upgrade() {
                view.read(cx).remember(cx);
            }
            true
        });

        Self {
            preferences,
            focus,
            store,
            language,
            language_select,
            shortcuts,
            bind,
            port,
            profile_select,
            listed: profiles.iter().map(|saved| saved.id.clone()).collect(),
            ticked,
            overlay: None,
            answered: None,
            applied: 0,
            serves_select,
            provider_select,
            model_select,
            offered: Vec::new(),
            folder: folder_field,
            folder_shown,
            endpoint: endpoint_field,
            model: model_field,
            api_key,
            temperature,
            reasoning_select,
            timeout,
            results_folder: results_field,
            results_shown,
            period_select,
            idle_select,
            host,
            section: at.unwrap_or(Section::General),
            client: Client::ClaudeCode,
            copied: false,
            screen,
            bounds: geometry_of(window),
            _selection: selection,
            _shortcuts: recorded,
            _bind: addressed,
            _port: typed,
            _profile_choice: chose_profile,
            _serves: chose_serves,
            _provider: chose_provider,
            _model_choice: chose_model,
            _folder: typed_folder,
            _endpoint: typed_endpoint,
            _model: typed_model,
            _api_key: typed_key,
            _temperature: typed_temperature,
            _reasoning: chose_reasoning,
            _timeout: typed_timeout,
            _results_folder: typed_results,
            _period: chose_period,
            _idle: chose_idle,
            _host: watched_host,
            _preferences: observed,
            _geometry: geometry,
            _activation: activation,
            _quit: quit,
        }
    }

    /// Write the geometry down, off the foreground thread.
    ///
    /// `Store` is `Send + Sync` and everything else here is owned, so
    /// nothing about closing a window waits on SQLite.
    fn remember(&self, cx: &App) {
        let Some((store, screen, bounds)) = self.remembered() else {
            return;
        };
        cx.background_executor()
            .spawn(async move { window_state::save(&store, &screen, bounds) })
            .detach();
    }

    /// Where this window is and what to file it under, owned so that a
    /// closure outliving the view can still write it.
    ///
    /// `None` for a screen the platform will not name stably — see the
    /// note on `Screen::key`.
    fn remembered(&self) -> Option<Remembered> {
        Some((self.store.clone(), self.screen.clone()?, self.bounds))
    }

    /// Put the widgets back into the language that is on screen now.
    ///
    /// The System row reads "System (English)", so the list itself is
    /// one of the things a language change moves. Rebuilt rather than
    /// left alone: it would otherwise be the one stale string in a
    /// window that had just been fully retranslated.
    fn retranslate(&self, window: &mut Window, cx: &mut Context<Self>) {
        window.set_window_title(&t(Message::SettingsTitle));

        let choices = language::choices();
        let row = language::row_of(&choices, &self.language).map(IndexPath::new);
        self.language_select.update(cx, |select, cx| {
            select.set_items(choices, window, cx);
            select.set_selected_index(row, window, cx);
        });

        // The other two dropdowns, for the same reason: every row in
        // them is a catalogue string, so a language change leaves them
        // reading the last language in a window that has otherwise
        // moved. Read before the updates rather than inside them — a
        // closure holding `cx` cannot also read an entity through it.
        let (provider, reasoning) = {
            let engine = &self.preferences.read(cx).engine;
            (engine.provider, engine.reasoning)
        };

        let serves = self.preferences.read(cx).serves;
        let choices = duty::serves_choices();
        let row = engine::row_of(&choices, &serves).map(IndexPath::new);
        self.serves_select.update(cx, |select, cx| {
            select.set_items(choices, window, cx);
            select.set_selected_index(row, window, cx);
        });

        let choices = engine::provider_choices();
        let row = engine::row_of(&choices, &provider).map(IndexPath::new);
        self.provider_select.update(cx, |select, cx| {
            select.set_items(choices, window, cx);
            select.set_selected_index(row, window, cx);
        });

        let choices = engine::reasoning_choices();
        let row = engine::row_of(&choices, &reasoning).map(IndexPath::new);
        self.reasoning_select.update(cx, |select, cx| {
            select.set_items(choices, window, cx);
            select.set_selected_index(row, window, cx);
        });

        let period = self.preferences.read(cx).retention().keep_for;
        let choices = retention::period_choices();
        let row = engine::row_of(&choices, &period).map(IndexPath::new);
        self.period_select.update(cx, |select, cx| {
            select.set_items(choices, window, cx);
            select.set_selected_index(row, window, cx);
        });

        let minutes = self.preferences.read(cx).local_policy().idle_minutes;
        let choices = idle_choices();
        let row = engine::row_of(&choices, &minutes).map(IndexPath::new);
        self.idle_select.update(cx, |select, cx| {
            select.set_items(choices, window, cx);
            select.set_selected_index(row, window, cx);
        });

        // Not a row label, but the one string inside a control that
        // would otherwise stay in the language the window opened in.
        self.api_key.update(cx, |field, cx| {
            field.set_placeholder(t(Message::SettingsEngineKeyPlaceholder), window, cx);
        });

        // The profile *dropdown* is deliberately not rebuilt here: its
        // rows are names a user typed, and there is nothing in them to
        // translate. Neither is an open dialog — it is built in the
        // language it opened in and answered in the same one.
    }

    /// The section list down the left.
    fn sections(&self, cx: &Context<Self>) -> impl IntoElement {
        let current = self.section;
        Sidebar::new("sections")
            // Nothing to collapse to. The toggle is for an application
            // window reclaiming space for a document; a preferences
            // window has one job and hiding half of it is not a
            // feature.
            .collapsible(false)
            .w(SIDEBAR_WIDTH)
            .child(SidebarMenu::new().children(Section::ALL.map(|section| {
                SidebarMenuItem::new(SharedString::from(t(section.title())))
                    // Ours, not the component's own set: `icon` takes
                    // anything that converts into a
                    // `gpui_component::Icon`, and `impl IconNamed for
                    // IconName` in `crate::icon` is what puts a
                    // promoted FA Free file on the other side of that
                    // conversion. Hand it one of gpui-component's
                    // lucide names instead and the row paints blank.
                    .icon(section.glyph())
                    .active(current == section)
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.section = section;
                        cx.notify();
                    }))
            })))
    }

    /// The rows of one section: a title with its control beside it, and
    /// the sentence underneath spanning the whole width.
    ///
    /// The sentence goes *below* rather than beside, and that is a
    /// measurement rather than a taste. The control column is fixed —
    /// it has to be, or the dropdown takes the row — so everything a
    /// narrow window costs comes out of the text next to it. Set
    /// side by side at the minimum width the descriptions here wrapped
    /// to five lines of three words, which is how a paragraph looks
    /// when it has been given a column meant for a label. Underneath,
    /// the same sentence gets the width of the section and reads in
    /// two.
    fn rows(&self, section: Section, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let border = theme.border;
        let muted = theme.muted_foreground;

        v_flex().children(section.rows().enumerate().map(|(index, setting)| {
            v_flex()
                .gap_1()
                .py_4()
                // Between the rows and never above the first one: the
                // heading above already draws that line.
                .when(index > 0, |row| row.border_t_1().border_color(border))
                .children(setting.caption().map(|caption| {
                    div()
                        .text_xs()
                        .font_semibold()
                        .text_color(muted)
                        .child(SharedString::from(t(caption)))
                }))
                .child(
                    h_flex()
                        .gap_6()
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                // Without this a long title refuses to
                                // wrap and pushes the control off the
                                // window instead.
                                .min_w(px(0.0))
                                .text_sm()
                                .font_medium()
                                .child(SharedString::from(t(setting.title()))),
                        )
                        // A fixed column rather than "whatever the
                        // widget wants". The dropdown asks for the full
                        // width of whatever it is put in, and against a
                        // `flex_1` label it wins: the title collapses to
                        // one letter per line and the window grows to
                        // fit it.
                        .child(
                            div()
                                .flex()
                                .flex_none()
                                .w(CONTROL_COLUMN)
                                .justify_end()
                                .child(self.control(setting, cx)),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .child(SharedString::from(t(setting.description()))),
                )
        }))
    }

    /// The widget on the right of one row.
    ///
    /// Exhaustive on purpose — see the note on [`Setting`].
    fn control(&self, setting: Setting, cx: &Context<Self>) -> AnyElement {
        match setting {
            Setting::Appearance => self.theme_selector(cx).into_any_element(),
            Setting::Language => self.language_selector().into_any_element(),
            Setting::ShortcutShow => self
                .shortcut_control(hotkey::Action::Show, cx)
                .into_any_element(),
            Setting::ShortcutPanel => self
                .shortcut_control(hotkey::Action::Panel, cx)
                .into_any_element(),
            Setting::Setup => Self::setup_control(cx).into_any_element(),
            Setting::WindowScreen => self.screen_choice(cx).into_any_element(),
            Setting::CloseAfterDrop => self.close_switch(cx).into_any_element(),
            Setting::CompareGrain => self.grain_choice(cx).into_any_element(),
            Setting::CompareFollow => self.follow_switch(cx).into_any_element(),
            Setting::EngineServes => self.serves_selector().into_any_element(),
            Setting::EngineKeep => self.keep_choice(cx).into_any_element(),
            Setting::EngineIdle => self.idle_selector(cx).into_any_element(),
            Setting::EngineLock => self.lock_switch(cx).into_any_element(),
            Setting::EngineProfile => self.profile_control(cx).into_any_element(),
            Setting::EngineProvider => self.provider_selector().into_any_element(),
            Setting::EngineEndpoint => self.endpoint_control(cx).into_any_element(),
            Setting::EngineModel => Input::new(&self.model).small().into_any_element(),
            Setting::EngineKey => self.key_control(cx).into_any_element(),
            Setting::EngineAllowRemote => self.allow_remote_switch(cx).into_any_element(),
            Setting::EngineTemperature => number_field(&self.temperature).into_any_element(),
            Setting::EngineReasoning => self.reasoning_selector().into_any_element(),
            Setting::EngineTimeout => number_field(&self.timeout).into_any_element(),
            Setting::ModelsFolder => self.folder_control(cx).into_any_element(),
            Setting::ModelForRewrite => self.model_selector().into_any_element(),
            Setting::ResultsDestination => self.destination_choice(cx).into_any_element(),
            Setting::ResultsFolder => self.results_folder_control(cx).into_any_element(),
            Setting::KeepOriginals => self.keep_originals_switch(cx).into_any_element(),
            Setting::KeepResults => self.keep_results_switch(cx).into_any_element(),
            Setting::KeepFor => self.period_selector().into_any_element(),
            Setting::ServeOverMcp => self.serve_switch(cx).into_any_element(),
            Setting::McpBind => self.bind_control(cx).into_any_element(),
            Setting::McpPort => self.port_field().into_any_element(),
        }
    }

    /// System / Light / Dark, in `ThemePreference::ALL` order — the
    /// click handler indexes back into it.
    fn theme_selector(&self, cx: &Context<Self>) -> impl IntoElement {
        let current = self.preferences.read(cx).theme;
        ButtonGroup::new("theme-preference")
            .small()
            .outline()
            .children(ThemePreference::ALL.map(|choice| {
                Button::new(choice.as_str())
                    .icon(choice.glyph())
                    .label(choice.label())
                    .selected(current == choice)
            }))
            .on_click(cx.listener(|view, clicked: &Vec<usize>, window, cx| {
                if let Some(choice) = ThemePreference::from_selection(clicked) {
                    view.preferences.update(cx, |preferences, cx| {
                        preferences.select_theme(choice, Some(window), cx);
                    });
                }
            }))
    }

    /// A dropdown rather than a button group, and every row written in
    /// its own language — see `language::choices`.
    fn language_selector(&self) -> impl IntoElement {
        Select::new(&self.language_select)
            .small()
            .menu_width(CONTROL_COLUMN)
            .placeholder(language::selector_label())
    }

    /// The shortcut recorder for `action`, with the desktop's answer
    /// under it.
    ///
    /// The recorder is the request. The line beneath is what actually
    /// happened when the desktop was asked, and it is on screen
    /// whenever there is an answer to give — for the reason the MCP
    /// page shows the port it got beside the one it asked for. A chord
    /// the OS refused is invisible from the field alone.
    fn shortcut_control(&self, action: hotkey::Action, cx: &Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let danger = cx.theme().danger;
        let recorder = self.shortcuts.get(&action).cloned();
        let answer = match self.preferences.read(cx).registration(action) {
            Registration::Unset => None,
            Registration::Registered => Some((t(Message::HotkeyRegistered), muted)),
            Registration::Refused(reason) => Some((
                t_args(Message::HotkeyRefused, &args!("reason" => reason.clone())),
                danger,
            )),
            Registration::Unavailable => Some((t(Message::HotkeyUnavailable), muted)),
        };
        v_flex()
            .gap_1()
            .w_full()
            // Always exactly one: the map is filled from `Action::ALL`
            // when the window opens. `children` rather than `child`
            // because a `get` is an `Option`, and inventing a second
            // recorder here to satisfy the type would be a field with
            // no row behind it.
            .children(recorder)
            .children(answer.map(|(text, tone)| {
                div()
                    .text_xs()
                    .text_color(tone)
                    .child(SharedString::from(text))
            }))
    }

    /// The button that reopens the setup walk-through — and, in a debug
    /// build, the one under it that forgets it was ever shown.
    ///
    /// The first asks; the main window answers. The walk-through is an
    /// overlay in that window's own tree — see `crate::setup` — and this
    /// window has no handle to it, so the request goes through
    /// [`Preferences`] as a counter the main window observes, the way a
    /// profile application reaches the fields here. Nothing is written:
    /// the row under this control is answered by Finish and Skip inside
    /// the walk-through, not by asking for it.
    ///
    /// The second is the row's only way back to absent, and it exists
    /// in a debug build alone: the first launch opens the walk-through
    /// by a path `--setup` does not take, and checking that path should
    /// not start with a database client. The label says which build it
    /// belongs to, because a control that is there in one and not the
    /// other should say so.
    fn setup_control(cx: &Context<Self>) -> impl IntoElement {
        let run = Button::new("run-setup")
            .small()
            .outline()
            .label(SharedString::from(t(Message::SettingsSetupRun)))
            .on_click(cx.listener(|view, _, _, cx| {
                view.preferences.update(cx, |preferences, cx| {
                    preferences.ask_for_setup(cx);
                });
            }));
        let column = v_flex().items_end().gap_1().child(run);
        #[cfg(debug_assertions)]
        let column = column.child(
            Button::new("forget-setup")
                .xsmall()
                .ghost()
                .label(SharedString::from(t(Message::SettingsSetupReset)))
                .on_click(cx.listener(|view, _, _, cx| {
                    view.preferences.update(cx, |preferences, cx| {
                        preferences.forget_setup(cx);
                    });
                })),
        );
        column
    }

    /// Which screen a window opens on: two radio buttons.
    ///
    /// Radio buttons and not a dropdown, which is what every other
    /// choice on this page is. Two mutually exclusive answers that both
    /// fit the control column are the case radio buttons exist for: a
    /// dropdown would hide half the question behind a click and save no
    /// space doing it.
    fn screen_choice(&self, cx: &Context<Self>) -> impl IntoElement {
        let current = self.preferences.read(cx).onto();
        RadioGroup::vertical("window-screen")
            .selected_index(Onto::ALL.iter().position(|onto| *onto == current))
            .children(Onto::ALL.map(|onto| Radio::new(onto.as_str()).label(t(onto.label()))))
            .on_click(cx.listener(|view, index: &usize, _, cx| {
                // Back through the list the buttons were built from, so
                // an index this control invents cannot become a
                // preference.
                let Some(onto) = Onto::ALL.get(*index).copied() else {
                    return;
                };
                view.preferences.update(cx, |preferences, cx| {
                    preferences.select_onto(onto, cx);
                });
            }))
    }

    /// Whether a drop gets this window out of the way.
    ///
    /// A switch and not a checkbox for the reason every other yes-or-no
    /// on this page is one: the rows of this window have a single
    /// shape, and a third kind of control in the same column would be a
    /// difference that means nothing.
    fn close_switch(&self, cx: &Context<Self>) -> impl IntoElement {
        let close = self.preferences.read(cx).closes_after_drop();
        Switch::new("close-after-drop")
            .checked(close)
            .on_click(cx.listener(|view, close: &bool, _, cx| {
                let close = *close;
                view.preferences.update(cx, |preferences, cx| {
                    preferences.close_after_drop(close, cx);
                });
            }))
    }

    /// The Compare page: how a result is shown beside its original.
    ///
    /// The notice above the rows is the one every pending surface
    /// carries: nothing is cleaned in this version yet, and the window
    /// compares what is typed against what was started from. The
    /// page's own sentence, under the heading, is the other thing a
    /// reader has to know — that a window reads these as it opens.
    fn compare(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(heading(
                Message::SettingsCompareTitle,
                Some(Message::SettingsCompareDescription),
                cx,
            ))
            .child(notice(
                IconName::CodeCompare,
                cx.theme().muted_foreground,
                vec![t(Message::ComparePending), t(Message::SettingsCompareExact)],
                cx,
            ))
            .child(self.rows(Section::Compare, cx))
    }

    /// Lines, words, characters — three radio buttons, the shape the
    /// destination choice has and for the same reason: three short
    /// answers that fit the column, and coarsest first.
    fn grain_choice(&self, cx: &Context<Self>) -> impl IntoElement {
        let current = self.preferences.read(cx).comparison().grain;
        RadioGroup::vertical("compare-grain")
            .selected_index(Grain::ALL.iter().position(|g| *g == current))
            .children(Grain::ALL.map(|grain| Radio::new(grain.id()).label(t(grain.title()))))
            .on_click(cx.listener(|view, index: &usize, _, cx| {
                let Some(grain) = Grain::ALL.get(*index).copied() else {
                    return;
                };
                view.preferences.update(cx, |preferences, cx| {
                    preferences.select_grain(grain, cx);
                });
            }))
    }

    /// The switch that keeps the original following the result's
    /// cursor.
    fn follow_switch(&self, cx: &Context<Self>) -> impl IntoElement {
        let follow = self.preferences.read(cx).comparison().follow;
        Switch::new("compare-follow")
            .checked(follow)
            .on_click(cx.listener(|view, follow: &bool, _, cx| {
                let follow = *follow;
                view.preferences.update(cx, |preferences, cx| {
                    preferences.follow_cursor(follow, cx);
                });
            }))
    }

    /// The switch that turns the MCP server on.
    fn serve_switch(&self, cx: &Context<Self>) -> impl IntoElement {
        let (serving, _) = self.preferences.read(cx).mcp();
        Switch::new("serve-over-mcp")
            .checked(serving)
            .on_click(cx.listener(|view, serving: &bool, _, cx| {
                let serving = *serving;
                view.preferences.update(cx, |preferences, cx| {
                    preferences.serve_over_mcp(serving, cx);
                });
            }))
    }

    /// A field with the two usual answers under it.
    ///
    /// This used to be a choice of two, and widening it is the point:
    /// a desktop with two interfaces, a machine answering only on its
    /// VPN address, a container published on one bridge — each of those
    /// is a specific address the user has, and a pair of buttons had no
    /// way to say any of them. lazy-shot's field is free text with the
    /// presets beside it, which is the right shape.
    ///
    /// The presets stay because they are what almost everybody means,
    /// and because `0.0.0.0` is a thing to be *offered* rather than
    /// remembered. They write into the field rather than beside it: the
    /// field is the setting, and a button that moved the preference
    /// while leaving the address on screen unchanged would be two
    /// controls disagreeing in front of the user.
    fn bind_control(&self, cx: &Context<Self>) -> impl IntoElement {
        let (_, endpoint) = self.preferences.read(cx).mcp();
        v_flex()
            .gap_1()
            .w_full()
            .child(Input::new(&self.bind).small())
            .child(
                h_flex().justify_end().child(
                    ButtonGroup::new("mcp-bind-presets")
                        .xsmall()
                        .outline()
                        .children(BindAddress::PRESETS.map(|choice| {
                            let address = SharedString::from(choice.to_string());
                            Button::new(address.clone())
                                .label(address)
                                .selected(endpoint.bind == choice)
                        }))
                        .on_click(cx.listener(|view, clicked: &Vec<usize>, window, cx| {
                            let Some(choice) = clicked
                                .first()
                                .and_then(|index| BindAddress::PRESETS.get(*index))
                                .copied()
                            else {
                                return;
                            };
                            view.bind.update(cx, |field, cx| {
                                field.set_value(choice.to_string(), window, cx);
                            });
                            view.preferences.update(cx, |preferences, cx| {
                                preferences.select_bind(choice, cx);
                            });
                        })),
                ),
            )
    }

    /// A plain field, not a stepper. A port is looked up and typed in
    /// whole; nobody arrives at 5056 by pressing `+` a thousand times.
    fn port_field(&self) -> impl IntoElement {
        div().w(px(96.0)).child(Input::new(&self.port).small())
    }

    /// The provider dropdown. Two of its three labels are sentences,
    /// which is what rules out a button group in a 240 px column.
    /// The "who rewrites" dropdown: the one control on this page that
    /// is about both halves of the product at once.
    fn serves_selector(&self) -> impl IntoElement {
        Select::new(&self.serves_select)
            .small()
            .menu_width(CONTROL_COLUMN)
    }

    fn provider_selector(&self) -> impl IntoElement {
        Select::new(&self.provider_select)
            .small()
            .menu_width(CONTROL_COLUMN)
    }

    /// The reasoning-effort dropdown. Five choices, one of which reads
    /// "Off (omit)" — a distinction that needs the room a dropdown row
    /// has and a button does not.
    fn reasoning_selector(&self) -> impl IntoElement {
        Select::new(&self.reasoning_select)
            .small()
            .menu_width(CONTROL_COLUMN)
    }

    /// The endpoint field, with the presets for the chosen provider
    /// under it.
    ///
    /// The same arrangement as the MCP address, and the same reasoning:
    /// the presets are what almost everybody means, and they write
    /// *into* the field rather than beside it — the field is the
    /// setting, and a button that moved the preference while leaving
    /// the URL on screen unchanged would be two controls disagreeing in
    /// front of the user.
    ///
    /// The list is the provider's, because the two wire formats are not
    /// interchangeable: `https://api.openai.com` under the Ollama
    /// provider reaches nothing at all.
    fn endpoint_control(&self, cx: &Context<Self>) -> impl IntoElement {
        let engine = self.preferences.read(cx).engine();
        let presets = engine.provider.presets();
        let chosen = engine.base_url.clone();

        v_flex()
            .gap_1()
            .w_full()
            .child(Input::new(&self.endpoint).small())
            .when(!presets.is_empty(), |control| {
                control.child(
                    h_flex().justify_end().child(
                        ButtonGroup::new("engine-endpoint-presets")
                            .xsmall()
                            .outline()
                            .children(presets.iter().map(|preset| {
                                Button::new(preset.url).label(preset.label).selected(
                                    BaseUrl::parse(preset.url).is_some_and(|url| url == chosen),
                                )
                            }))
                            .on_click(cx.listener(|view, clicked: &Vec<usize>, window, cx| {
                                let presets = view.preferences.read(cx).engine().provider.presets();
                                let Some(chosen) = clicked
                                    .first()
                                    .and_then(|index| presets.get(*index))
                                    .and_then(|preset| BaseUrl::parse(preset.url))
                                else {
                                    return;
                                };
                                view.endpoint.update(cx, |field, cx| {
                                    field.set_value(chosen.to_string(), window, cx);
                                });
                                view.preferences.update(cx, |preferences, cx| {
                                    preferences.select_endpoint(chosen, cx);
                                });
                            })),
                    ),
                )
            })
    }

    /// The API key: a masked field, two buttons, and a sentence saying
    /// what the credential store currently holds.
    ///
    /// Masked, with the eye in its trailing edge that every password
    /// field has — `Input::mask_toggle`, whose two glyphs are
    /// `icons/eye.svg` and `icons/eye-off.svg` and which are shipped
    /// for exactly this reason. A field that cannot be revealed is one
    /// where a mistyped or half-pasted key is found out about later, by
    /// a 401 that reads like a wrong key rather than like a wrong
    /// paste.
    ///
    /// **Write-only, and that is the one deliberate difference from a
    /// browser's password field.** A key that has been *saved* is never
    /// put back into this field, and the field empties the moment it is
    /// saved — so the eye only ever reveals what the user has just
    /// typed, never what the credential store is holding. Loading it
    /// back would put a live credential on screen during a screen
    /// share, in a screenshot attached to a bug report, and in the
    /// accessibility tree, and buy nothing: what the user needs from
    /// this control after they have used it is the answer to "is there
    /// a key", and that is the sentence underneath.
    ///
    /// Saving is a button and not a keystroke, and that is not
    /// conservatism: every other field here writes on change, but this
    /// one writes to the operating system's credential store, which
    /// blocks and can raise a permission dialog. A key typed a
    /// character at a time would be forty writes and, on a first run,
    /// forty prompts.
    fn key_control(&self, cx: &Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let danger = cx.theme().danger;
        let preferences = self.preferences.read(cx);
        let engine = preferences.engine();
        let wanted = engine.provider.takes_a_key();
        let origin = engine.base_url.origin();
        let in_the_clear = engine::key_would_travel_in_the_clear(&engine.base_url);
        let persists = preferences.credentials_persist();
        let state = preferences.key().clone();
        let stored = state == KeyState::Stored;
        // The one thing on this page that depends on what is *in* a
        // field rather than on what has been chosen — see the
        // subscription in `SettingsView::new`.
        let typed = !self.api_key.read(cx).value().trim().is_empty();

        let (note, tone) = if !wanted {
            (t(Message::SettingsEngineKeyNotUsed), muted)
        } else {
            match &state {
                KeyState::Unknown => (t(Message::SettingsEngineStateChecking), muted),
                KeyState::Stored => (
                    t_args(
                        Message::SettingsEngineKeyStored,
                        &args!("origin" => origin.clone()),
                    ),
                    muted,
                ),
                KeyState::Absent => (
                    t_args(
                        Message::SettingsEngineKeyAbsent,
                        &args!("origin" => origin.clone()),
                    ),
                    muted,
                ),
                KeyState::Failed(reason) => (
                    t_args(
                        Message::SettingsEngineKeyFailed,
                        &args!("reason" => reason.clone()),
                    ),
                    danger,
                ),
            }
        };

        v_flex()
            .gap_1()
            .w_full()
            .child(
                Input::new(&self.api_key)
                    .small()
                    // The eye. Its glyphs are asked for by
                    // `gpui_component::IconName::Eye` / `EyeOff`, so the
                    // files have to be ours under *those* spellings or
                    // the button paints blank space and logs
                    // `asset not found` once a frame — see
                    // docs/architecture/icons.md.
                    .mask_toggle()
                    .disabled(!wanted),
            )
            .child(
                // Ghost rather than outline: these sit directly under a
                // field, in a 240 px column, and an outlined pair there
                // reads as two boxes stuck to the bottom edge of the
                // input rather than as two things to do with it. The
                // same pair on the profile row is styled to match — two
                // buttons on one page both labelled Save must not be two
                // different-looking buttons.
                h_flex()
                    .gap_1()
                    .justify_end()
                    .child(
                        Button::new("engine-key-save")
                            .xsmall()
                            .ghost()
                            .label(t(Message::SettingsEngineKeySave))
                            .disabled(!wanted || !typed)
                            .on_click(cx.listener(|view, _, window, cx| {
                                let secret =
                                    Secret::from(view.api_key.read(cx).value().to_string());
                                if secret.is_empty() {
                                    return;
                                }
                                view.preferences.update(cx, |preferences, cx| {
                                    preferences.store_key(secret, cx);
                                });
                                // Emptied here rather than when the
                                // write comes back: the field is the
                                // only copy of the key still on screen,
                                // and it should stop being one as soon
                                // as there is somewhere better for it.
                                view.api_key.update(cx, |field, cx| {
                                    field.set_value("", window, cx);
                                });
                            })),
                    )
                    .child(
                        Button::new("engine-key-forget")
                            .xsmall()
                            .ghost()
                            .label(t(Message::SettingsEngineKeyForget))
                            .disabled(!stored)
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.preferences.update(cx, |preferences, cx| {
                                    preferences.forget_key(cx);
                                });
                            })),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(tone)
                    .child(SharedString::from(note)),
            )
            // Both of these are conditions rather than states, and each
            // is only on screen while it is true. A warning that is
            // always there is one nobody reads on the day it starts
            // applying.
            .when(wanted && in_the_clear, |control| {
                control.child(
                    div()
                        .text_xs()
                        .text_color(danger)
                        .child(SharedString::from(t_args(
                            Message::SettingsEngineKeyWouldBeInTheClear,
                            &args!("origin" => origin.clone()),
                        ))),
                )
            })
            .when(wanted && !persists, |control| {
                control.child(
                    div()
                        .text_xs()
                        .text_color(danger)
                        .child(SharedString::from(t(
                            Message::SettingsEngineKeyNotPersistent,
                        ))),
                )
            })
    }

    /// The switch that decides whether the document may leave this
    /// machine. Default-deny, and the only control on this page whose
    /// off position is the safe one.
    fn allow_remote_switch(&self, cx: &Context<Self>) -> impl IntoElement {
        let allowed = self.preferences.read(cx).engine().allow_remote;
        Switch::new("engine-allow-remote")
            .checked(allowed)
            .on_click(cx.listener(|view, allowed: &bool, _, cx| {
                let allowed = *allowed;
                view.preferences.update(cx, |preferences, cx| {
                    preferences.allow_remote(allowed, cx);
                });
            }))
    }

    /// The Engine page: what this configuration would do, the rows that
    /// decide it, and the sentence saying that nothing does it yet.
    fn engine(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(heading(
                Message::SettingsEngineTitle,
                Some(Message::SettingsEngineDescription),
                cx,
            ))
            .child(self.state_of_the_engine(cx))
            .child(self.local_model(cx))
            .child(self.rows(Section::Engine, cx))
    }

    /// The model on this machine — whether it is in memory, how much the
    /// process holds, **Unload now** — or, when an endpoint is on duty, the
    /// endpoint; and **Check** with what it found, for either.
    ///
    /// Between the banner and the rows, rather than inside a row: it is a
    /// state with buttons and a result, and the 240 px control column
    /// would wrap every one of its sentences. The three rows that decide
    /// how long the machine's model is kept are directly under "Who
    /// rewrites", below, and say that they concern the model on this
    /// machine — an endpoint keeps nothing here.
    fn local_model(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let border = theme.border;
        let background = theme.muted;
        let Some(host) = self.host.clone() else {
            return div().into_any_element();
        };
        let preferences = self.preferences.read(cx);
        let duty = preferences.duty(Role::Rewrite);
        let keeping = preferences.local_policy().keeping;
        let state = host.read(cx);
        let endpoint = match duty.performer() {
            Some(Performer::Endpoint(remote)) => Some(remote),
            _ => None,
        };
        let (title, (tone, lines)) = match endpoint {
            Some(remote) => (
                Message::SettingsEngineRemoteTitle,
                endpoint_status(remote, state.refused().as_ref()),
            ),
            None => {
                let (here, file) = match duty.performer() {
                    Some(Performer::Machine(local)) => (
                        true,
                        match preferences.model_state(&local.id) {
                            State::Present { bytes } => Some(bytes),
                            _ => None,
                        },
                    ),
                    _ => (false, None),
                };
                (
                    Message::SettingsEngineLocalTitle,
                    local_status(
                        here,
                        state.model(),
                        state.loaded(),
                        state
                            .loaded_at()
                            .map(|at| at.format("%H:%M").to_string())
                            .as_deref(),
                        keeping,
                        file,
                    ),
                )
            }
        };
        let notes = check_note(duty.performer());
        let checked = check_lines(state.check());
        let running = matches!(state.check(), Check::Running { .. });
        let can_unload = state.loaded().holds();
        let can_check = duty.performer().is_some() && state.can_check() && !running;
        let tray = cx.try_global::<Tray>().is_some();
        let machine = endpoint.is_none();

        let unloading = host.clone();
        let checking = host.clone();
        let cancelling = host;
        v_flex()
            .gap_2()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(border)
            .bg(background)
            .text_xs()
            .child(div().font_semibold().child(SharedString::from(t(title))))
            .child(
                v_flex()
                    .gap_1()
                    .children(lines.into_iter().enumerate().map(|(index, line)| {
                        div()
                            .text_color(if index == 0 { tone.colour(cx) } else { muted })
                            .child(SharedString::from(line))
                    })),
            )
            .child(
                h_flex()
                    .gap_2()
                    // An endpoint has nothing loaded here to unload.
                    .when(machine, |row| {
                        row.child(
                            Button::new("engine-unload")
                                .small()
                                .outline()
                                .label(SharedString::from(t(Message::SettingsEngineLocalUnload)))
                                .tooltip(SharedString::from(t(if can_unload {
                                    Message::SettingsEngineLocalUnloadTooltip
                                } else {
                                    Message::SettingsEngineLocalUnloadDisabled
                                })))
                                .disabled(!can_unload)
                                .on_click(move |_, _, cx| {
                                    unloading.update(cx, |host, cx| host.unload_now(cx));
                                }),
                        )
                    })
                    .child(
                        Button::new("engine-check")
                            .small()
                            .outline()
                            .label(SharedString::from(t(Message::SettingsEngineLocalCheck)))
                            .tooltip(SharedString::from(t(if machine {
                                Message::SettingsEngineLocalCheckTooltip
                            } else {
                                Message::SettingsEngineRemoteCheckTooltip
                            })))
                            .disabled(!can_check)
                            .on_click(move |_, _, cx| {
                                checking.update(cx, |host, cx| host.run_check(cx));
                            }),
                    )
                    .when(running, |row| {
                        row.child(
                            Button::new("engine-check-cancel")
                                .small()
                                .ghost()
                                .label(SharedString::from(t(
                                    Message::SettingsEngineLocalCheckCancel,
                                )))
                                .on_click(move |_, _, cx| {
                                    cancelling.read(cx).cancel_check();
                                }),
                        )
                    }),
            )
            .children(
                checked
                    .into_iter()
                    .map(|line| div().child(SharedString::from(line))),
            )
            .children(
                notes
                    .into_iter()
                    .map(|line| div().text_color(muted).child(SharedString::from(line))),
            )
            .when(machine && keeping == Keeping::Resident && !tray, |card| {
                card.child(
                    div()
                        .text_color(muted)
                        .child(SharedString::from(t(Message::SettingsEngineLocalNoTray))),
                )
            })
            .into_any_element()
    }

    /// Load when needed, or keep loaded: two radio buttons.
    fn keep_choice(&self, cx: &Context<Self>) -> impl IntoElement {
        let current = self.preferences.read(cx).local_policy().keeping;
        RadioGroup::vertical("engine-keep")
            .selected_index(Keeping::ALL.iter().position(|k| *k == current))
            .children(
                Keeping::ALL.map(|keeping| Radio::new(keeping.id()).label(t(keeping.title()))),
            )
            .on_click(cx.listener(|view, index: &usize, _, cx| {
                let Some(keeping) = Keeping::ALL.get(*index).copied() else {
                    return;
                };
                view.preferences.update(cx, |preferences, cx| {
                    preferences.select_keeping(keeping, cx);
                });
            }))
    }

    /// The idle span. Disabled while the model is kept loaded, when it
    /// means nothing.
    fn idle_selector(&self, cx: &Context<Self>) -> impl IntoElement {
        let resident = self.preferences.read(cx).local_policy().keeping == Keeping::Resident;
        Select::new(&self.idle_select)
            .small()
            .menu_width(CONTROL_COLUMN)
            .disabled(resident)
    }

    /// The switch that locks the model in RAM.
    fn lock_switch(&self, cx: &Context<Self>) -> impl IntoElement {
        let lock = self.preferences.read(cx).local_policy().lock;
        Switch::new("engine-lock")
            .checked(lock)
            .on_click(cx.listener(|view, lock: &bool, _, cx| {
                let lock = *lock;
                view.preferences.update(cx, |preferences, cx| {
                    preferences.select_lock(lock, cx);
                });
            }))
    }

    /// The banner at the top of the Engine page.
    ///
    /// Everything it says comes from [`engine_banner`], which is a free
    /// function over values so the sentences can be checked without a
    /// window. What stays here is the one thing that needs one: turning
    /// a [`Tone`] into a colour out of the active theme.
    fn state_of_the_engine(&self, cx: &Context<Self>) -> impl IntoElement {
        let preferences = self.preferences.read(cx);
        // Asked before the banner rather than inside it, because "I
        // have not looked in the credential store yet" is a fact about
        // this window and not about who is on duty.
        let looking =
            preferences.engine().provider.takes_a_key() && *preferences.key() == KeyState::Unknown;
        let duty = preferences.duty(Role::Rewrite);
        let (glyph, tone, lines) = engine_banner(&duty, looking);
        notice(glyph, tone.colour(cx), lines, cx)
    }

    /// The dropdown of saved configurations.
    ///
    /// Its placeholder is an invitation and not “none”, because the only
    /// time it is ever seen is when the settings on this page match no
    /// saved profile: the control is not drawn at all while the list is
    /// empty, so “no saved profiles” there would have been a sentence
    /// that was false every time it appeared.
    fn profile_selector(&self) -> impl IntoElement {
        Select::new(&self.profile_select)
            .small()
            .menu_width(CONTROL_COLUMN)
            .placeholder(SharedString::from(t(
                Message::SettingsEngineProfilePlaceholder,
            )))
    }

    /// Rebuild the profile rows when the list, or which of them is
    /// ticked, has actually moved.
    ///
    /// Two guards and not one. [`listed`] covers the rows themselves,
    /// because `set_items` closes an open dropdown; [`ticked`] covers
    /// the selection, which moves on its own when a profile is applied
    /// or saved from somewhere that is not this control.
    ///
    /// [`listed`]: SettingsView::listed
    /// [`ticked`]: SettingsView::ticked
    fn refresh_profile_choices(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (profiles, standing) = {
            let preferences = self.preferences.read(cx);
            (preferences.profiles().to_vec(), preferences.standing())
        };
        let ids = profiles
            .iter()
            .map(|saved| saved.id.clone())
            .collect::<Vec<_>>();
        let ticked = standing.id().map(str::to_owned);
        if ids == self.listed && ticked == self.ticked {
            return;
        }

        let rows_moved = ids != self.listed;
        self.listed = ids;
        self.ticked.clone_from(&ticked);

        let choices = profile::choices(&profiles);
        let row = ticked
            .as_ref()
            .and_then(|id| engine::row_of(&choices, id))
            .map(IndexPath::new);
        self.profile_select.update(cx, |select, cx| {
            if rows_moved {
                select.set_items(choices, window, cx);
            }
            select.set_selected_index(row, window, cx);
        });
    }

    /// Put an applied profile into the fields it came from.
    ///
    /// Called only when [`Preferences::applied`] has moved, which is the
    /// whole design: five of these seven settings are *read* from these
    /// widgets as they are typed, so a profile that changed the entity
    /// without changing them would be overwritten by the next keystroke
    /// in any one of them.
    ///
    /// Every write here is idempotent at the far end — each `select_*`
    /// returns early on a value it already holds — so the change events
    /// these set off cost a comparison and stop.
    fn apply_engine_fields(&self, window: &mut Window, cx: &mut Context<Self>) {
        let engine = self.preferences.read(cx).engine().clone();

        self.endpoint.update(cx, |field, cx| {
            field.set_value(engine.base_url.to_string(), window, cx);
        });
        self.model.update(cx, |field, cx| {
            field.set_value(engine.model.clone(), window, cx);
        });
        self.temperature.update(cx, |field, cx| {
            field.set_value(engine.temperature.to_string(), window, cx);
        });
        self.timeout.update(cx, |field, cx| {
            field.set_value(engine.timeout.to_string(), window, cx);
        });

        let choices = engine::provider_choices();
        let row = engine::row_of(&choices, &engine.provider).map(IndexPath::new);
        self.provider_select.update(cx, |select, cx| {
            select.set_selected_index(row, window, cx);
        });

        let choices = engine::reasoning_choices();
        let row = engine::row_of(&choices, &engine.reasoning).map(IndexPath::new);
        self.reasoning_select.update(cx, |select, cx| {
            select.set_selected_index(row, window, cx);
        });
    }

    /// The saved-profile row: the list, and the two things that can be
    /// done to it.
    ///
    /// Neither of them acts here. Save opens a dialog because the
    /// question it asks has two answers — a new name, or one of the
    /// names already taken — and the second of those is a list, not a
    /// field. Delete opens one because it is the only control on these
    /// pages whose effect cannot be undone by clicking the other way.
    fn profile_control(&self, cx: &Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let preferences = self.preferences.read(cx);
        let empty = preferences.profiles().is_empty();
        let standing = preferences.standing();
        // Delete removes the profile the page is *on*, by its own id.
        // Not one re-derived from a name — see `profile::by_name` — and
        // not whatever a field happens to hold, because there is no
        // longer a field: the name lives in the dialog now.
        let removable = standing.id().is_some();
        let lines = profile_lines(&standing);

        v_flex()
            .gap_1()
            .w_full()
            // Not drawn at all while there is nothing in it, and that is
            // an icons rule rather than a taste one: an empty `Select`
            // paints gpui-component's own empty state, which asks for
            // `icons/inbox.svg` — a glyph this repository does not ship,
            // so it would render as blank space and log `asset not
            // found` once a frame. A dropdown with no rows is a control
            // that does nothing anyway; what a first launch has here is
            // Save.
            .when(!empty, |control| control.child(self.profile_selector()))
            .child(
                h_flex()
                    .gap_1()
                    .justify_end()
                    .child(
                        Button::new("engine-profile-save")
                            .xsmall()
                            .ghost()
                            .label(t(Message::SettingsEngineProfileSave))
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.name_a_profile(window, cx);
                            })),
                    )
                    .child(
                        Button::new("engine-profile-delete")
                            .xsmall()
                            .ghost()
                            .label(t(Message::SettingsEngineProfileDelete))
                            .disabled(!removable)
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.confirm_removing_a_profile(window, cx);
                            })),
                    ),
            )
            .children(lines.into_iter().map(|line| {
                div()
                    .text_xs()
                    .text_color(muted)
                    .child(SharedString::from(line))
            }))
    }

    /// Ask what these settings should be saved as.
    ///
    /// Opened from a click listener and not from inside a
    /// `WindowHandle::update`: this overlay is an element in this view's
    /// own tree, so there is no `Root` in the middle and nothing to
    /// borrow twice — see `crate::dialog`.
    fn name_a_profile(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (starting, taken) = {
            let preferences = self.preferences.read(cx);
            (
                preferences.standing().name().unwrap_or_default().to_owned(),
                preferences
                    .profiles()
                    .iter()
                    .map(|saved| saved.name.clone())
                    .collect::<Vec<_>>(),
            )
        };

        let dialog = cx.new(|cx| {
            Naming::new(
                t(Message::SettingsEngineProfileNameTitle),
                vec![t(Message::SettingsEngineProfileNameBody)],
                t(Message::SettingsEngineProfileNameTaken),
                taken,
                t(Message::SettingsEngineProfileNameConfirm),
                t(Message::SettingsEngineProfileCancel),
                starting,
                t(Message::SettingsEngineProfileNamePlaceholder),
                profile::typeable_name,
                window,
                cx,
            )
        });

        let watched = cx.subscribe_in(&dialog, window, |view, _, chosen: &Chosen, window, cx| {
            if let Chosen::Name(name) = chosen {
                let name = name.clone();
                view.preferences.update(cx, |preferences, cx| {
                    preferences.save_profile(&name, cx);
                });
            }
            view.close_the_dialog(window, cx);
        });

        self.overlay = Some(Overlay::Naming(dialog));
        self.answered = Some(watched);
        cx.notify();
    }

    /// Ask before removing the profile the page is on.
    fn confirm_removing_a_profile(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((id, name)) = ({
            let standing = self.preferences.read(cx).standing();
            standing.id().map(|id| {
                (
                    id.to_owned(),
                    standing.name().unwrap_or_default().to_owned(),
                )
            })
        }) else {
            return;
        };

        let dialog = cx.new(|cx| {
            Confirm::new(
                t_args(
                    Message::SettingsEngineProfileDeleteTitle,
                    &args!("name" => name),
                ),
                vec![t(Message::SettingsEngineProfileDeleteBody)],
                t(Message::SettingsEngineProfileDeleteConfirm),
                t(Message::SettingsEngineProfileCancel),
                window,
                cx,
            )
        });

        let watched = cx.subscribe_in(
            &dialog,
            window,
            move |view, _, answer: &Answer, window, cx| {
                if *answer == Answer::Accepted {
                    let id = id.clone();
                    view.preferences.update(cx, |preferences, cx| {
                        preferences.forget_profile(&id, cx);
                    });
                }
                view.close_the_dialog(window, cx);
            },
        );

        self.overlay = Some(Overlay::Confirm(dialog));
        self.answered = Some(watched);
        cx.notify();
    }

    /// Take the dialog down and hand the keyboard back.
    ///
    /// The order matters. The dialog holds focus while it is open — that
    /// is what makes it modal — so dropping it without saying where
    /// focus goes next leaves a window whose keyboard reaches nothing,
    /// which is the state `SettingsView::focus` exists to avoid in the
    /// first place.
    fn close_the_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.overlay = None;
        self.answered = None;
        self.focus.focus(window, cx);
        cx.notify();
    }

    /// The dialog on top of the page, if there is one.
    ///
    /// `deferred` with a priority above everything else that defers, and
    /// that is not a paint-order nicety — it is what makes the dialog
    /// modal at all. Painting last in tree order is not enough: several
    /// gpui-component controls defer their own drawing, the `Sidebar`
    /// among them, so a plain sibling ends up *under* it in both paint
    /// and hit testing. A click on the sidebar then reached straight
    /// through the backdrop and changed the page behind an open dialog,
    /// which is precisely the thing a backdrop exists to stop. The
    /// priorities in use around here are 1 for popovers, selects and
    /// context menus and 2 for tooltips and date pickers; a dialog is
    /// above all of them.
    fn dialog(&self) -> Option<AnyElement> {
        let dialog: AnyElement = match self.overlay.as_ref()? {
            Overlay::Naming(dialog) => dialog.clone().into_any_element(),
            Overlay::Confirm(dialog) => dialog.clone().into_any_element(),
        };
        Some(
            deferred(dialog)
                .with_priority(DIALOG_PRIORITY)
                .into_any_element(),
        )
    }

    /// The models folder: the field, and under it the picker and the
    /// way back to the default.
    ///
    /// The same arrangement as the address and the endpoint — the field
    /// is the setting, and both buttons write *into* it — and one
    /// difference: the whole control is off while a download runs,
    /// with the sentence under it saying why. `select_models_dir`
    /// refuses for the same span; this is the wall the user sees and
    /// that is the one that holds.
    fn folder_control(&self, cx: &Context<Self>) -> impl IntoElement {
        let preferences = self.preferences.read(cx);
        let busy = preferences.any_download_running();
        let is_default = preferences.models_dir_is_default();
        let muted = cx.theme().muted_foreground;

        v_flex()
            .gap_1()
            .w_full()
            .child(Input::new(&self.folder).small().disabled(busy))
            .child(
                h_flex()
                    .justify_end()
                    .gap_1()
                    .child(
                        Button::new("models-folder-choose")
                            .xsmall()
                            .outline()
                            .icon(IconName::FolderOpen)
                            .label(SharedString::from(t(Message::SettingsModelsFolderChoose)))
                            .disabled(busy)
                            .on_click(cx.listener(|_, _, window, cx| {
                                Self::choose_folder(window, cx, |view, dir, window, cx| {
                                    view.set_folder(Some(dir), window, cx);
                                });
                            })),
                    )
                    .child(
                        Button::new("models-folder-default")
                            .xsmall()
                            .outline()
                            .label(SharedString::from(t(Message::SettingsModelsFolderDefault)))
                            .selected(is_default)
                            .disabled(busy || is_default)
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.set_folder(None, window, cx);
                            })),
                    ),
            )
            .when(busy, |control| {
                control.child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .child(SharedString::from(t(Message::SettingsModelsFolderBusy))),
                )
            })
    }

    /// What the field says, once the user is done with it.
    ///
    /// Read generously and refused strictly — `models::folder_typed` —
    /// and a refusal puts back what the field showed, the way the
    /// shortcut recorder puts back the old chord: the alternative is a
    /// field showing a folder that is not the one in effect.
    fn commit_folder(&mut self, typed: String, window: &mut Window, cx: &mut Context<Self>) {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        match models::folder_typed(&typed, home.as_deref()) {
            models::Typed::Default => self.set_folder(None, window, cx),
            models::Typed::Folder(dir) => self.set_folder(Some(dir), window, cx),
            models::Typed::Unusable => {
                let shown = self.folder_shown.display().to_string();
                self.folder.update(cx, |field, cx| {
                    field.set_value(shown, window, cx);
                });
            }
        }
    }

    /// Move the folder, and show what was moved to.
    ///
    /// The field is written here and not only by the observer, because
    /// the observer compares against `folder_shown` and a commit that
    /// changed nothing — the same folder typed again, or a refusal
    /// while a download runs — would otherwise leave whatever was
    /// typed on screen over a preference that did not move.
    fn set_folder(&mut self, dir: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) {
        self.preferences.update(cx, |preferences, cx| {
            preferences.select_models_dir(dir, cx);
        });
        let shown = self.preferences.read(cx).models_dir().to_path_buf();
        self.folder_shown = shown;
        let text = self.folder_shown.display().to_string();
        self.folder.update(cx, |field, cx| {
            field.set_value(text, window, cx);
        });
    }

    /// Put the folder in effect into the field, when it moved somewhere
    /// this window did not see — the walk-through, or another window.
    /// A no-op otherwise, which is what keeps it from overwriting a
    /// path somebody is halfway through typing.
    fn show_the_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let now = self.preferences.read(cx).models_dir().to_path_buf();
        if now == self.folder_shown {
            return;
        }
        self.folder_shown = now;
        let text = self.folder_shown.display().to_string();
        self.folder.update(cx, |field, cx| {
            field.set_value(text, window, cx);
        });
    }

    /// The platform's folder picker, and the answer written into the
    /// field.
    ///
    /// The picker is the platform's own panel and comes back over a
    /// channel; nothing here waits on it. Cancelling leaves everything
    /// as it was.
    fn choose_folder(
        window: &Window,
        cx: &Context<Self>,
        chosen: fn(&mut Self, PathBuf, &mut Window, &mut Context<Self>),
    ) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(SharedString::from(t(Message::SettingsModelsFolderChoose))),
        });
        cx.spawn_in(window, async move |view, cx| {
            let picked = match picked.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                Ok(Ok(None)) => None,
                Ok(Err(error)) => {
                    tracing::warn!(%error, "the folder picker could not open");
                    None
                }
                Err(_) => None,
            };
            let Some(dir) = picked else {
                return;
            };
            // The window may have closed while the panel was up; a
            // choice with nowhere to land is dropped, not applied
            // behind the user's back.
            view.update_in(cx, |view, window, cx| {
                chosen(view, dir, window, cx);
            })
            .ok();
        })
        .detach();
    }

    /// The dropdown of models that are actually on this machine.
    ///
    /// Its placeholder is the empty row's label rather than a prompt,
    /// because "no local model" is a real answer and not a missing one.
    fn model_selector(&self) -> impl IntoElement {
        Select::new(&self.model_select)
            .small()
            .menu_width(CONTROL_COLUMN)
            .placeholder(SharedString::from(t(Message::SettingsModelsRewriteNone)))
    }

    /// Rebuild the model rows when what is on this machine has changed.
    ///
    /// Guarded by [`SettingsView::offered`] rather than run every frame:
    /// `set_items` closes an open dropdown, so a rebuild on every
    /// notification would shut the list under the pointer of whoever
    /// was choosing from it.
    fn refresh_model_choices(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (installed, chosen) = {
            let preferences = self.preferences.read(cx);
            (
                preferences
                    .installed_for(Role::Rewrite)
                    .into_iter()
                    .map(|entry| entry.id.clone())
                    .collect::<Vec<_>>(),
                preferences.rewrite_model().map(str::to_owned),
            )
        };
        if installed == self.offered {
            return;
        }
        self.offered.clone_from(&installed);

        let entries = self.preferences.read(cx).installed_for(Role::Rewrite);
        let choices = models::model_choices(entries);
        let row = engine::row_of(&choices, &chosen).map(IndexPath::new);
        self.model_select.update(cx, |select, cx| {
            select.set_items(choices, window, cx);
            select.set_selected_index(row, window, cx);
        });
    }

    /// The Models page: what this machine can hold, the model chosen for
    /// rewriting, and one card per catalogue entry.
    fn models(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(heading(
                Message::SettingsModelsTitle,
                Some(Message::SettingsModelsDescription),
                cx,
            ))
            .child(self.state_of_the_shelf(cx))
            .child(self.rows(Section::Models, cx))
            .child(
                v_flex().gap_2().children(
                    self.preferences
                        .read(cx)
                        .catalogue()
                        .models
                        .iter()
                        .map(|entry| self.model_card(entry, cx)),
                ),
            )
            .children(self.also_in_the_folder(cx))
    }

    /// Everything the look through the folder turned up that the
    /// catalogue did not put there — or nothing at all, when there is
    /// nothing to list.
    ///
    /// A heading, the sentence that says these are listed and not
    /// verified or used, and one line per file: where under the folder,
    /// and how big. No button, because there is nothing this build can
    /// do with one of them, and a card with no action is a card that
    /// looks broken. The banner carries the count either way, so an
    /// empty folder is still described rather than silently missing a
    /// section.
    fn also_in_the_folder(&self, cx: &Context<Self>) -> Option<impl IntoElement> {
        let preferences = self.preferences.read(cx);
        let others = preferences.folder().others();
        if others.is_empty() {
            return None;
        }
        let root = preferences.models_dir();
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let border = theme.border;
        let radius = theme.radius;

        Some(
            v_flex()
                .gap_2()
                .child(heading(
                    Message::SettingsModelsFoundTitle,
                    Some(Message::SettingsModelsFoundDescription),
                    cx,
                ))
                .child(
                    v_flex()
                        .rounded(radius)
                        .border_1()
                        .border_color(border)
                        .children(others.iter().enumerate().map(|(index, found)| {
                            let (at, size) = models::found_row(found, root);
                            h_flex()
                                .gap_4()
                                .items_center()
                                .px_3()
                                .py_2()
                                .when(index > 0, |row| row.border_t_1().border_color(border))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w(px(0.0))
                                        .text_sm()
                                        .child(SharedString::from(at)),
                                )
                                .child(div().flex_none().text_xs().text_color(muted).child(
                                    SharedString::from(format!(
                                        "{size} · {}",
                                        found.format.extension()
                                    )),
                                ))
                        })),
                ),
        )
    }

    /// The banner at the top of the Models page.
    ///
    /// Two lines and always both: what this machine reports, and that a
    /// downloaded model is a file and nothing more in this build. The
    /// second one is the same bargain the Engine banner and the MCP
    /// tools make, and it stays until E2 and E3 have both landed.
    fn state_of_the_shelf(&self, cx: &Context<Self>) -> impl IntoElement {
        let preferences = self.preferences.read(cx);
        let (said, why) = preferences.download_outcome();
        let (tone, lines) = models_banner(
            preferences
                .models_scanned()
                .then(|| preferences.host())
                .flatten(),
            preferences.models_scanned(),
            Shelf {
                dir: preferences.models_dir(),
                installed: preferences.installed_count(),
                folder: preferences.folder(),
            },
            said,
            why,
        );
        notice(IconName::HardDrive, tone.colour(cx), lines, cx)
    }

    /// One catalogue entry.
    fn model_card(&self, entry: &ModelEntry, cx: &Context<Self>) -> impl IntoElement {
        let preferences = self.preferences.read(cx);
        let card = models::card(
            entry,
            preferences.host(),
            &preferences.model_state(&entry.id),
            preferences.downloading(&entry.id),
        );
        let elsewhere = preferences.any_download_running() && !card.availability.is_running();
        let chosen = preferences.rewrite_model() == Some(entry.id.as_str());
        // The catalogue's own answer to "which of these", shown only
        // while the question is still open — see `models::recommended`.
        let suggested = models::recommended(
            preferences.catalogue(),
            Role::Rewrite,
            preferences.host(),
            preferences.rewrite_model(),
        )
        .is_some_and(|best| best.id == entry.id);
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let id = SharedString::from(entry.id.clone());

        v_flex()
            .gap_1()
            .p_3()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(theme.border)
            .child(
                h_flex()
                    .gap_4()
                    .items_center()
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w(px(0.0))
                            .gap_1()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_medium()
                                            .child(SharedString::from(card.display.clone())),
                                    )
                                    // The tick is the only thing that
                                    // says which model would actually
                                    // be used, and it belongs on the
                                    // card rather than only in a
                                    // dropdown three rows up.
                                    .when(chosen, |row| {
                                        row.child(Icon::new(IconName::CircleCheck).small())
                                    })
                                    // Never both: `recommended`
                                    // answers `None` once anything is
                                    // chosen, so the tick and this are
                                    // two states of one question.
                                    .when(suggested, |row| {
                                        row.child(div().text_xs().text_color(muted).child(
                                            SharedString::from(t(
                                                Message::SettingsModelsRecommended,
                                            )),
                                        ))
                                    }),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted)
                                    // The roles are catalogue
                                    // vocabulary, not prose: they are
                                    // the ids the manifest writes, and
                                    // translating one would strand a
                                    // settings row.
                                    .child(SharedString::from(format!(
                                        "{} · {} · {}",
                                        card.size,
                                        card.needs,
                                        card.roles
                                            .iter()
                                            .map(|role| role.id())
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    ))),
                            ),
                    )
                    .child(div().flex().flex_none().justify_end().child({
                        let label = card.availability.action();
                        Button::new(id.clone())
                            .small()
                            .outline()
                            .label(SharedString::from(t(label)))
                            // Every other card's button is off
                            // while one download runs: the
                            // free-space check that let this
                            // one start was made for one file.
                            .disabled(elsewhere)
                            .on_click(cx.listener({
                                let id = entry.id.clone();
                                let availability = card.availability.clone();
                                move |view, _, _, cx| {
                                    view.act_on_model(&id, &availability, cx);
                                }
                            }))
                    })),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child(SharedString::from(card.line())),
            )
    }

    /// The one thing this card's button does.
    ///
    /// Split out so the mapping from state to action is a plain match
    /// over values rather than a closure inside an element tree —
    /// `every_state_offers_one_thing_and_it_is_never_start_over` is the
    /// half of it that can be checked without a window.
    fn act_on_model(&self, id: &str, availability: &models::Availability, cx: &mut Context<Self>) {
        self.preferences
            .update(cx, |preferences, cx| match availability {
                models::Availability::Absent | models::Availability::Resumable { .. } => {
                    preferences.download_model(id, cx);
                }
                models::Availability::Downloading { .. } => preferences.stop_download(cx),
                models::Availability::Installed | models::Availability::Damaged => {
                    preferences.remove_model(id, cx);
                }
            });
    }

    /// The General page: the preferences that do not belong to a named
    /// feature.
    fn general(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(heading(Section::General.title(), None, cx))
            .child(self.rows(Section::General, cx))
    }

    /// The Placement page: which screen a window opens on, and — one
    /// card per display — where on it.
    ///
    /// The cards are the page. The row above them is one answer for the
    /// whole product; a card is an answer for one display, and there is
    /// a card for every display attached because the question only
    /// exists once there are two.
    fn placement(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(heading(
                Message::SettingsPlacementTitle,
                Some(Message::SettingsPlacementDescription),
                cx,
            ))
            .child(self.state_of_the_screens(cx))
            .child(self.rows(Section::Placement, cx))
            .child(
                v_flex().gap_3().children(
                    self.preferences
                        .read(cx)
                        .screens()
                        .iter()
                        .enumerate()
                        .map(|(index, display)| self.display_card(index, display, cx)),
                ),
            )
    }

    /// The banner at the top of the Placement page.
    fn state_of_the_screens(&self, cx: &Context<Self>) -> impl IntoElement {
        let preferences = self.preferences.read(cx);
        let lines = screens_banner(preferences.screens().len(), preferences.screens_scanned());
        notice(
            IconName::TableCellsLarge,
            cx.theme().muted_foreground,
            lines,
            cx,
        )
    }

    /// One display: what it is called, and where a window opens on it.
    ///
    /// `index` numbers the card and nothing else. It is what names a
    /// display the platform has no name for, and what keeps the element
    /// ids inside the grid apart — a uuid would do the second job and
    /// not the first, and a display with no uuid is exactly the one
    /// that needs both.
    fn display_card(
        &self,
        index: usize,
        display: &Connected,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let spot = self.preferences.read(cx).spot_of(&display.screen);
        // The card for the display the panel is on, while there is a
        // panel — which is the card whose choices move it while the
        // user watches. Never this window: Settings opens beside
        // whatever asked for it and is not placed by these choices at
        // all. See `apply_here`.
        let here = self
            .preferences
            .read(cx)
            .panel_screen()
            .is_some_and(|screen| screen.id == display.screen.id);
        let name = display.name.clone().unwrap_or_else(|| {
            t_args(
                Message::SettingsPlacementDisplay,
                &args!("number" => index + 1),
            )
        });
        let size = display.screen.visible.size;

        v_flex()
            .gap_2()
            .p_3()
            .rounded(theme.radius)
            .border_1()
            .border_color(theme.border)
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .font_medium()
                            .child(SharedString::from(name)),
                    )
                    .when(display.primary, |row| {
                        row.child(tag(Message::SettingsPlacementPrimary, cx))
                    })
                    .when(here, |row| {
                        row.child(tag(Message::SettingsPlacementHere, cx))
                    })
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted)
                            .child(SharedString::from(t_args(
                                Message::SettingsPlacementResolution,
                                &args!(
                                    "width" => size.width.as_f32().round().to_string(),
                                    "height" => size.height.as_f32().round().to_string(),
                                ),
                            ))),
                    ),
            )
            .child(
                h_flex()
                    .gap_3()
                    .items_start()
                    .child(zone_grid(index, display, spot, cx))
                    .child(
                        v_flex()
                            .gap_2()
                            .flex_1()
                            .min_w(px(0.0))
                            .child(div().text_xs().text_color(muted).child(SharedString::from(
                                match spot.at {
                                    placement::Answer::Zone(zone) => t_args(
                                        Message::SettingsPlacementOpensIn,
                                        &args!("zone" => t(zone.label())),
                                    ),
                                    placement::Answer::Manual => {
                                        t(Message::SettingsPlacementOpensWhereLeft)
                                    }
                                },
                            )))
                            // Two ways back, each shown only while it
                            // has somewhere to go. A cell cannot be
                            // un-clicked — clicking it again would have
                            // to mean two things — so these are the
                            // controls that undo one.
                            .child(
                                h_flex()
                                    .gap_2()
                                    .children(
                                        (matches!(spot.at, placement::Answer::Zone(_))
                                            && spot.rect.is_some())
                                        .then(|| {
                                            Button::new(SharedString::from(format!(
                                                "by-hand-{index}"
                                            )))
                                            .small()
                                            .outline()
                                            .label(SharedString::from(t(
                                                Message::SettingsPlacementWhereItWasLeft,
                                            )))
                                            .on_click(cx.listener({
                                                let display = display.clone();
                                                move |view, _, _, cx| {
                                                    view.preferences.update(
                                                        cx,
                                                        |preferences, cx| {
                                                            preferences
                                                                .back_to_hand(&display.screen, cx);
                                                        },
                                                    );
                                                }
                                            }))
                                        }),
                                    )
                                    .children((spot != Spot::default()).then(|| {
                                        Button::new(SharedString::from(format!("default-{index}")))
                                            .small()
                                            .ghost()
                                            .label(SharedString::from(t(
                                                Message::SettingsPlacementRestoreDefault,
                                            )))
                                            .on_click(cx.listener({
                                                let display = display.clone();
                                                move |view, _, _, cx| {
                                                    view.restore_default(&display, cx);
                                                }
                                            }))
                                    })),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted)
                                    .child(SharedString::from(t(
                                        Message::SettingsPlacementDragHint,
                                    ))),
                            ),
                    ),
            )
    }

    /// Choose where windows open on one display — or take the answer
    /// back.
    ///
    /// Two things happen, and only the first always does: the answer is
    /// written down for that display, and *this* window moves into the
    /// zone if it is already on that display. See [`apply_here`] for
    /// why the second one is conditional.
    ///
    /// [`apply_here`]: SettingsView::apply_here
    fn choose_zone(&self, display: &Connected, zone: Zone, cx: &mut Context<Self>) {
        self.preferences.update(cx, |preferences, cx| {
            preferences.choose_zone(&display.screen, zone, cx);
        });
        self.apply_here(display, zone, cx);
    }

    /// Throw away everything this display was told, and put the panel
    /// back where — and at the size — a display nobody has answered for
    /// would get it.
    ///
    /// The size is the half a zone never carries, which is why this is
    /// its own path rather than "choose the default cell": somebody who
    /// has dragged the panel to twice its size and asks for the default
    /// back is asking for the size back too.
    fn restore_default(&self, display: &Connected, cx: &mut Context<Self>) {
        self.preferences.update(cx, |preferences, cx| {
            preferences.restore_default(&display.screen, cx);
        });
        self.put_back(display, Spot::default(), Some(panel::SIZE), cx);
    }

    /// Get out of the way of the window that was just placed — if that
    /// is what the user asked for.
    ///
    /// After a *drop* and not after a click on the same cell, which is
    /// the whole of what the switch above says. The two gestures are
    /// not the same request: a drag ends with the pointer on the part
    /// of the screen the window is to open in, which is the moment
    /// there is something to look at, while the cells are also a row of
    /// buttons somebody works along from the keyboard comparing two
    /// corners — and a window that vanished on the first press would
    /// make the second impossible.
    fn make_way(&self, window: &mut Window, cx: &Context<Self>) {
        if !self.preferences.read(cx).closes_after_drop() {
            return;
        }
        // The two statements `CloseSettings` runs, in that order and
        // for the same reason: the platform does not fire
        // `on_window_should_close` for a programmatic close, so a
        // window that closed itself here would be the one way out that
        // forgets where it was.
        self.remember(cx);
        window.remove_window();
    }

    /// Move the panel into `zone`, if there is a panel and it is on
    /// that display.
    fn apply_here(&self, display: &Connected, zone: Zone, cx: &mut Context<Self>) {
        // No rectangle and no size: a cell says where a window goes and
        // never how big it is, so the panel keeps whatever size it has.
        let spot = Spot {
            at: placement::Answer::Zone(zone),
            rect: None,
        };
        self.put_back(display, spot, None, cx);
    }

    /// Put the panel where this display's answer says, and — when a
    /// `size` is given — at that size.
    ///
    /// The condition inside is the feature. Each display holds its own
    /// answer, and choosing one has to be visible immediately when the
    /// window it is about is on screen — otherwise the grid is a
    /// picture of a promise. But a window can only be moved *on the
    /// screen it is already on*: GPUI has no API for moving one at all
    /// (see [`screen::translate`]), and the AppKit call underneath takes
    /// a translation rather than a destination, because the display's
    /// global origin is the one fact GPUI's backend hides.
    ///
    /// So an answer given on the card for the display the panel is on
    /// takes effect now, and every other one — another card, or a panel
    /// that is not on screen — is honoured the next time it is
    /// summoned. Which is most of the time: the panel is a window you
    /// call up, and the usual case is that this does nothing at all
    /// today and everything at the next summon.
    ///
    /// The window this moves is deliberately never the one this page is
    /// drawn in. Settings opens beside whatever asked for it, and a
    /// preferences window that jumped into a corner while its own grid
    /// was being read would be answering a question about a different
    /// window.
    fn put_back(
        &self,
        display: &Connected,
        spot: Spot,
        size: Option<Size<Pixels>>,
        cx: &mut Context<Self>,
    ) {
        let (Some(panel), true) = (
            self.preferences.read(cx).panel_window(),
            self.preferences
                .read(cx)
                .panel_screen()
                .is_some_and(|screen| screen.id == display.screen.id),
        ) else {
            return;
        };

        let held = self.preferences.clone();
        // A different window's update, from inside this one's: allowed,
        // and not the case the "never from inside `WindowHandle::update`"
        // rule is about — nothing here opens a window or touches the
        // other window's `Root`.
        let moved = panel.update(cx, |_, window, cx| {
            let Some(here) = screen::of_window(window, cx) else {
                return false;
            };
            // The frame, because that is what the user sees and what
            // has to land inside the screen. Both rectangles are in
            // this display's own coordinates, which is what makes the
            // subtraction below mean anything.
            let frame = window.bounds();
            let wanted = placement::where_it_goes(spot, size.unwrap_or(frame.size), here.visible);

            // Said before the move rather than after: the window
            // reports the new rectangle from inside the calls below,
            // and a move this application made must not come back as
            // the user's answer. See `Preferences::we_placed_it`.
            held.update(cx, |preferences, _| {
                preferences.we_placed_it(Some(wanted));
            });

            if size.is_some() {
                // `setContentSize:` under there, which keeps the
                // window's top-left corner — so the delta below is
                // still the right one when it lands, and it lands on
                // the foreground executor a moment after this returns.
                window.resize(wanted.size);
            }

            let delta = point(
                wanted.origin.x - frame.origin.x,
                wanted.origin.y - frame.origin.y,
            );
            screen::translate(window, delta)
        });

        if !matches!(moved, Ok(true)) {
            tracing::debug!(
                "the panel did not move; the answer applies the next time it is summoned"
            );
        }
    }

    /// The MCP page: what the server is doing, the rows that decide it,
    /// and the snippet that connects to it.
    /// The Retention page: where results go, and what is kept.
    fn retention(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(heading(
                Message::SettingsRetentionTitle,
                Some(Message::SettingsRetentionDescription),
                cx,
            ))
            .child(self.state_of_the_kept(cx))
            .child(self.rows(Section::Retention, cx))
    }

    /// The banner at the top of the Retention page. Everything it says
    /// comes from [`retention_banner`], over values, so the sentences
    /// can be checked without a window.
    fn state_of_the_kept(&self, cx: &Context<Self>) -> impl IntoElement {
        let preferences = self.preferences.read(cx);
        let lines = retention_banner(
            preferences.retention(),
            preferences.results_folder(),
            &preferences.homes().kept,
        );
        notice(IconName::BoxArchive, cx.theme().muted_foreground, lines, cx)
    }

    /// Beside / into the folder / in place, as three radio buttons —
    /// the shape the screen choice has, and for the same reason: three
    /// short answers that fit the column, and a dropdown would hide
    /// two of them behind a click.
    fn destination_choice(&self, cx: &Context<Self>) -> impl IntoElement {
        let current = self.preferences.read(cx).retention().destination;
        RadioGroup::vertical("results-destination")
            .selected_index(Destination::ALL.iter().position(|d| *d == current))
            .children(
                Destination::ALL
                    .map(|destination| Radio::new(destination.id()).label(t(destination.label()))),
            )
            .on_click(cx.listener(|view, index: &usize, _, cx| {
                // Back through the list the buttons were built from, so
                // an index this control invents cannot become a
                // preference.
                let Some(destination) = Destination::ALL.get(*index).copied() else {
                    return;
                };
                view.preferences.update(cx, |preferences, cx| {
                    preferences.select_destination(destination, cx);
                });
            }))
    }

    /// The results folder: a field, the platform's picker, and
    /// "Default" — the models folder's control over a different row.
    fn results_folder_control(&self, cx: &Context<Self>) -> impl IntoElement {
        let is_default = self.preferences.read(cx).results_folder_is_default();
        v_flex()
            .gap_1()
            .w_full()
            .child(Input::new(&self.results_folder).small())
            .child(
                h_flex()
                    .justify_end()
                    .gap_1()
                    .child(
                        Button::new("results-folder-choose")
                            .xsmall()
                            .outline()
                            .icon(IconName::FolderOpen)
                            .label(SharedString::from(t(Message::SettingsModelsFolderChoose)))
                            .on_click(cx.listener(|_, _, window, cx| {
                                Self::choose_folder(window, cx, |view, dir, window, cx| {
                                    view.set_results_folder(Some(dir), window, cx);
                                });
                            })),
                    )
                    .child(
                        Button::new("results-folder-default")
                            .xsmall()
                            .outline()
                            .label(SharedString::from(t(Message::SettingsModelsFolderDefault)))
                            .selected(is_default)
                            .disabled(is_default)
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.set_results_folder(None, window, cx);
                            })),
                    ),
            )
    }

    /// What the results field says, once the user is done with it —
    /// [`commit_folder`](Self::commit_folder) over the other row, with
    /// the same refusal: a path this build cannot use puts back the
    /// one in effect.
    fn commit_results_folder(
        &mut self,
        typed: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        match models::folder_typed(&typed, home.as_deref()) {
            models::Typed::Default => self.set_results_folder(None, window, cx),
            models::Typed::Folder(dir) => self.set_results_folder(Some(dir), window, cx),
            models::Typed::Unusable => {
                let shown = self.results_shown.display().to_string();
                self.results_folder.update(cx, |field, cx| {
                    field.set_value(shown, window, cx);
                });
            }
        }
    }

    /// Move the results folder, and show what was moved to.
    fn set_results_folder(
        &mut self,
        dir: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.preferences.update(cx, |preferences, cx| {
            preferences.select_results_folder(dir, cx);
        });
        let shown = self.preferences.read(cx).results_folder().to_path_buf();
        self.results_shown = shown;
        let text = self.results_shown.display().to_string();
        self.results_folder.update(cx, |field, cx| {
            field.set_value(text, window, cx);
        });
    }

    /// Put the folder in effect into the field when it moved somewhere
    /// this window did not see; a no-op otherwise, so a path somebody
    /// is halfway through typing is left alone.
    fn show_the_results_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let now = self.preferences.read(cx).results_folder().to_path_buf();
        if now == self.results_shown {
            return;
        }
        self.results_shown = now;
        let text = self.results_shown.display().to_string();
        self.results_folder.update(cx, |field, cx| {
            field.set_value(text, window, cx);
        });
    }

    /// Whether originals that arrived without a file are kept.
    fn keep_originals_switch(&self, cx: &Context<Self>) -> impl IntoElement {
        let keep = self.preferences.read(cx).retention().keep_originals;
        Switch::new("keep-originals")
            .checked(keep)
            .on_click(cx.listener(|view, keep: &bool, _, cx| {
                let keep = *keep;
                view.preferences.update(cx, |preferences, cx| {
                    preferences.keep_originals(keep, cx);
                });
            }))
    }

    /// Whether their results are.
    fn keep_results_switch(&self, cx: &Context<Self>) -> impl IntoElement {
        let keep = self.preferences.read(cx).retention().keep_results;
        Switch::new("keep-results")
            .checked(keep)
            .on_click(cx.listener(|view, keep: &bool, _, cx| {
                let keep = *keep;
                view.preferences.update(cx, |preferences, cx| {
                    preferences.keep_results(keep, cx);
                });
            }))
    }

    /// For how long. Five spans and no free number — see
    /// `retention::Period`.
    fn period_selector(&self) -> impl IntoElement {
        Select::new(&self.period_select)
            .small()
            .menu_width(CONTROL_COLUMN)
    }

    fn mcp(&self, cx: &Context<Self>) -> impl IntoElement {
        let (_, asked_for) = self.preferences.read(cx).mcp();
        v_flex()
            .gap_4()
            .child(heading(
                Message::SettingsMcpTitle,
                Some(Message::SettingsMcpDescription),
                cx,
            ))
            .child(self.state_of_the_server(cx))
            .child(self.rows(Section::Mcp, cx))
            // Under the address it is about, and only when it is true.
            // A warning that is always on screen is a warning nobody
            // reads on the day it starts applying.
            .children(exposed(asked_for.bind, cx))
            .child(self.snippet(self.endpoint_on_screen(cx), cx))
    }

    /// The endpoint the page describes.
    ///
    /// Where the server *is* whenever there is one, and where it was
    /// asked to be otherwise. The distinction is the whole reason the
    /// port scan is safe to have: a server that stepped past a taken
    /// port and left the snippet saying otherwise would hand out a
    /// configuration that dials a port nothing is listening on, which
    /// is a worse failure than not starting.
    fn endpoint_on_screen(&self, cx: &Context<Self>) -> Endpoint {
        let preferences = self.preferences.read(cx);
        on_screen(preferences.status(), preferences.mcp().1)
    }

    /// The banner at the top of the page: what is running, and what it
    /// can do.
    ///
    /// Two facts and one box. The first line changes with the server —
    /// off, coming up, answering somewhere, or refusing to start with
    /// the operating system's reason — and the last one never does: the
    /// tools clean, with Layer A, and nothing rewrites, because Layer B
    /// is not in this version. A user who reads nothing else on this
    /// page has to read both. The decision is [`mcp_banner`]'s.
    fn state_of_the_server(&self, cx: &Context<Self>) -> impl IntoElement {
        let (glyph, tone, lines) = mcp_banner(self.preferences.read(cx).status());
        notice(glyph, tone.colour(cx), lines, cx)
    }

    /// Copy the snippet, and say so for two seconds.
    ///
    /// The glyph is `copy.svg`, promoted from the FA Free corpus — which
    /// is also the file `gpui_component::Clipboard` asks for. That
    /// component was the first thing tried and painted blank space
    /// while logging `asset not found: icons/copy.svg` **once per
    /// frame**, because this application registers *its* asset source
    /// as the only one GPUI can resolve against and the file was not in
    /// it. It is now, so Clipboard would work here.
    ///
    /// This stays a button anyway, and the reason is the word beside
    /// the glyph: Clipboard is icon-only with a tooltip, and a tooltip
    /// is a thing you find by hovering something you already suspected
    /// was a button. `Copy` / `Copied` is in the catalogue, reads in
    /// every language this build ships, and is what turns the tick into
    /// a sentence rather than a decoration.
    fn copy_button(
        &self,
        id: SharedString,
        snippet: SharedString,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let copied = self.copied;
        let label = if copied {
            Message::SettingsMcpCopied
        } else {
            Message::SettingsMcpCopy
        };

        Button::new(id)
            .xsmall()
            .ghost()
            .child(
                h_flex()
                    .gap_1()
                    .items_center()
                    // The tick replaces the glyph rather than joining
                    // it: a row that keeps both is a button that grows
                    // by an icon's width the moment it is pressed, and
                    // the snippet under it moves.
                    .child(
                        Icon::new(if copied {
                            IconName::Check
                        } else {
                            IconName::Copy
                        })
                        .small(),
                    )
                    .child(SharedString::from(t(label))),
            )
            .on_click(cx.listener(move |view, _, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(snippet.to_string()));
                view.copied = true;
                cx.notify();

                // Long enough to be read, short enough that the button
                // is back to offering the thing it does before anyone
                // reaches for it again.
                cx.spawn(async move |view, cx| {
                    cx.background_executor()
                        .timer(std::time::Duration::from_secs(2))
                        .await;
                    view.update(cx, |view, cx| {
                        view.copied = false;
                        cx.notify();
                    })
                    .ok();
                })
                .detach();
            }))
    }

    /// The configuration to paste into a client, and the tabs that
    /// choose which client's shape it takes.
    fn snippet(&self, endpoint: Endpoint, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let border = theme.border;
        let muted = theme.muted_foreground;
        let mono = theme.mono_font_family.clone();
        let selected = Client::ALL
            .iter()
            .position(|client| *client == self.client)
            .unwrap_or_default();
        let snippet = SharedString::from(self.client.snippet(endpoint));
        // Keyed by the client rather than by the block: an element id
        // that four tabs share is one scroll position and one hover
        // state pretending to be four.
        let block = SharedString::from(format!("mcp-snippet-{}", self.client.id()));
        let copy = SharedString::from(format!("copy-mcp-snippet-{}", self.client.id()));

        v_flex()
            .gap_2()
            .pt_2()
            .child(heading(
                Message::SettingsMcpSnippetsTitle,
                Some(Message::SettingsMcpSnippetsDescription),
                cx,
            ))
            // The address on its own line, because it is the one thing
            // on this page a user might need without wanting the whole
            // JSON blob — and because `0.0.0.0` in the row above is not
            // what a client dials. See `BindAddress::host`.
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .font_family(mono.clone())
                    .child(SharedString::from(t_args(
                        Message::SettingsMcpEndpoint,
                        &args!("url" => endpoint.url()),
                    ))),
            )
            .child(
                TabBar::new("mcp-clients")
                    .underline()
                    .selected_index(selected)
                    .children(Client::ALL.map(|client| Tab::new().label(client.label())))
                    .on_click(cx.listener(|view, index: &usize, _, cx| {
                        if let Some(client) = Client::ALL.get(*index) {
                            view.client = *client;
                            // The tick belongs to the snippet that was
                            // copied, not to the button. Leaving it up
                            // across a tab switch claims something that
                            // did not happen.
                            view.copied = false;
                            cx.notify();
                        }
                    })),
            )
            .child(
                div()
                    .relative()
                    .child(
                        div()
                            .id(block)
                            // A snippet is one long line more often
                            // than it is not, and a window that grows
                            // sideways to fit one is a window that no
                            // longer fits the screen.
                            .overflow_x_scroll()
                            .w_full()
                            .p_3()
                            .pr(px(44.0))
                            .rounded_md()
                            .border_1()
                            .border_color(border)
                            .bg(theme.muted)
                            .text_xs()
                            .font_family(mono)
                            .child(snippet.clone()),
                    )
                    .child(
                        div()
                            .absolute()
                            .top_1()
                            .right_1()
                            .child(self.copy_button(copy, snippet, cx)),
                    ),
            )
    }
}

/// A field narrow enough to say "this is a small number".
///
/// A free function for the reason [`heading`] is one: nothing about it
/// depends on which view is drawing it. Not a stepper — a temperature
/// and a timeout are looked up and typed in whole, and nobody arrives
/// at 120 by pressing `+`.
fn number_field(state: &Entity<InputState>) -> impl IntoElement {
    div().w(px(96.0)).child(Input::new(state).small())
}

/// A page's title, and the sentence under it when it has one.
///
/// A free function and not a method: nothing about a heading depends on
/// which view is drawing it, and saying so here is what keeps it that
/// way when the second window wants one.
fn heading(title: Message, description: Option<Message>, cx: &App) -> impl IntoElement {
    let muted = cx.theme().muted_foreground;
    v_flex()
        .gap_1()
        .child(
            div()
                .text_base()
                .font_semibold()
                .child(SharedString::from(t(title))),
        )
        .children(description.map(|description| {
            div()
                .text_sm()
                .text_color(muted)
                .child(SharedString::from(t(description)))
        }))
}

/// A boxed glyph and a line or two beside it.
///
/// The first line is the point and takes the tone; anything after it is
/// context and stays muted, so a banner with two sentences still reads
/// as one statement rather than as two competing ones.
pub(crate) fn notice(
    glyph: IconName,
    tone: Hsla,
    lines: Vec<String>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let muted = theme.muted_foreground;
    h_flex()
        .gap_2()
        .items_start()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .bg(theme.muted)
        .text_xs()
        .child(Icon::new(glyph).small().color(tone))
        .child(v_flex().gap_1().flex_1().min_w(px(0.0)).children(
            lines.into_iter().enumerate().map(|(index, line)| {
                div()
                    .text_color(if index == 0 { tone } else { muted })
                    .child(SharedString::from(line))
            }),
        ))
}

/// One display's six zones, drawn to that display's own shape — and the
/// window on it.
///
/// The window is drawn twice over, in one of two ways and never both: in
/// the cell it was given, or — when it was put somewhere by hand — as
/// that rectangle, to scale, wherever on the screen it actually is.
/// The second is the whole of how "by hand" reads at a glance: no cell
/// is lit, and the window is sitting off the grid lines.
fn zone_grid(
    index: usize,
    display: &Connected,
    spot: Spot,
    cx: &Context<SettingsView>,
) -> impl IntoElement {
    let theme = cx.theme();
    let visible = display.screen.visible;
    // Clamped, because the card is a picture of a screen and not a
    // scale drawing of one: an ultrawide would otherwise push the
    // text column off the window, and a display this build measured
    // as zero-high would divide by nothing.
    let ratio =
        (visible.size.width.as_f32() / visible.size.height.as_f32().max(1.0)).clamp(0.6, 2.4);
    let width = GRID_HEIGHT.as_f32() * ratio;

    // The lines *between* the cells are the page's own background and
    // not `border`: a border colour on a muted ground is a hairline
    // nobody sees, and six invisible cells are one rectangle. Against
    // the card the same colour reads as a gutter — which is what it is.
    let gutter = theme.background;
    let chosen = match spot.at {
        placement::Answer::Zone(zone) => Some(zone),
        placement::Answer::Manual => None,
    };

    v_flex()
        .flex_none()
        .relative()
        .w(px(width))
        .h(GRID_HEIGHT)
        .rounded_sm()
        .border_1()
        .border_color(theme.border)
        .bg(theme.muted)
        .overflow_hidden()
        // Laid out by column and row rather than from a flat list,
        // so the picture on screen and the names in the database
        // are the same fact — `Zone::at` is the one place that
        // mapping lives, and the grid reads it the way a placement
        // does.
        .children((0..Zone::ROWS).map(|row| {
            h_flex()
                .flex_1()
                .w_full()
                .when(row > 0, |cells| cells.border_t_1().border_color(gutter))
                .children(
                    (0..Zone::COLUMNS)
                        .filter_map(move |column| Zone::at(column, row))
                        .enumerate()
                        .map(|(column, zone)| {
                            zone_cell(index, display, zone, chosen, column > 0, cx)
                        }),
                )
        }))
        // Over the cells rather than in one, because a rectangle a hand
        // chose does not belong to any of them.
        .children(chosen.is_none().then(|| {
            let rect = spot.rect.unwrap_or(visible);
            let across = |value: Pixels, from: Pixels, of: Pixels, onto: f32| {
                (value.as_f32() - from.as_f32()) / of.as_f32().max(1.0) * onto
            };
            let left = across(rect.origin.x, visible.origin.x, visible.size.width, width);
            let top = across(
                rect.origin.y,
                visible.origin.y,
                visible.size.height,
                GRID_HEIGHT.as_f32(),
            );
            let across_width = across(rect.size.width, px(0.0), visible.size.width, width);
            let across_height = across(
                rect.size.height,
                px(0.0),
                visible.size.height,
                GRID_HEIGHT.as_f32(),
            );

            div()
                .absolute()
                .left(px(left))
                .top(px(top))
                // Small enough to be lost at this scale otherwise: a
                // 420-point panel on a 3840-point display is four
                // pixels of card.
                .w(px(across_width.max(14.0)))
                .h(px(across_height.max(10.0)))
                .child(chip(
                    index,
                    size(px(across_width.max(14.0)), px(across_height.max(10.0))),
                    cx,
                ))
        }))
}

/// One cell: a drop target, a button, and the window itself when this
/// is where it goes.
///
/// Both ways in are here on purpose. The drag is the one that explains
/// itself — a window, picked up, put somewhere — and the click is the
/// one that works from the keyboard, because `Button` is focusable and
/// a `div` with an `on_click` is not. A preference that can only be
/// changed by dragging is a preference somebody cannot change.
///
/// They part company in one place, and only one: a drop can close this
/// window behind it, and a click never does. See
/// [`SettingsView::make_way`].
fn zone_cell(
    index: usize,
    display: &Connected,
    zone: Zone,
    chosen: Option<Zone>,
    divider: bool,
    cx: &Context<SettingsView>,
) -> impl IntoElement {
    let theme = cx.theme();
    let taken = chosen == Some(zone);

    div()
        .flex_1()
        .h_full()
        // The same gutter `zone_grid` draws between the rows.
        .when(divider, |cell| {
            cell.border_l_1().border_color(theme.background)
        })
        // The cell the window would land in, while it is in the
        // air. Without it a drag is six identical rectangles and a
        // guess.
        .drag_over::<DraggedWindow>(|style, _, _, cx| style.bg(cx.theme().accent))
        .on_drop(cx.listener({
            let display = display.clone();
            move |view, _: &DraggedWindow, window, cx| {
                view.choose_zone(&display, zone, cx);
                view.make_way(window, cx);
            }
        }))
        .child(
            Button::new(SharedString::from(format!("zone-{index}-{}", zone.id())))
                .ghost()
                .w_full()
                .h_full()
                .selected(taken)
                .tooltip(SharedString::from(t(zone.label())))
                .on_click(cx.listener({
                    let display = display.clone();
                    move |view, _, _, cx| {
                        view.choose_zone(&display, zone, cx);
                    }
                }))
                .when(taken, |cell| {
                    cell.child(chip(index, size(px(26.0), px(18.0)), cx))
                }),
        )
}

/// The window, as something that can be picked up.
///
/// One per card, wherever it happens to be sitting — in the chosen
/// cell, or beside the sentence when no cell is chosen. One id per card
/// is therefore enough, and is what keeps two cards' chips from being
/// one element as far as GPUI is concerned.
///
/// A free function, like [`tag`] and [`heading`]: nothing about a
/// window-shaped rectangle depends on which view is drawing it.
fn chip(index: usize, dimensions: Size<Pixels>, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    v_flex()
        .id(SharedString::from(format!("chip-{index}")))
        .flex_none()
        .w(dimensions.width)
        .h(dimensions.height)
        .rounded_sm()
        .border_1()
        .border_color(theme.primary)
        .bg(theme.background)
        .cursor_pointer()
        .child(div().h(px(4.0)).w_full().bg(theme.primary))
        .on_drag(DraggedWindow, |_, _, _, cx| cx.new(|_| WindowChip))
}

/// A short word beside a name: "Primary", "This window".
///
/// A fact about the thing it sits next to rather than a sentence about
/// the page, which is why it is a tag and not a [`notice`]: a row of
/// display names has to stay readable at a glance, and three words in
/// prose beside each one would not be.
fn tag(message: Message, cx: &App) -> impl IntoElement {
    // The theme is reached through the component rather than here; the
    // parameter keeps the call sites uniform with `notice` and
    // `heading`, which do need it.
    let _ = cx;
    Tag::secondary()
        .small()
        .child(SharedString::from(t(message)))
}

/// The Placement page's banner, as values.
///
/// The same shape [`models_banner`] and [`engine_banner`] have, and for
/// the same reason: the sentences a person reads are decided over plain
/// values, so they can be checked without a window.
///
/// The first line is what is attached — or that the displays have not
/// been read yet, which is a different thing from a machine with no
/// displays and lasts a frame or two at startup. The last line never
/// changes: the only window this product has to place today is the
/// Settings window, the workspace windows land in E7, and a page that
/// implied otherwise would be describing a product that does not exist
/// yet.
fn screens_banner(attached: usize, scanned: bool) -> Vec<String> {
    vec![
        if scanned {
            t_args(
                Message::SettingsPlacementAttached,
                &args!("count" => attached),
            )
        } else {
            t(Message::SettingsPlacementLooking)
        },
        t(Message::SettingsPlacementOnlyWindow),
    ]
}

/// The Retention page's banner, as values.
///
/// The same shape [`screens_banner`] has, and for the same reason: the
/// sentences a person reads are decided over plain values, so they can
/// be checked without a window.
///
/// The first line is where results go and what happens to the file;
/// the second is what the product keeps of its own, which for a file
/// is nothing and for a paste is whatever the two switches say. The
/// last line never changes: no window writes anything until E4/E7, and
/// the CLI, which does write, never reads these rows — a page that
/// described a window writing files would describe one that does not
/// exist yet.
fn retention_banner(retention: &Retention, results: &Path, kept: &Path) -> Vec<String> {
    let destination = match retention.destination {
        Destination::Beside => t(Message::SettingsRetentionBeside),
        Destination::Folder => t_args(
            Message::SettingsRetentionInto,
            &args!("folder" => results.display().to_string()),
        ),
        Destination::Replace => t(Message::SettingsRetentionOver),
    };
    let keeps = match (retention.keep_originals, retention.keep_results) {
        (false, false) => t(Message::SettingsRetentionKeepsNothing),
        (originals, results) => t_args(
            match (originals, results) {
                (true, false) => Message::SettingsRetentionKeepsOriginals,
                (false, true) => Message::SettingsRetentionKeepsResults,
                _ => Message::SettingsRetentionKeepsBoth,
            },
            &args!(
                "folder" => kept.display().to_string(),
                "period" => t(retention.keep_for.span())
            ),
        ),
    };
    vec![destination, keeps, t(Message::SettingsRetentionPending)]
}

/// The warning that belongs to an address, and only while it applies.
///
/// Loopback keeps this machine to itself; everything else — the
/// wildcard, or a specific interface — is a server other people can
/// reach, and this one asks for no password. Widening the field made
/// that easier to arrive at by accident, which is exactly when a
/// warning is worth having: `192.168.1.101` does not look like an
/// exposure the way `0.0.0.0` does, and it is the same one.
fn exposed(bind: BindAddress, cx: &App) -> Option<impl IntoElement> {
    bind.on_the_network().then(|| {
        notice(
            IconName::TriangleExclamation,
            cx.theme().warning,
            vec![t_args(
                Message::SettingsMcpExposed,
                &args!("address" => bind.to_string()),
            )],
            cx,
        )
    })
}

/// How loudly the Engine banner says what it says.
///
/// Named rather than a colour, so that the decision — is *this* a
/// warning? — can be tested without a theme, and so that the four
/// answers stay four rather than becoming a palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tone {
    /// Nothing is wrong and nothing is happening. "No engine" is this,
    /// because running Layer A alone is not a fault.
    Quiet,
    /// Configured, and the document stays here.
    Good,
    /// Configured or nearly so, and something about it is worth reading
    /// twice: the document would leave this machine, or has been
    /// stopped from doing so.
    Warn,
    /// A credential is involved and something is wrong with it.
    Bad,
}

impl Tone {
    pub(crate) fn colour(self, cx: &App) -> Hsla {
        let theme = cx.theme();
        match self {
            Self::Quiet => theme.muted_foreground,
            Self::Good => theme.success,
            Self::Warn => theme.warning,
            Self::Bad => theme.danger,
        }
    }
}

/// What the Engine banner shows: a glyph, a tone, and one or two lines.
///
/// A free function over values, like `on_screen` above, so that the
/// rules can be tested without a window — and there are two of them
/// worth testing. The first line is what this configuration would
/// actually do, or the first thing standing in the way of it doing
/// anything, one at a time: three of the five refusals stop applying
/// the moment the first is fixed, and a banner listing all of them is
/// one a reader gives up on. The last line never changes, because
/// Layer B lands in E2 and until then nothing here sends a request.
/// A user who reads nothing else on this page has to read both.
/// The Models page's banner, as values.
///
/// The same shape [`engine_banner`] has, and for the same reason: the
/// sentences a user reads are decided here, over plain values, so they
/// can be checked without a window.
///
/// Always ends with [`Message::SettingsModelsPending`]. A downloaded
/// model is a file on disk and nothing more in this build — loading one
/// is epic E2 and this catalogue is E3 — and a page that let somebody
/// spend seven gigabytes without saying so would be selling work that
/// has not been done.
fn models_banner(
    host: Option<Host>,
    scanned: bool,
    shelf: Shelf<'_>,
    said: Option<Message>,
    why: Option<&str>,
) -> (Tone, Vec<String>) {
    let mut lines = vec![if scanned {
        models::host_line(host)
    } else {
        t(Message::SettingsModelsVerifying)
    }];
    // Which folder, and what is in it beyond the cards. Absent until
    // the scan has read it: the first line already says it is looking.
    lines.extend(models::folder_line(
        shelf.dir,
        shelf.installed,
        shelf.folder,
    ));
    if let Some(said) = said {
        lines.push(match why {
            // The store's own words, never translated: it names a URL,
            // an HTTP status or an operating system error, and a
            // localized one strands whoever is being asked to debug it.
            Some(reason) => t_args(said, &args!("reason" => reason.to_string())),
            None => t(said),
        });
    }
    lines.push(t(Message::SettingsModelsPending));
    // A stopped download is not a fault — the bytes are kept and the
    // next attempt carries on — so only a real failure changes the
    // tone. A folder that cannot be read is one too; a folder that
    // does not exist yet is not, because on a fresh install it never
    // does until the first download.
    let tone = if why.is_some() || shelf.folder.is_a_fault() {
        Tone::Warn
    } else {
        Tone::Quiet
    };
    (tone, lines)
}

/// The folder half of the Models banner's input: where the weights
/// live, how many catalogue entries are whole there, and what else the
/// walk found. Borrowed, because the banner is built on the frame.
#[derive(Debug, Clone, Copy)]
struct Shelf<'a> {
    dir: &'a Path,
    installed: usize,
    folder: &'a Folder,
}

/// Which sentences the profile row says, in order.
///
/// Named apart from the rendering below so that
/// `a_profile_never_offers_to_save_the_key` can check *which* sentences
/// the control says without rendering one: this suite moves the
/// process-wide language while it runs — `language::tests` does it on
/// purpose — and a test comparing rendered text would be racing it. The
/// same reason the engine banner's gate reads an epic name rather than
/// a sentence.
fn profile_notes(standing: &Standing) -> Vec<Message> {
    vec![
        match standing {
            Standing::Unsaved => Message::SettingsEngineProfileUnsaved,
            Standing::Saved { .. } => Message::SettingsEngineProfileSaved,
            Standing::Modified { .. } => Message::SettingsEngineProfileModified,
        },
        // Always, and last. A profile is a row in a file people back up,
        // sync and attach to bug reports, and what makes that safe is
        // that it holds no credential. The moment somebody wants to know
        // is the moment they are about to copy the database somewhere,
        // which is not a moment anyone goes looking for a manual — so
        // the control says it whether or not anything is saved yet.
        Message::SettingsEngineProfileNoKey,
    ]
}

/// What the profile row says about the settings on screen.
///
/// Two lines and always both: where these settings stand relative to
/// what is saved, and what a profile does not carry.
fn profile_lines(standing: &Standing) -> Vec<String> {
    let mut notes = profile_notes(standing).into_iter();
    let standing_says = notes
        .next()
        .expect("a standing always says where it stands");
    let mut lines = vec![match standing {
        Standing::Unsaved => t(standing_says),
        Standing::Saved { name, .. } | Standing::Modified { name, .. } => {
            t_args(standing_says, &args!("name" => name.clone()))
        }
    }];
    lines.extend(notes.map(t));
    lines
}

/// What the Engine page's banner says, from who is on duty.
///
/// It reads a [`Duty`] rather than the settings directly, and that is
/// the load-bearing part. `Provider::Off` used to render as "no engine,
/// Layer A alone", which stops being true the moment a rewriter has
/// been downloaded and chosen on the Models page — the role falls to
/// this machine, and the page said the opposite. One rule, in
/// [`crate::duty`], and one sentence per answer it can give.
///
/// `looking` is the window's own state and not the duty's: the
/// credential store has been asked and has not answered. It only
/// suppresses a *ready* endpoint, never a refusal, because a refusal is
/// already true whatever the key turns out to be.
pub(crate) fn engine_banner(duty: &Duty, looking: bool) -> (IconName, Tone, Vec<String>) {
    let (glyph, tone, first) = match duty {
        Duty::Assigned {
            performer: Performer::Endpoint(_),
            ..
        } if looking => (
            IconName::CircleInfo,
            Tone::Quiet,
            t(Message::SettingsEngineStateChecking),
        ),
        Duty::Assigned { performer, .. } => on_duty_line(performer),
        Duty::Vacant(vacancy) => vacancy_line(vacancy),
    };

    let mut lines = vec![first];
    // What makes an ordered choice a statement rather than a
    // substitution: the side that was asked first said no, and this is
    // what it said. Only ever present when that side was configured —
    // `duty` does not report a provider nobody chose as having lost.
    if let Some(passed) = duty.instead_of() {
        let (_, _, reason) = vacancy_line(passed);
        lines.push(t_args(
            Message::SettingsEngineStateSecondChoice,
            &args!("reason" => reason),
        ));
    }
    lines.push(t(Message::SettingsEnginePending));

    (glyph, tone, lines)
}

/// The sentence for whoever is on duty.
fn on_duty_line(performer: &Performer) -> (IconName, Tone, String) {
    match performer {
        Performer::Endpoint(remote) => {
            let leaves = !remote.on_this_machine;
            (
                IconName::Microchip,
                if leaves { Tone::Warn } else { Tone::Good },
                t_args(
                    if leaves {
                        Message::SettingsEngineStateReadyRemote
                    } else {
                        Message::SettingsEngineStateReadyLocal
                    },
                    &args!("endpoint" => remote.endpoint.clone()),
                ),
            )
        }
        // Weights on this machine. Never a warning: this is the one
        // arrangement where the document does not go anywhere at all.
        Performer::Machine(local) => (
            IconName::Microchip,
            Tone::Good,
            t_args(
                Message::SettingsEngineStateReadyMachine,
                &args!("model" => local.display.clone()),
            ),
        ),
    }
}

/// The sentence for a side that cannot answer.
///
/// One function for both jobs: the banner's first line when nobody is
/// on duty, and its second when somebody is but this is what the other
/// one said. Two renderings of one fact would drift.
fn vacancy_line(vacancy: &Vacancy) -> (IconName, Tone, String) {
    match vacancy {
        // Not "no engine": that is the endpoint's absence and has its
        // own sentence below. This one is the machine's, and it is a
        // whole sentence like the rest, because an ordered choice
        // renders it as a reason and `MachineOnly` — the side the setup
        // walk-through points a fresh install at — renders it as the
        // first line.
        Vacancy::NoModelChosen => (
            IconName::CircleInfo,
            Tone::Quiet,
            t(Message::SettingsEngineStateNoModelChosen),
        ),
        Vacancy::ModelNotHere { .. } => (
            IconName::CircleInfo,
            Tone::Quiet,
            t(Message::SettingsEngineStateModelNotHere),
        ),
        Vacancy::ModelGone { id } | Vacancy::ModelDoesNotServe { id } => (
            IconName::TriangleExclamation,
            Tone::Warn,
            // The id and not the display name: the entry it names is
            // not in this catalogue, or is not what it was, so there is
            // no display name to be had and the stored value is what a
            // reader can act on.
            t_args(
                Message::SettingsEngineStateModelUnusable,
                &args!("model" => id.clone()),
            ),
        ),
        Vacancy::NoSuchProfile { named } => (
            IconName::TriangleExclamation,
            Tone::Warn,
            t_args(
                Message::SettingsEngineStateNoSuchProfile,
                &args!("name" => named.clone()),
            ),
        ),
        Vacancy::Endpoint(refused) => match refused {
            Refusal::NoEngine => (
                IconName::CircleInfo,
                Tone::Quiet,
                t(Message::SettingsEngineStateOff),
            ),
            Refusal::NoModel => (
                IconName::CircleInfo,
                Tone::Quiet,
                t(Message::SettingsEngineStateNoModel),
            ),
            Refusal::NoKey { origin } => (
                IconName::CircleInfo,
                Tone::Quiet,
                t_args(
                    Message::SettingsEngineStateNoKey,
                    &args!("origin" => origin.clone()),
                ),
            ),
            Refusal::RemoteNotAllowed { host } => (
                IconName::TriangleExclamation,
                Tone::Warn,
                t_args(
                    Message::SettingsEngineStateRemoteRefused,
                    &args!("host" => host.clone()),
                ),
            ),
            Refusal::KeyInTheClear { origin } => (
                IconName::TriangleExclamation,
                Tone::Bad,
                t_args(
                    Message::SettingsEngineStateKeyInTheClear,
                    &args!("origin" => origin.clone()),
                ),
            ),
            Refusal::KeyUnreadable { reason } => (
                IconName::TriangleExclamation,
                Tone::Bad,
                t_args(
                    Message::SettingsEngineStateKeyUnreadable,
                    &args!("reason" => reason.clone()),
                ),
            ),
        },
    }
}

/// The idle spans, as the dropdown lists them.
fn idle_choices() -> Vec<Choice<u32>> {
    IDLE_MINUTES
        .into_iter()
        .map(|minutes| {
            Choice::new(
                minutes,
                t_args(
                    Message::SettingsEngineIdleMinutes,
                    &args!("count" => minutes),
                ),
                minutes.to_string(),
            )
        })
        .collect()
}

/// The local-model block's state, as values: a tone and its lines.
///
/// A free function over values, like the banners, so the sentences can be
/// checked without a window. Memory is the process's, measured (D55), and
/// is shown only when it was read; nothing here says a word about
/// rewriting, because nothing rewrites.
fn local_status(
    here: bool,
    model: Option<&str>,
    loaded: &Loaded,
    since: Option<&str>,
    keeping: Keeping,
    file: Option<u64>,
) -> (Tone, Vec<String>) {
    let Some(model) = model.filter(|_| here) else {
        return (Tone::Quiet, vec![t(Message::SettingsEngineLocalNotHere)]);
    };
    let model = model.to_owned();
    let since = since.unwrap_or("").to_owned();
    match loaded {
        Loaded::No => {
            let mut lines = vec![t_args(
                Message::SettingsEngineLocalNotLoaded,
                &args!("model" => model),
            )];
            if keeping == Keeping::Resident {
                lines.push(t(Message::SettingsEngineLocalResidentAgain));
            }
            (Tone::Quiet, lines)
        }
        Loaded::Loading => (
            Tone::Quiet,
            vec![t_args(
                Message::SettingsEngineLocalLoading,
                &args!("model" => model),
            )],
        ),
        Loaded::Yes { resident_mb, .. } => {
            let mut lines = vec![match resident_mb {
                Some(mb) => t_args(
                    Message::SettingsEngineLocalLoaded,
                    &args!(
                        "model" => model,
                        "ram" => engine_host::memory_label(*mb),
                        "since" => since,
                    ),
                ),
                None => t_args(
                    Message::SettingsEngineLocalLoadedUnmeasured,
                    &args!("model" => model, "since" => since),
                ),
            }];
            // The file beside the measurement (D55): the two are what a
            // reader compares, and neither is the estimate.
            lines.extend(file.map(|bytes| {
                t_args(
                    Message::SettingsEngineLocalFile,
                    &args!("size" => models::bytes_label(bytes)),
                )
            }));
            (Tone::Good, lines)
        }
        Loaded::Failed(why) => {
            let mut lines = vec![t_args(
                Message::SettingsEngineLocalFailed,
                &args!("reason" => engine_host::refusal_line(why)),
            )];
            lines.extend(engine_host::refusal_detail(why).map(str::to_owned));
            (Tone::Warn, lines)
        }
    }
}

/// The endpoint block's state, as values: where a check goes, or why the
/// endpoint on duty cannot be asked at all.
fn endpoint_status(remote: &duty::Remote, refused: Option<&Unavailable>) -> (Tone, Vec<String>) {
    match refused {
        Some(why) => {
            let mut lines = vec![t_args(
                Message::SettingsEngineRemoteRefused,
                &args!("reason" => engine_host::refusal_line(why)),
            )];
            lines.extend(engine_host::refusal_detail(why).map(str::to_owned));
            (Tone::Warn, lines)
        }
        None => (
            if remote.on_this_machine {
                Tone::Good
            } else {
                Tone::Warn
            },
            vec![t_args(
                Message::SettingsEngineRemoteAsks,
                &args!("model" => remote.model.clone(), "endpoint" => remote.endpoint.clone()),
            )],
        ),
    }
}

/// What a check is, under its button: that it is not a rewrite — and, for
/// an endpoint that is not this machine, that its prompt was sent there.
///
/// A free function over values so the sentences can be checked without a
/// window. The prompt is a fixed sentence and never a document, but it
/// left this machine, and a page that let somebody press Check without
/// saying so would be the quiet send this product refuses everywhere else.
fn check_note(performer: Option<&Performer>) -> Vec<String> {
    match performer {
        Some(Performer::Endpoint(remote)) => {
            let mut lines = vec![t(Message::SettingsEngineRemoteCheckNote)];
            if !remote.on_this_machine {
                lines.push(t_args(
                    Message::SettingsEngineRemoteCheckSentTo,
                    &args!("origin" => remote.origin.clone()),
                ));
            }
            lines
        }
        _ => vec![t(Message::SettingsEngineLocalCheckNote)],
    }
}

/// What the last check found, as lines.
///
/// The model's words are shown as its output — quoted, at most eighty
/// characters — and never as a rewrite of anything (D54).
fn check_lines(check: &Check) -> Vec<String> {
    match check {
        Check::Idle => Vec::new(),
        Check::Running { .. } => vec![t(Message::SettingsEngineLocalChecking)],
        Check::Done(CheckOutcome::Answered {
            load_ms,
            tokens,
            per_second,
            text,
        }) => {
            let mut lines = vec![t_args(
                Message::SettingsEngineLocalCheckAnswered,
                &args!("text" => text.clone()),
            )];
            if let Some(ms) = load_ms {
                lines.push(t_args(
                    Message::SettingsEngineLocalCheckLoad,
                    &args!("seconds" => format!("{:.1}", *ms as f64 / 1000.0)),
                ));
            }
            lines.push(match per_second {
                Some(rate) => t_args(
                    Message::SettingsEngineLocalCheckSpeed,
                    &args!("tokens" => tokens.to_string(), "rate" => format!("{rate:.1}")),
                ),
                None => t_args(
                    Message::SettingsEngineLocalCheckSpeedUnknown,
                    &args!("tokens" => tokens.to_string()),
                ),
            });
            lines
        }
        Check::Done(CheckOutcome::EndpointAnswered {
            first_ms,
            pieces,
            per_second,
            text,
        }) => {
            let mut lines = vec![t_args(
                Message::SettingsEngineRemoteCheckAnswered,
                &args!("text" => text.clone()),
            )];
            if let Some(ms) = first_ms {
                lines.push(t_args(
                    Message::SettingsEngineRemoteCheckFirst,
                    &args!("seconds" => format!("{:.1}", *ms as f64 / 1000.0)),
                ));
            }
            lines.push(match per_second {
                Some(rate) => t_args(
                    Message::SettingsEngineRemoteCheckSpeed,
                    &args!("pieces" => pieces.to_string(), "rate" => format!("{rate:.1}")),
                ),
                None => t_args(
                    Message::SettingsEngineRemoteCheckSpeedUnknown,
                    &args!("pieces" => pieces.to_string()),
                ),
            });
            lines
        }
        Check::Done(CheckOutcome::Refused(why)) => {
            let mut lines = vec![t_args(
                Message::SettingsEngineLocalCheckFailed,
                &args!("reason" => engine_host::refusal_line(why)),
            )];
            lines.extend(engine_host::refusal_detail(why).map(str::to_owned));
            lines
        }
        Check::Done(CheckOutcome::Failed(reason)) => vec![t_args(
            Message::SettingsEngineLocalCheckFailed,
            &args!("reason" => reason.clone()),
        )],
        Check::Done(CheckOutcome::Cancelled) => vec![t(Message::SettingsEngineLocalCheckCancelled)],
    }
}

/// The MCP page's banner, as values: what is running, and what the
/// tools do. The last line never changes with the server — it is what
/// the tools are, not what the socket is doing.
///
/// A free function over values, like [`on_screen`] below and
/// [`models_banner`] above, so the sentences can be checked without a
/// window.
fn mcp_banner(status: &Status) -> (IconName, Tone, Vec<String>) {
    let (glyph, tone, mut lines) = match status {
        Status::Off => (
            IconName::CircleInfo,
            Tone::Quiet,
            vec![t(Message::SettingsMcpStatusOff)],
        ),
        Status::Starting => (
            IconName::CircleInfo,
            Tone::Quiet,
            vec![t(Message::SettingsMcpStatusStarting)],
        ),
        Status::Listening { endpoint, wanted } => {
            let mut lines = vec![t_args(
                Message::SettingsMcpStatusListening,
                &args!("url" => endpoint.url()),
            )];
            if endpoint.port != *wanted {
                // Both as text. A Fluent number is grouped by the
                // locale, and a port that reads "5 056" is not a port
                // anybody can type back in.
                lines.push(t_args(
                    Message::SettingsMcpStatusMoved,
                    &args!(
                        "wanted" => wanted.to_string(),
                        "port" => endpoint.port.to_string(),
                    ),
                ));
            }
            (IconName::Plug, Tone::Good, lines)
        }
        Status::Failed(reason) => (
            IconName::TriangleExclamation,
            Tone::Bad,
            vec![t_args(
                Message::SettingsMcpStatusFailed,
                &args!("reason" => reason.clone()),
            )],
        ),
    };
    lines.push(t(Message::SettingsMcpToolsLayerA));
    (glyph, tone, lines)
}

/// The endpoint a page should describe, given what the server is doing
/// and what it was asked to do.
///
/// A free function so that the rule can be tested without a window: the
/// running server wins, because it is the one a snippet has to reach.
fn on_screen(status: &Status, asked_for: Endpoint) -> Endpoint {
    match status {
        Status::Listening { endpoint, .. } => *endpoint,
        Status::Off | Status::Starting | Status::Failed(_) => asked_for,
    }
}

impl Focusable for SettingsView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

/// The rectangle to remember for a window that is on screen.
///
/// Not `window.bounds()`, and the difference is a bug that took four
/// close-and-reopens to see: GPUI's macOS backend takes
/// `WindowOptions::window_bounds` as the *content* rectangle — it hands
/// it straight to `initWithContentRect:` — and returns the *frame* from
/// `Window::bounds()`, titlebar included. Saving what comes out and
/// passing it back in therefore adds a titlebar's height to the window
/// every time it is opened: 340, 373, 406, 439, and by the end of the
/// week Settings is taller than the screen.
///
/// The origin does round-trip — both ends mean the frame's top-left —
/// so only the size has to be taken from the content.
fn geometry_of(window: &Window) -> Bounds<Pixels> {
    Bounds::new(window.bounds().origin, window.viewport_size())
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let background = theme.background;
        let foreground = theme.foreground;

        // An outer container whose only job is to be the coordinate
        // system the dialog is positioned against, and to be the last
        // thing painted. The row below is the page; putting the overlay
        // among *its* children left the sidebar undimmed, because a
        // sibling of the sidebar is not reliably painted over it.
        div()
            .size_full()
            .relative()
            .bg(background)
            .text_color(foreground)
            .child(
                h_flex()
                    .id("settings")
                    .track_focus(&self.focus)
                    .key_context(CONTEXT)
                    .on_action(cx.listener(|view, _: &CloseSettings, window, cx| {
                        // A dialog is modal, so Escape and ⌘W belong to it while
                        // one is open. Closing the window out from under a
                        // dialog would answer a question the user was still
                        // being asked — and Escape reaching here at all is what
                        // closed this window the first time a dialog was
                        // dismissed in it. The dialog stops propagation now; this
                        // is the second lock on the same door.
                        if view.overlay.is_some() {
                            view.close_the_dialog(window, cx);
                            return;
                        }
                        // The window's own close, not the application's: this is
                        // one window of several and ⌘W has never meant more
                        // than that.
                        //
                        // The geometry is written here rather than left to
                        // `on_window_should_close`, which the platform fires for
                        // the close *button* and not for a programmatic
                        // `remove_window` — closing with the keyboard would
                        // otherwise be the one way out that forgets where the
                        // window was.
                        view.remember(cx);
                        // One `ERROR gpui::window: window not found` follows
                        // every use of this, and it is not ours to fix: GPUI's
                        // frame callback is delivered once more after the
                        // window has left `App::windows`, and logs the miss.
                        // `remove_window` is the only programmatic close the
                        // API has — the platform's own close, which the button
                        // uses, is not exposed — and deferring the call does
                        // not move the race. A settings window that ignores ⌘W
                        // is the worse trade.
                        window.remove_window();
                    }))
                    .size_full()
                    .items_start()
                    // Read fresh on every frame: this is what makes a theme
                    // switch repaint rather than half-repaint.
                    .bg(background)
                    .text_color(foreground)
                    .child(self.sections(cx))
                    .child(
                        v_flex()
                            .id(self.section.id())
                            // The section scrolls, the sidebar does not. MCP is
                            // already taller than the smallest window this
                            // opens in, and every section after it will be
                            // longer than the last.
                            .overflow_y_scroll()
                            .flex_1()
                            .min_w(px(0.0))
                            .h_full()
                            .px_6()
                            .py_4()
                            .child(match self.section {
                                Section::General => self.general(cx).into_any_element(),
                                Section::Placement => self.placement(cx).into_any_element(),
                                Section::Compare => self.compare(cx).into_any_element(),
                                Section::Engine => self.engine(cx).into_any_element(),
                                Section::Models => self.models(cx).into_any_element(),
                                Section::Retention => self.retention(cx).into_any_element(),
                                Section::Mcp => self.mcp(cx).into_any_element(),
                            }),
                    ),
            )
            // Last, and a child of the outer container rather than of
            // the row: painted over every part of the page, and outside
            // the scrolling section so a dialog is centred on the window
            // rather than on wherever the page happens to be scrolled
            // to.
            .children(self.dialog())
    }
}

/// Put the Settings window on screen, or bring it forward if it is
/// already there.
///
/// The guard is not paranoia: the window is reachable from the menu
/// bar, from the status bar and from `secondary-,` at once, and none of
/// those can see the others. Two Settings windows would be two sets of
/// controls over one set of preferences.
///
/// `at` is the section a *new* window lands on, and `None` means
/// General. For a window that is already open the two differ: `None`
/// leaves it where the user left it — a second ⌘, is a request to see
/// the window rather than a request to be moved — and `Some` turns it
/// to that page, because a button that *names* a page ("Open the
/// Engine page…") is a request to be moved, and landing on General
/// with the sentence that promised Engine still on screen would read
/// as the button not working. The gear, the shortcut and the menu bar
/// all pass `None`.
pub fn open(
    preferences: &Entity<Preferences>,
    main: AnyWindowHandle,
    origin: Origin,
    at: Option<Section>,
    cx: &mut App,
) {
    if let Some(open) = cx.try_global::<Opened>() {
        let handle = open.window;
        let view = open.view.clone();
        // `cx.windows()` is the authority on whether it is still there:
        // a closed window leaves the global behind, and asking is
        // cheaper than observing every close to clear it.
        if cx.windows().contains(&handle) {
            if let Some(section) = at {
                view.update(cx, |view, cx| {
                    view.section = section;
                    cx.notify();
                });
            }
            if let Err(error) = handle.update(cx, |_, window, _| window.activate_window()) {
                tracing::warn!(%error, "could not bring the Settings window forward");
            }
            return;
        }
    }

    let store = preferences.read(cx).store();
    let (screen, bounds) = place(origin, main, &store, cx);
    let preferences = preferences.clone();

    // The builder below is the only place that holds the new view, and
    // the global wants it. Handed back out through a slot, the way
    // `main` gets the preferences out of the main window's builder.
    let view_slot: std::rc::Rc<std::cell::RefCell<Option<Entity<SettingsView>>>> =
        std::rc::Rc::default();
    let slot = view_slot.clone();

    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some(t(Message::SettingsTitle).into()),
                ..Default::default()
            }),
            window_min_size: Some(MIN_SIZE),
            // Without this the rectangle above is read against the
            // primary display whatever it was measured on, and a window
            // remembered 944 points from the left of the second monitor
            // opens 944 points from the left of the first one. The
            // rectangle and the display are one fact in two halves —
            // see the note at the top of `screen`.
            display_id: screen.as_ref().map(|screen| screen.id),
            // An ordinary window, deliberately, and both alternatives
            // were tried on the real thing before settling.
            // `WindowKind::Dialog` is a macOS *sheet* — attached to the
            // main window's titlebar, unmovable, which would retire the
            // per-screen geometry entirely. `WindowKind::Floating` is an
            // `NSPanel` at `NSFloatingWindowLevel`: it does keep
            // Settings from being buried under the main window, and it
            // hides itself when the application is deactivated rather
            // than hovering over other applications — but coming back
            // to the application leaves the panel *visible and not
            // focused*, with the key window the main one, so ⌘W and the
            // keyboard go somewhere the user is not looking. A normal
            // window comes back focused. This is also what every macOS
            // application does with its preferences.
            //
            // The cost is real and small: click the main window and
            // Settings goes behind it, completely, because it is
            // centred over it. Every way of opening it raises it again.
            ..Default::default()
        },
        move |window, cx| {
            let screen = screen.clone();
            let view = cx.new(|cx| SettingsView::new(preferences, screen, at, window, cx));
            *slot.borrow_mut() = Some(view.clone());
            cx.new(|cx| Root::new(view, window, cx))
        },
    );

    match (opened, view_slot.take()) {
        (Ok(window), Some(view)) => cx.set_global(Opened {
            window: window.into(),
            view,
        }),
        (Ok(_), None) => {
            // Unreachable: the builder ran, because `opened` is `Ok`.
            // Logged rather than asserted, for the reason `main` logs
            // the same case — a window that is on screen is not
            // improved by a panic beside it.
            tracing::error!("the Settings view did not come back out of the window builder");
        }
        (Err(error), _) => tracing::warn!(%error, "could not open the Settings window"),
    }
}

/// Which screen the window goes on, and the rectangle it takes there.
///
/// `main` is passed in rather than looked up, and that is not a style
/// choice: `AnyWindowHandle::update` on a window whose update is
/// already on the stack returns `Err("window not found")`, so a `place`
/// that went looking would silently decide the main window did not
/// exist whenever it was called from inside it. Every caller reaches
/// this through the deferred action in `main`, outside any window's
/// update — see `install_shortcut`.
fn place(
    origin: Origin,
    main: AnyWindowHandle,
    store: &wipemark_store::Store,
    cx: &mut App,
) -> (Option<Screen>, Bounds<Pixels>) {
    // Its screen is the answer for `MainWindow`, its rectangle is what
    // a window opened from it is centred over, and the difference
    // between its frame and its content is the titlebar height — the
    // one thing about the window being opened that cannot be measured
    // before it exists, and the two windows wear the same one.
    let main = main
        .update(cx, |_, window, cx| {
            let chrome = window.bounds().size.height - window.viewport_size().height;
            screen::of_window(window, cx).map(|screen| (screen, window.bounds(), chrome))
        })
        .ok()
        .flatten();
    let chrome = main.as_ref().map_or(Pixels::ZERO, |(_, _, chrome)| *chrome);
    let main = main.map(|(screen, bounds, _)| (screen, bounds));

    // Deliberately *not* `placement::screen_for`. That preference is
    // about the window the product is — the one the Placement page
    // draws a grid for — and this one belongs beside whatever asked for
    // it: over the main window for the gear and `⌘,`, on the pointer's
    // screen for the menu bar, which is drawn on every display and so
    // says nothing about where the user is looking.
    let (screen, over): (Option<Screen>, Option<Bounds<Pixels>>) = match origin {
        Origin::MainWindow => match main {
            Some((screen, bounds)) => (Some(screen), Some(bounds)),
            None => (screen::primary(cx), None),
        },
        Origin::Tray => match screen::under_the_pointer(cx) {
            Some(screen) => (Some(screen), None),
            None => match main {
                Some((screen, bounds)) => (Some(screen), Some(bounds)),
                None => (screen::primary(cx), None),
            },
        },
    };

    let Some(screen) = screen else {
        // No display at all is not a state a desktop is usually in, and
        // GPUI will place the window itself given a rectangle it cannot
        // check. Better than refusing to open Settings.
        return (None, Bounds::new(point(px(0.0), px(0.0)), DEFAULT_SIZE));
    };

    // Everything below is in *frame* rectangles — what the user sees,
    // and what has to fit on the screen — and only the last line
    // converts back to the content rectangle GPUI is handed.
    let remembered = window_state::load(store, &screen)
        .map(|content| placement::frame_of(content, chrome))
        // A rectangle remembered by an older build was remembered for
        // an older layout, and this one is wider. Grown rather than
        // discarded: where the window was is still the user's answer,
        // and only its size has been overtaken.
        .map(|frame| placement::at_least(frame, placement::frame_of_size(MIN_SIZE, chrome)));

    let wanted = placement::frame_for(
        // No zone, ever. A preferences window that put itself in a
        // corner would be answering a question about a different
        // window — see the note on `SettingsView::apply_here`.
        None,
        remembered,
        // The *frame* the default size produces, and not the size
        // itself. GPUI is handed a content rectangle, but what a person
        // sees centred is the frame, and the frame is a titlebar
        // taller: centring the content leaves every Settings window
        // sitting half a titlebar high — sixteen pixels, which nobody
        // can name and everybody can see.
        placement::frame_of_size(DEFAULT_SIZE, chrome),
        over,
        screen.visible,
    );

    let frame = screen::contained(wanted, screen.visible);
    (Some(screen.clone()), placement::content_of(frame, chrome))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use gpui::size;

    use super::*;

    /// A sidebar row's glyph is how it is found on the second visit.
    /// Two rows carrying the same one have spent the icon column and
    /// bought nothing with it, and the day a third section is added is
    /// the day that happens.
    #[test]
    fn every_section_has_a_glyph_of_its_own() {
        let glyphs: BTreeSet<&str> = Section::ALL
            .iter()
            .map(|section| section.glyph().id())
            .collect();
        assert_eq!(
            glyphs.len(),
            Section::ALL.len(),
            "two sections share a glyph: {glyphs:?}"
        );
    }

    /// The rule the window exists to keep: everything this build
    /// persists as a preference can be changed from it.
    ///
    /// The failure it catches is the quiet one. A later epic adds a key
    /// to `config` — an engine, a model directory, an endpoint — wires
    /// the reader and the writer, and stops; the preference is real,
    /// survives a restart, and has no way in but a database client.
    #[test]
    fn every_persisted_preference_has_a_row() {
        let rows: BTreeSet<&str> = Setting::ALL
            .iter()
            .filter_map(|setting| match setting.storage() {
                Storage::Row(key) => Some(key),
                Storage::Credentials => None,
            })
            .collect();
        let persisted: BTreeSet<&str> = config::PERSISTED.iter().copied().collect();
        assert_eq!(
            rows, persisted,
            "the Settings window and the persisted keys have to hold the same set of \
             preferences: anything persisted and not shown can only be changed with a database \
             client, and anything shown and not persisted is a control whose choice does not \
             survive a restart"
        );
    }

    fn every_standing() -> Vec<Standing> {
        vec![
            Standing::Unsaved,
            Standing::Saved {
                id: "work-gateway".to_owned(),
                name: "Work gateway".to_owned(),
            },
            Standing::Modified {
                id: "work-gateway".to_owned(),
                name: "Work gateway".to_owned(),
            },
        ]
    }

    /// The sentence that has to be on screen in every state the control
    /// can be in.
    ///
    /// A profile is a row in a file people back up, sync and attach to
    /// bug reports, and the reason that is safe is that it holds no
    /// credential. Checked over the *messages* rather than the rendered
    /// text, for the reason `the_engine_banner_always_says_a_rewrite_is_not_here_yet`
    /// checks an epic name: another test in this binary moves the
    /// process-wide language while this one runs.
    ///
    /// The failure it exists for is a fourth `Standing` added later with
    /// a cheerful first line and no second one.
    #[test]
    fn a_profile_never_offers_to_save_the_key() {
        for standing in every_standing() {
            let notes = profile_notes(&standing);
            assert_eq!(
                notes.last(),
                Some(&Message::SettingsEngineProfileNoKey),
                "{standing:?} stopped saying that a profile is not a key"
            );
            assert_eq!(
                profile_lines(&standing).len(),
                notes.len(),
                "{standing:?} renders a different number of lines than it names"
            );
            assert!(
                !profile_lines(&standing)[0].trim().is_empty(),
                "{standing:?} says nothing about where it stands"
            );
        }
    }

    /// A profile that has been edited is not a profile that has been
    /// saved, and the control has to say which. Collapsing the two is
    /// how a user saves over a profile they meant to keep — or walks
    /// away from changes they meant to store.
    #[test]
    fn an_edited_profile_does_not_read_as_a_saved_one() {
        let saved = profile_notes(&Standing::Saved {
            id: "work-gateway".to_owned(),
            name: "Work gateway".to_owned(),
        });
        let modified = profile_notes(&Standing::Modified {
            id: "work-gateway".to_owned(),
            name: "Work gateway".to_owned(),
        });
        assert_ne!(
            saved[0], modified[0],
            "a profile with unsaved changes reads exactly like one without"
        );
    }

    /// The other half of that set, and the reason it is a set with a
    /// hole in it rather than a set.
    ///
    /// Exactly one row is not a settings row, it is the API key, and it
    /// is a `Storage::Credentials` on purpose rather than by omission.
    /// The failure this catches is a later edit that "fixes" the hole
    /// by giving the key a row — which is how a credential ends up in a
    /// file that gets synced, backed up and attached to bug reports.
    /// `config::a_key_is_never_written_to_the_settings_table` is the
    /// same rule stated over the database itself.
    #[test]
    fn the_only_preference_that_is_not_a_row_is_the_credential() {
        let elsewhere: Vec<Setting> = Setting::ALL
            .into_iter()
            .filter(|setting| setting.storage() == Storage::Credentials)
            .collect();
        assert_eq!(
            elsewhere,
            vec![Setting::EngineKey],
            "a preference left the settings table, or the key joined it"
        );
    }

    /// A section with no rows is an entry in the sidebar that opens an
    /// empty page — which reads as a broken window, not as an empty
    /// one.
    #[test]
    fn every_section_has_at_least_one_row() {
        for section in Section::ALL {
            assert!(section.rows().next().is_some(), "{section:?} lists nothing");
        }
    }

    /// Every row belongs to exactly one section, so walking the
    /// sections walks the rows. A row whose section is not in
    /// `Section::ALL` is a preference that is persisted, has a widget,
    /// and cannot be reached.
    #[test]
    fn every_row_is_reachable_from_the_sidebar() {
        let reachable: Vec<Setting> = Section::ALL.into_iter().flat_map(Section::rows).collect();
        assert_eq!(reachable, Setting::ALL.to_vec());
    }

    /// Every row reads as something, in whatever language the suite
    /// happens to be running in. A key added to the enum and forgotten
    /// in the catalogue renders as its own id — which looks like a
    /// missing translation and is not.
    #[test]
    fn every_row_has_a_title_and_a_sentence_under_it() {
        for setting in Setting::ALL {
            for (what, message) in [
                ("title", setting.title()),
                ("description", setting.description()),
            ] {
                let text = t(message);
                assert!(!text.is_empty(), "{setting:?} has no {what}");
                assert_ne!(
                    text,
                    message.id(),
                    "{setting:?}'s {what} rendered as its own catalogue key"
                );
            }
        }
    }

    /// Two rows that read the same are one row and a bug. Compared over
    /// the rendered text rather than the keys, because two distinct keys
    /// carrying the same sentence is exactly the copy-paste this
    /// catches.
    #[test]
    fn no_two_rows_say_the_same_thing() {
        for (index, setting) in Setting::ALL.iter().enumerate() {
            for other in &Setting::ALL[index + 1..] {
                assert_ne!(
                    t(setting.title()),
                    t(other.title()),
                    "{setting:?} and {other:?} have the same title"
                );
                assert_ne!(
                    t(setting.description()),
                    t(other.description()),
                    "{setting:?} and {other:?} have the same description"
                );
            }
        }
    }

    /// The banner for one engine configuration, with nothing on this
    /// machine's disk and no profiles saved.
    ///
    /// The banner reads a [`Duty`] now, so a test about a sentence has
    /// to say who is on duty. Every gate below is about the endpoint
    /// half of one; the choice *between* an endpoint and this machine
    /// is `crate::duty`'s own suite.
    fn banner_for(settings: &EngineSettings, key: &KeyState) -> (IconName, Tone, Vec<String>) {
        let keys = BTreeMap::from([(engine::account_of(&settings.base_url), key.clone())]);
        let profiles: Vec<Profile> = Vec::new();
        let chosen = BTreeMap::new();
        let on_disk = BTreeMap::new();
        let catalogue = models::catalogue();
        let duty = duty::on_duty(
            &Roster {
                live: settings,
                profiles: &profiles,
                active: None,
                keys: &keys,
                catalogue: &catalogue,
                chosen: &chosen,
                on_disk: &on_disk,
                host: None,
                serves: Serves::default(),
            },
            Role::Rewrite,
            duty::Pick::Live,
        );
        let looking = settings.provider.takes_a_key() && *key == KeyState::Unknown;
        engine_banner(&duty, looking)
    }

    /// The sentence that was wrong before there was a duty to ask.
    ///
    /// `Provider::Off` renders as "no engine, { -layer-a } alone", and
    /// that stops being true the moment a rewriter has been downloaded
    /// and chosen on the Models page: the role falls to this machine.
    /// The page was describing a configuration the user had already
    /// left, in the calmest possible voice.
    #[test]
    fn a_downloaded_rewriter_is_not_reported_as_no_engine() {
        let catalogue = models::catalogue();
        let entry = catalogue
            .for_role(Role::Rewrite)
            .first()
            .copied()
            .expect("the catalogue ships a rewriter")
            .clone();
        let settings = EngineSettings::default();
        let profiles: Vec<Profile> = Vec::new();
        let keys = BTreeMap::new();
        let chosen = BTreeMap::from([(Role::Rewrite, entry.id.clone())]);
        let on_disk = BTreeMap::from([(
            entry.id.clone(),
            OnDisk {
                state: State::Present {
                    bytes: entry.total_bytes(),
                },
                weights: Some(PathBuf::from("/models/rewriter/weights.gguf")),
            },
        )]);

        let duty = duty::on_duty(
            &Roster {
                live: &settings,
                profiles: &profiles,
                active: None,
                keys: &keys,
                catalogue: &catalogue,
                chosen: &chosen,
                on_disk: &on_disk,
                host: None,
                serves: Serves::default(),
            },
            Role::Rewrite,
            duty::Pick::Live,
        );
        let (_, tone, lines) = engine_banner(&duty, false);
        let (_, alone, _) = banner_for(&settings, &KeyState::Unknown);

        assert_ne!(
            tone, alone,
            "a machine that can rewrite reads the same as one that cannot"
        );
        // The display name is a format and not a translation, so this
        // holds whatever language another test has left the process in.
        assert!(
            lines[0].contains(&entry.display),
            "the banner did not name the model that would run: {:?}",
            lines[0]
        );
        assert!(
            reads_as(&lines[1], Message::SettingsEnginePending),
            "the banner did not end with the sentence that says nothing sends a \
             document anywhere yet: {:?}",
            lines[1]
        );
    }

    /// The ordered choice, on screen. A second choice that answered
    /// silently would be the whole point of the setting thrown away:
    /// the user asked for a ranking, and a ranking that does not say
    /// which rank answered is a fallback wearing a preference's name.
    #[test]
    fn a_second_choice_says_what_the_first_one_could_not_do() {
        let catalogue = models::catalogue();
        let entry = catalogue
            .for_role(Role::Rewrite)
            .first()
            .copied()
            .expect("the catalogue ships a rewriter")
            .clone();
        // A remote endpoint nobody allowed, and a downloaded model.
        let settings = EngineSettings {
            provider: Provider::Ollama,
            base_url: BaseUrl::parse("https://ollama.example.com").expect("a base URL"),
            model: "llama3.1:8b".to_owned(),
            allow_remote: false,
            ..EngineSettings::default()
        };
        let profiles: Vec<Profile> = Vec::new();
        let keys = BTreeMap::new();
        let chosen = BTreeMap::from([(Role::Rewrite, entry.id.clone())]);
        let on_disk = BTreeMap::from([(
            entry.id.clone(),
            OnDisk {
                state: State::Present {
                    bytes: entry.total_bytes(),
                },
                weights: Some(PathBuf::from("/models/rewriter/weights.gguf")),
            },
        )]);

        let duty = duty::on_duty(
            &Roster {
                live: &settings,
                profiles: &profiles,
                active: None,
                keys: &keys,
                catalogue: &catalogue,
                chosen: &chosen,
                on_disk: &on_disk,
                host: None,
                serves: Serves::EndpointFirst,
            },
            Role::Rewrite,
            duty::Pick::Live,
        );
        let (_, _, lines) = engine_banner(&duty, false);

        assert_eq!(
            lines.len(),
            3,
            "who answered, why the other did not, and that nothing sends yet: {lines:?}"
        );
        assert!(
            lines[0].contains(&entry.display),
            "the first line did not name who answered: {:?}",
            lines[0]
        );
        // The host is a format, so this holds whatever language another
        // test has left the process in.
        assert!(
            lines[1].contains("ollama.example.com"),
            "the second line did not say what the first choice could not do: {:?}",
            lines[1]
        );
        assert!(
            reads_as(
                lines.last().expect("three lines"),
                Message::SettingsEnginePending
            ),
            "{lines:?}"
        );
    }

    /// The bargain the MCP pane makes, kept on this page too: the last
    /// line of the banner says that Layer B is not here yet, in every
    /// state the page can be in.
    ///
    /// The failure it exists for is the one that ships: somebody adds a
    /// state, writes a cheerful first line for it, and a user reads
    /// "Configured, and the document would stay on this machine" as a
    /// promise that clicking Rewrite will do something. Nothing sends a
    /// request in this build, and every state has to say so.
    #[test]
    fn the_engine_banner_always_says_a_rewrite_is_not_here_yet() {
        for (settings, key) in every_engine_state() {
            let (_, _, lines) = banner_for(&settings, &key);
            assert_eq!(
                lines.len(),
                2,
                "{settings:?} with {key:?} is not two lines: what it would do, and that \
                 nothing does it yet"
            );
            assert!(
                reads_as(
                    lines.last().expect("at least one line"),
                    Message::SettingsEnginePending
                ),
                "{settings:?} with {key:?} does not say that nothing sends a document \
                 anywhere yet: {:?}",
                lines[1]
            );
            for line in &lines {
                assert!(!line.is_empty(), "{settings:?} produced an empty line");
            }
        }
    }

    /// The MCP pane's bargain: whatever the socket is doing, the last
    /// line says what the tools do — they clean, and nothing rewrites.
    /// A state added with a cheerful first line and no second one would
    /// let a user read "Running" as "the tools rewrite".
    #[test]
    fn the_mcp_banner_always_says_what_the_tools_do() {
        let here = |port| Endpoint {
            bind: crate::mcp::BindAddress::LOOPBACK,
            port,
        };
        for (status, count, tone) in [
            (Status::Off, 2, Tone::Quiet),
            (Status::Starting, 2, Tone::Quiet),
            (
                Status::Listening {
                    endpoint: here(5056),
                    wanted: 5056,
                },
                2,
                Tone::Good,
            ),
            (
                Status::Listening {
                    endpoint: here(5057),
                    wanted: 5056,
                },
                3,
                Tone::Good,
            ),
            (
                Status::Failed("Address already in use".to_owned()),
                2,
                Tone::Bad,
            ),
        ] {
            let (_, said, lines) = mcp_banner(&status);
            assert_eq!(lines.len(), count, "{status:?}: {lines:?}");
            assert_eq!(said, tone, "{status:?}");
            assert!(
                reads_as(
                    lines.last().expect("at least one line"),
                    Message::SettingsMcpToolsLayerA
                ),
                "{status:?} does not end by saying what the tools do: {lines:?}"
            );
            for line in &lines {
                assert!(!line.is_empty(), "{status:?} produced an empty line");
            }
        }
    }

    /// One state of the Models page, as the banner sees it.
    #[derive(Debug, Clone)]
    struct ModelsState {
        host: Option<Host>,
        scanned: bool,
        folder: Folder,
        said: Option<Message>,
        why: Option<&'static str>,
    }

    /// The folder half of a state, borrowed for one call.
    fn shelf<'a>(state: &'a ModelsState) -> Shelf<'a> {
        Shelf {
            dir: Path::new("/data/models"),
            installed: 1,
            folder: &state.folder,
        }
    }

    /// Every state this page can be in, for the gates below.
    fn every_models_state() -> Vec<ModelsState> {
        let folders = [
            Folder::Unread,
            Folder::Missing,
            Folder::Unreadable("permission denied".into()),
            Folder::Read { others: Vec::new() },
            Folder::Read {
                others: vec![wipemark_models::scan::Found {
                    path: PathBuf::from("/data/models/elsewhere/other.gguf"),
                    bytes: 4_000_000_000,
                    format: wipemark_models::manifest::Format::Gguf,
                }],
            },
        ];
        let machines = [
            None,
            Some(Host {
                total_ram_mb: 0,
                available_ram_mb: 0,
                vram_mb: None,
                unified_memory: false,
            }),
            Some(Host {
                total_ram_mb: 16_384,
                available_ram_mb: 4_096,
                vram_mb: Some(16_384),
                unified_memory: true,
            }),
            Some(Host {
                total_ram_mb: 131_072,
                available_ram_mb: 65_536,
                vram_mb: Some(24_564),
                unified_memory: false,
            }),
        ];
        let outcomes = [
            (None, None),
            (Some(Message::SettingsModelsStopped), None),
            (Some(Message::SettingsModelsFailed), Some("HTTP 503")),
        ];
        let mut states = Vec::new();
        for host in machines {
            for scanned in [false, true] {
                for folder in &folders {
                    for (said, why) in outcomes {
                        states.push(ModelsState {
                            host,
                            scanned,
                            folder: folder.clone(),
                            said,
                            why,
                        });
                    }
                }
            }
        }
        states
    }

    /// The same promise the Engine banner makes, on the page where it
    /// costs the most to break: a user can spend seven gigabytes of
    /// disk and an hour of bandwidth here, and every state has to say
    /// that nothing loads the file yet.
    #[test]
    fn the_models_banner_always_says_the_weights_are_not_used_yet() {
        for state in every_models_state() {
            let (_, lines) = models_banner(
                state.host,
                state.scanned,
                shelf(&state),
                state.said,
                state.why,
            );
            let last = lines.last().expect("a banner is never empty");
            assert!(
                reads_as(last, Message::SettingsModelsPending),
                "{state:?} does not say the weights are not used yet: {last:?}"
            );
            for line in &lines {
                assert!(!line.is_empty(), "{state:?} produced an empty line");
            }
        }
    }

    /// The banner names the folder the moment it has been read, in
    /// every state it can be in — and never before, because a sentence
    /// about a folder nobody has read is a guess.
    #[test]
    fn the_banner_names_the_folder_once_it_has_been_read() {
        for state in every_models_state() {
            let (_, lines) = models_banner(
                state.host,
                state.scanned,
                shelf(&state),
                state.said,
                state.why,
            );
            let names_it = lines.iter().any(|line| line.contains("/data/models"));
            assert_eq!(
                names_it,
                state.folder != Folder::Unread,
                "{state:?}: {lines:?}"
            );
        }
    }

    /// A folder that cannot be read is something the user has to fix;
    /// one that does not exist yet is the ordinary state on a fresh
    /// install, and painting it as a fault would send every first-time
    /// user looking for a problem that is not there.
    #[test]
    fn an_unreadable_folder_is_a_warning_and_a_missing_one_is_not() {
        for state in every_models_state() {
            if state.why.is_some() {
                continue;
            }
            let (tone, _) = models_banner(
                state.host,
                state.scanned,
                shelf(&state),
                state.said,
                state.why,
            );
            assert_eq!(
                tone == Tone::Warn,
                state.folder.is_a_fault(),
                "{:?} is the wrong tone",
                state.folder
            );
        }
    }

    /// A download that failed is a warning; one the user stopped is
    /// not. Stopping keeps what was downloaded and the next attempt
    /// carries on from it, and colouring that as a fault would teach
    /// people to be afraid of the button.
    #[test]
    fn stopping_a_download_is_not_reported_as_a_fault() {
        for state in every_models_state() {
            // The folder's own fault is the other test's business.
            if state.folder.is_a_fault() {
                continue;
            }
            let (tone, _) = models_banner(
                state.host,
                state.scanned,
                shelf(&state),
                state.said,
                state.why,
            );
            assert_eq!(
                tone == Tone::Warn,
                state.why.is_some(),
                "{:?} with {:?} is the wrong tone",
                state.said,
                state.why
            );
        }
    }

    /// A machine nothing has measured is never told a model will not
    /// work on it — and "still looking" is not the same sentence as
    /// "could not read this machine", because one of them will change
    /// on its own and the other will not.
    #[test]
    fn a_machine_that_has_not_been_read_is_not_a_machine_that_refused() {
        let roomy = Host {
            total_ram_mb: 131_072,
            available_ram_mb: 65_536,
            vram_mb: None,
            unified_memory: false,
        };
        let unread = Shelf {
            dir: Path::new("/data/models"),
            installed: 0,
            folder: &Folder::Unread,
        };
        let (_, looking) = models_banner(None, false, unread, None, None);
        let (_, unreadable) = models_banner(None, true, unread, None, None);
        let (_, read) = models_banner(Some(roomy), true, unread, None, None);
        assert_ne!(looking[0], unreadable[0]);
        assert_ne!(unreadable[0], read[0]);
        assert!(read[0].contains("131072"), "{:?}", read[0]);
    }

    /// A page that reported "the document leaves this machine" in the
    /// same grey as "no engine configured" would be one where the
    /// difference has to be read to be noticed. It is the one thing on
    /// this page a user must not have to read carefully.
    #[test]
    fn a_configuration_that_sends_the_document_away_is_never_a_quiet_notice() {
        for (settings, key) in every_engine_state() {
            let (_, tone, _) = banner_for(&settings, &key);
            // Still reading the credential store is not yet "would
            // send": `engine::refusal` answers `None` for it because it
            // has nothing to refuse *yet*, and the banner says so in
            // the same breath.
            let looking = settings.provider.takes_a_key() && key == KeyState::Unknown;
            let would_send = !looking
                && engine::refusal(&settings, &key).is_none()
                && settings.base_url.leaves_this_machine();
            if would_send {
                assert_ne!(
                    tone,
                    Tone::Quiet,
                    "{settings:?} would send the document off this machine quietly"
                );
                assert_ne!(tone, Tone::Good, "{settings:?} called that a good outcome");
            }
        }
    }

    /// A credential problem is the loudest thing this page has to say,
    /// and it is the one a user has to act on before anything works.
    #[test]
    fn a_key_that_could_only_travel_in_the_clear_is_reported_at_the_top() {
        let settings = EngineSettings {
            provider: Provider::OpenAiCompatible,
            base_url: BaseUrl::parse("http://gateway.example.com:8000").expect("a base URL"),
            model: "some-model".to_owned(),
            allow_remote: true,
            ..EngineSettings::default()
        };
        let (glyph, tone, lines) = banner_for(&settings, &KeyState::Stored);
        assert_eq!(tone, Tone::Bad);
        assert_eq!(glyph.id(), IconName::TriangleExclamation.id());
        assert!(
            lines[0].contains("gateway.example.com"),
            "the banner did not name the endpoint it is refusing: {:?}",
            lines[0]
        );
    }

    /// A fresh install opens on a page that says the product works
    /// without any of this.
    #[test]
    fn a_first_launch_reads_as_a_choice_rather_than_a_fault() {
        let (glyph, tone, lines) = banner_for(&EngineSettings::default(), &KeyState::Unknown);
        assert_eq!(tone, Tone::Quiet, "a fresh install is not a warning");
        assert_eq!(glyph.id(), IconName::CircleInfo.id());
        assert_ne!(
            lines[0], lines[1],
            "the first line said the same thing as the second"
        );
    }

    /// Every state the Engine page can be in, for the gates above.
    ///
    /// Built by combination rather than by hand: the point of these
    /// tests is the state somebody forgets, and a list written out by
    /// the same person who wrote the match would forget the same one.
    fn every_engine_state() -> Vec<(EngineSettings, KeyState)> {
        let endpoints = [
            "http://127.0.0.1:11434",
            "https://api.openai.com",
            "http://gateway.example.com:8000",
        ];
        let keys = [
            KeyState::Unknown,
            KeyState::Absent,
            KeyState::Stored,
            KeyState::Failed("the keychain is locked".to_owned()),
        ];

        let mut states = Vec::new();
        for provider in Provider::ALL {
            for endpoint in endpoints {
                for model in ["", "some-model"] {
                    for allow_remote in [false, true] {
                        for key in &keys {
                            states.push((
                                EngineSettings {
                                    provider,
                                    base_url: BaseUrl::parse(endpoint).expect("a base URL"),
                                    model: model.to_owned(),
                                    allow_remote,
                                    ..EngineSettings::default()
                                },
                                key.clone(),
                            ));
                        }
                    }
                }
            }
        }
        states
    }

    /// The name the command line uses is the id, not the label, so
    /// `--settings=general` means the same thing on a German desktop.
    #[test]
    fn a_section_is_named_on_the_command_line_by_its_id() {
        for section in Section::ALL {
            assert_eq!(Section::parse(section.id()), Some(section));
            assert_eq!(Section::parse(&section.id().to_uppercase()), Some(section));
            assert_eq!(
                Section::parse(&format!("  {}  ", section.id())),
                Some(section)
            );
        }
        assert_eq!(Section::parse("licensing"), None);
        assert_eq!(Section::parse(""), None);
        assert_eq!(Section::parse("Genral"), None, "a typo is not guessed at");
        for section in Section::ALL {
            assert!(
                Section::names().contains(section.id()),
                "{section:?} is missing from the list a diagnostic prints"
            );
        }
    }

    /// Two sidebar entries reading the same word is a window with no
    /// way to tell where you are.
    #[test]
    fn no_two_sections_read_the_same() {
        let names: BTreeSet<String> = Section::ALL.iter().map(|s| t(s.title())).collect();
        assert_eq!(names.len(), Section::ALL.len());
        let ids: BTreeSet<&str> = Section::ALL.iter().map(|s| s.id()).collect();
        assert_eq!(ids.len(), Section::ALL.len());
    }

    /// A window is centred by what the user sees — its frame — and the
    /// frame is a titlebar taller than the rectangle GPUI is handed.
    /// Centring the content instead is off by half a titlebar, every
    /// time, on every screen.
    /// Whether this line is `message`, in any language this build
    /// ships.
    ///
    /// The banners are rendered through the process-wide language, and
    /// one test in this suite moves that language while the others run
    /// — so comparing against a single rendering is a race that fails
    /// once in a hundred runs and never on the machine that wrote it.
    /// Comparing against the set is exact, cannot race, and says what
    /// these gates actually mean: *this* sentence, whichever language
    /// the page happens to be in.
    ///
    /// It replaced looking for `"E2"` in the line, which was
    /// language-independent for the wrong reason — an epic number is
    /// not something a reader can act on, and none of them is in a
    /// window any more.
    fn reads_as(line: &str, message: Message) -> bool {
        wipemark_i18n::available_languages().iter().any(|language| {
            wipemark_i18n::Localizer::for_languages(
                std::slice::from_ref(&language.id),
                wipemark_i18n::Rendering::Ui,
            )
            .format(message)
                == line
        })
    }

    /// The local-model block says what is in memory: the measured figure
    /// when there is one and no figure at all when there is not, the file
    /// beside it, the refusal's own sentence with llama.cpp's words under
    /// it — and after an unload, that "keep loaded" is still the answer.
    #[test]
    fn the_local_block_shows_memory_only_when_it_was_measured() {
        let model = Some("Qwen3 4B Instruct");
        let measured = Loaded::Yes {
            since: std::time::Instant::now(),
            resident_mb: Some(4_000),
        };
        let unmeasured = Loaded::Yes {
            since: std::time::Instant::now(),
            resident_mb: None,
        };
        let (tone, lines) = local_status(
            true,
            model,
            &measured,
            Some("14:05"),
            Keeping::OnDemand,
            Some(2_546_340_960),
        );
        assert_eq!(tone, Tone::Good);
        assert!(
            lines[0].contains(&engine_host::memory_label(4_000)),
            "{lines:?}"
        );
        assert!(lines[0].contains("14:05"), "{lines:?}");
        assert!(
            lines[1].contains(&models::bytes_label(2_546_340_960)),
            "{lines:?}"
        );

        let (_, lines) = local_status(
            true,
            model,
            &unmeasured,
            Some("14:05"),
            Keeping::OnDemand,
            None,
        );
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(
            !lines[0].contains(&engine_host::memory_label(0)),
            "an unread figure was shown as a number: {lines:?}"
        );

        let (tone, lines) = local_status(
            true,
            model,
            &Loaded::Failed(wipemark_engine::Unavailable::LoadFailed {
                detail: "llama_model_load_from_file returned null".to_owned(),
            }),
            None,
            Keeping::OnDemand,
            None,
        );
        assert_eq!(tone, Tone::Warn);
        assert_eq!(lines[1], "llama_model_load_from_file returned null");

        let (_, resident) = local_status(true, model, &Loaded::No, None, Keeping::Resident, None);
        let (_, on_demand) = local_status(true, model, &Loaded::No, None, Keeping::OnDemand, None);
        assert_eq!(
            resident.len(),
            2,
            "unloaded under keep-loaded says it comes back"
        );
        assert_eq!(on_demand.len(), 1);

        // Not on duty here: one quiet line, and no model named.
        let (tone, lines) = local_status(false, model, &measured, None, Keeping::OnDemand, None);
        assert_eq!(tone, Tone::Quiet);
        assert!(!lines[0].contains("Qwen3"), "{lines:?}");
    }

    /// A check's answer is the model's words and its timing; nothing in
    /// what the block says about it calls it a rewrite.
    #[test]
    fn a_check_reports_what_it_measured() {
        assert!(check_lines(&Check::Idle).is_empty());
        let lines = check_lines(&Check::Done(CheckOutcome::Answered {
            load_ms: Some(2_400),
            tokens: 3,
            per_second: Some(10.4),
            text: "ready".to_owned(),
        }));
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert!(lines[0].contains("ready"), "{lines:?}");
        assert!(lines[1].contains("2.4"), "{lines:?}");
        assert!(lines[2].contains("10.4"), "{lines:?}");
        let already = check_lines(&Check::Done(CheckOutcome::Answered {
            load_ms: None,
            tokens: 1,
            per_second: None,
            text: "ready".to_owned(),
        }));
        assert_eq!(
            already.len(),
            2,
            "no load line for a model already loaded: {already:?}"
        );
    }

    /// An endpoint's check is timed from the request to its first piece —
    /// there is nothing to load — and counts pieces, which is what a stream
    /// is made of whatever the server calls a token.
    #[test]
    fn an_endpoint_check_reports_its_first_piece_and_its_pieces() {
        let lines = check_lines(&Check::Done(CheckOutcome::EndpointAnswered {
            first_ms: Some(1_250),
            pieces: 16,
            per_second: Some(12.5),
            text: "one two three".to_owned(),
        }));
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert!(lines[0].contains("one two three"), "{lines:?}");
        assert!(
            lines[1].contains("1.2") || lines[1].contains("1.3"),
            "{lines:?}"
        );
        assert!(
            lines[2].contains("16") && lines[2].contains("12.5"),
            "{lines:?}"
        );
        let quick = check_lines(&Check::Done(CheckOutcome::EndpointAnswered {
            first_ms: None,
            pieces: 1,
            per_second: None,
            text: "one".to_owned(),
        }));
        assert_eq!(quick.len(), 2, "{quick:?}");
    }

    /// The block between the banner and the rows is a state, not a row, and
    /// no row is titled like it: the endpoint's URL field once read "The
    /// endpoint", the block's title, instead of its own.
    #[test]
    fn no_row_is_titled_like_the_block_above_the_rows() {
        for setting in Setting::ALL {
            for block in [
                Message::SettingsEngineRemoteTitle,
                Message::SettingsEngineLocalTitle,
            ] {
                assert_ne!(setting.title(), block, "{setting:?}");
            }
        }
    }

    /// The check's note says what a check is — and, for an endpoint that is
    /// not this machine, that its prompt went there. A fixed sentence and
    /// not a document, but it left this machine, and the page says so.
    #[test]
    fn the_check_on_an_endpoint_says_where_its_prompt_went() {
        let remote = |on_this_machine: bool, origin: &str| {
            Performer::Endpoint(duty::Remote {
                profile: None,
                provider: Provider::OpenAiCompatible,
                endpoint: format!("{origin}/v1/chat/completions"),
                origin: origin.to_owned(),
                model: "gpt-4o-mini".to_owned(),
                temperature: 0.9,
                reasoning: ReasoningEffort::None,
                timeout: 120,
                account: Some(origin.to_owned()),
                on_this_machine,
            })
        };

        let away = check_note(Some(&remote(false, "https://api.openai.com")));
        assert_eq!(away.len(), 2, "{away:?}");
        assert!(
            reads_as(&away[0], Message::SettingsEngineRemoteCheckNote),
            "{away:?}"
        );
        assert!(away[1].contains("https://api.openai.com"), "{away:?}");

        let here = check_note(Some(&remote(true, "http://127.0.0.1:8089")));
        assert_eq!(here.len(), 1, "nothing left this machine: {here:?}");
        assert!(reads_as(&here[0], Message::SettingsEngineRemoteCheckNote));

        let machine = check_note(None);
        assert_eq!(machine.len(), 1);
        assert!(reads_as(&machine[0], Message::SettingsEngineLocalCheckNote));
    }

    /// The Placement page's banner keeps the same bargain the Engine,
    /// Models and MCP banners do: whatever else it says, it says which
    /// windows these choices actually place today. Until E7 that is one
    /// window, and a page that implied the workspace was already
    /// arranged by it would be describing a product that does not
    /// exist.
    #[test]
    fn the_placement_banner_always_says_which_windows_it_places() {
        for (attached, scanned) in [(0, false), (1, true), (3, true)] {
            let lines = screens_banner(attached, scanned);
            assert!(
                lines
                    .last()
                    .is_some_and(|last| *last == t(Message::SettingsPlacementOnlyWindow)),
                "{attached} displays, scanned {scanned}: {lines:?}"
            );
            assert_eq!(lines.len(), 2, "a banner is a state and a standing fact");
        }
    }

    /// Every state the Retention page can be in: three destinations,
    /// four settings of the two switches, five periods.
    fn every_retention_state() -> Vec<Retention> {
        let mut states = Vec::new();
        for destination in Destination::ALL {
            for (keep_originals, keep_results) in
                [(false, false), (true, false), (false, true), (true, true)]
            {
                for keep_for in Period::ALL {
                    states.push(Retention {
                        destination,
                        folder: None,
                        keep_originals,
                        keep_results,
                        keep_for,
                    });
                }
            }
        }
        states
    }

    /// The Retention page's banner keeps the bargain the other four
    /// keep: whatever else it says, it says that nothing is written
    /// yet. A page that described results landing beside files, over a
    /// product with no scrubber in it, would be describing a product
    /// that does not exist.
    #[test]
    fn the_retention_banner_always_says_nothing_is_written_yet() {
        let results = Path::new("/Users/someone/Downloads");
        let kept = Path::new("/Users/someone/Library/Application Support/wipemark/kept");
        for retention in every_retention_state() {
            let lines = retention_banner(&retention, results, kept);
            assert_eq!(lines.len(), 3, "{retention:?}: {lines:?}");
            assert!(
                reads_as(&lines[2], Message::SettingsRetentionPending),
                "{retention:?} stopped saying that nothing is written yet: {lines:?}"
            );
            for line in &lines {
                assert!(!line.trim().is_empty(), "{retention:?}: an empty line");
            }
        }
    }

    /// The banner's first line says where results go and its second
    /// what is kept — and each moves only with its own rows. Three
    /// destinations read as three sentences; the two switches read as
    /// four; and with both switches off the period is not mentioned,
    /// because a period for copies nobody asked for is a sentence about
    /// nothing.
    #[test]
    fn the_retention_banner_says_what_each_row_changes() {
        let results = Path::new("/Users/someone/Downloads");
        let kept = Path::new("/kept");
        let firsts: BTreeSet<String> = Destination::ALL
            .into_iter()
            .map(|destination| {
                let retention = Retention {
                    destination,
                    ..Retention::default()
                };
                retention_banner(&retention, results, kept).remove(0)
            })
            .collect();
        assert_eq!(
            firsts.len(),
            Destination::ALL.len(),
            "two destinations read the same"
        );

        let seconds: BTreeSet<String> =
            [(false, false), (true, false), (false, true), (true, true)]
                .into_iter()
                .map(|(keep_originals, keep_results)| {
                    let retention = Retention {
                        keep_originals,
                        keep_results,
                        ..Retention::default()
                    };
                    retention_banner(&retention, results, kept).remove(1)
                })
                .collect();
        assert_eq!(
            seconds.len(),
            4,
            "two settings of the switches read the same"
        );

        // The period changes the sentence only while something is kept:
        // with both switches off, every period reads as the one line
        // that mentions no period. Checked against the message rather
        // than by counting distinct renderings, because a test in this
        // suite moves the process-wide language while this one runs
        // and one sentence in two languages would count as two.
        for keep_for in Period::ALL {
            let retention = Retention {
                keep_for,
                ..Retention::default()
            };
            let line = retention_banner(&retention, results, kept).remove(1);
            assert!(
                reads_as(&line, Message::SettingsRetentionKeepsNothing),
                "{keep_for:?}: the period is mentioned while nothing is kept: {line}"
            );
        }
        let on: BTreeSet<String> = Period::ALL
            .into_iter()
            .map(|keep_for| {
                let retention = Retention {
                    keep_originals: true,
                    keep_for,
                    ..Retention::default()
                };
                retention_banner(&retention, results, kept).remove(1)
            })
            .collect();
        assert_eq!(on.len(), Period::ALL.len(), "two periods read the same");

        // The folder the results go into is named, and only when they
        // go into one.
        let into = Retention {
            destination: Destination::Folder,
            ..Retention::default()
        };
        assert!(retention_banner(&into, results, kept)[0].contains("Downloads"));
        assert!(!retention_banner(&Retention::default(), results, kept)[0].contains("Downloads"));
    }

    /// Reading the displays is not the same as having none, and the
    /// page has to be able to say so: an empty list at startup lasts a
    /// frame or two, and "no displays attached" is not a state a
    /// desktop is ever in.
    #[test]
    fn a_machine_that_has_not_been_read_is_not_a_machine_with_no_screens() {
        assert_ne!(screens_banner(0, false)[0], screens_banner(0, true)[0]);
    }

    #[test]
    fn a_window_is_centred_by_its_frame_and_not_its_content() {
        let over = Bounds::new(point(px(0.0), px(0.0)), size(px(1000.0), px(800.0)));
        let content = size(px(400.0), px(300.0));
        let chrome = px(30.0);

        let frame = placement::frame_for(
            None,
            None,
            placement::frame_of_size(content, chrome),
            Some(over),
            over,
        );
        assert_eq!(
            frame.origin.y.as_f32() + frame.size.height.as_f32() / 2.0,
            400.0,
            "the frame's centre has to land on the centre of what it was centred over"
        );
        assert_eq!(
            frame.origin.x.as_f32() + frame.size.width.as_f32() / 2.0,
            500.0
        );
    }

    /// The two conversions are each other's inverse, or the window
    /// grows or shrinks by a titlebar every time it is reopened — which
    /// is exactly the bug `geometry_of` documents, seen from the other
    /// side.
    #[test]
    fn the_frame_and_the_content_round_trip() {
        let content = Bounds::new(point(px(10.0), px(20.0)), size(px(560.0), px(340.0)));
        assert_eq!(
            placement::content_of(placement::frame_of(content, px(33.0)), px(33.0)),
            content
        );
    }

    /// The window this build opens is wider than the window the last
    /// one did, and every existing install has a row saying otherwise.
    /// Restoring it verbatim opens a window the platform then refuses
    /// to make that small — leaving a sidebar and a sliver.
    #[test]
    fn a_rectangle_remembered_by_a_narrower_build_is_grown_not_discarded() {
        let remembered = Bounds::new(point(px(410.0), px(620.0)), size(px(560.0), px(340.0)));
        let grown = placement::at_least(remembered, MIN_SIZE);

        assert_eq!(grown.origin, remembered.origin, "the window moved");
        assert_eq!(grown.size, MIN_SIZE);
    }

    /// And a rectangle that is already big enough is left exactly
    /// alone — the clamp is a floor, not a resize.
    #[test]
    fn a_rectangle_that_already_fits_is_left_alone() {
        let remembered = Bounds::new(point(px(10.0), px(20.0)), size(px(900.0), px(700.0)));
        assert_eq!(placement::at_least(remembered, MIN_SIZE), remembered);
    }

    /// The failure this catches is the one the port scan introduces: a
    /// server that stepped past a taken port, and a page that went on
    /// describing the port it asked for. Everything below the banner is
    /// built from this — the endpoint line, and the four snippets — so
    /// getting it wrong hands the user a configuration that dials a
    /// port nothing is listening on, which is worse than not starting
    /// at all.
    #[test]
    fn the_page_describes_the_port_the_server_actually_took() {
        let asked_for = Endpoint {
            bind: BindAddress::LOOPBACK,
            port: 5056,
        };
        let landed = Endpoint {
            bind: BindAddress::LOOPBACK,
            port: 5057,
        };
        let status = Status::Listening {
            endpoint: landed,
            wanted: asked_for.port,
        };

        assert_eq!(on_screen(&status, asked_for), landed);
        assert_eq!(
            on_screen(&status, asked_for).url(),
            "http://127.0.0.1:5057/mcp"
        );
    }

    /// And with nothing listening there is no such thing as where it
    /// actually is, so the page describes what was asked for — which is
    /// what the snippet will be true of the moment the switch goes on.
    #[test]
    fn a_server_that_is_not_running_is_described_by_what_was_asked_for() {
        let asked_for = Endpoint {
            bind: BindAddress::EVERYWHERE,
            port: 7777,
        };
        for status in [
            Status::Off,
            Status::Starting,
            Status::Failed("address already in use".to_owned()),
        ] {
            assert_eq!(
                on_screen(&status, asked_for),
                asked_for,
                "{status:?} described an endpoint nobody asked for"
            );
        }
    }

    /// The window has to fit on the smallest screen it can be asked to
    /// open on, or `screen::contained` shrinks it below the minimum the
    /// platform will then refuse to honour.
    #[test]
    fn the_default_size_is_not_smaller_than_the_minimum() {
        assert!(DEFAULT_SIZE.width.as_f32() >= MIN_SIZE.width.as_f32());
        assert!(DEFAULT_SIZE.height.as_f32() >= MIN_SIZE.height.as_f32());
    }

    /// The sidebar keeps its width when the window is squeezed, so
    /// everything a narrow window costs comes out of the section beside
    /// it. The smallest one still has to fit a control column and a
    /// title next to it, or the titles start wrapping.
    #[test]
    fn the_smallest_window_still_has_room_for_a_row() {
        let beside = MIN_SIZE.width.as_f32() - SIDEBAR_WIDTH.as_f32();
        assert!(
            beside >= CONTROL_COLUMN.as_f32() * 2.0,
            "at the minimum width the control column would take {beside} points of the section"
        );
    }
}
