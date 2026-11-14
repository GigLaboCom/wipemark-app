//! The other GigLabo products: [`ProductsButton`] ("More from GigLabo") for
//! an application's toolbar, and [`ProductsList`], what the dialog it opens
//! shows, laid out as the site's Products page: the products as cards
//! under the site's groups, each with its icon, its name, one line and its
//! address, then the studio's card. A click on a card opens its site in the
//! browser.
//!
//! gpui-kit-grid's `crates/products`, ported from gpui-kit to gpui and
//! gpui-component (D836). Two things are the host's here that the original
//! did itself:
//!
//! * **The dialog.** The original opened gpui-component's `Root` dialog
//!   (`open_products`). An application whose dialogs are elements of its
//!   own view's tree — Wipemark's, `apps/wipemark-app/src/dialog.rs` —
//!   puts [`ProductsList`] in one of its own, and the button's click
//!   ([`ProductsButton::on_click`]) is what opens it.
//! * **The words.** Every word a person reads here but the products' own
//!   comes from the host, in its language: the button's label and
//!   [`Words`]. The products' names and taglines are `products.json`'s, in
//!   English. And the host names itself ([`ProductsList::with_current`]),
//!   so that it is not in its own list.
//!
//! ```ignore
//! h_flex()
//!     // the application's own toolbar …
//!     .child(ProductsButton::new(label).on_click(|_, window, cx| …))
//! ```
//!
//! The list is `products.json`, the entries of the site's Products page;
//! the icons are `icons/`, 128 px PNG files made from the same originals as
//! the site's. Both are data, kept out of the code, so that a list shared
//! by several applications can replace them.
//!
//! `GIGLABO_PRODUCTS=off` in the environment hides the button (stills and
//! recordings without it); the variable is read once, and only the exact
//! value `off` counts. [`set_hidden`] hides it from code.

use std::sync::{Arc, OnceLock};

use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, img, px, white, AnyElement, App, ClickEvent, FontWeight, Global, Image, ImageFormat,
    InteractiveElement as _, IntoElement, ParentElement as _, Pixels, RenderOnce, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::{h_flex, v_flex, ActiveTheme as _, Sizable, Size};
use serde::Deserialize;

/// A product's icon, the studio's logo and the button's, drawn from
/// 128 px PNG files.
const ICON_SIZE: f32 = 40.;
const LOGO_SIZE: f32 = 52.;
const BUTTON_LOGO_SIZE: f32 = 16.;

/// The products and the studio, as the site lists them.
const LIST: &str = include_str!("../products.json");

/// Each card's icon, by its `id` in `products.json`.
const ICONS: [(&str, &[u8]); 7] = [
    ("smbact", include_bytes!("../icons/smbact.png")),
    ("wipemark", include_bytes!("../icons/wipemark.png")),
    ("mnemovi", include_bytes!("../icons/mnemovi.png")),
    (
        "heretic-lazy-shot",
        include_bytes!("../icons/heretic-lazy-shot.png"),
    ),
    ("watchword", include_bytes!("../icons/watchword.png")),
    ("moat-atlas", include_bytes!("../icons/moat-atlas.png")),
    ("giglabo", include_bytes!("../icons/giglabo.png")),
];

/// The products' sections, in the site's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Group {
    Desktop,
    Ai,
    Web,
}

impl Group {
    const ALL: [Group; 3] = [Group::Desktop, Group::Ai, Group::Web];
}

/// `products.json`.
#[derive(Deserialize)]
struct Listing {
    products: Vec<ProductEntry>,
    studio: CardEntry,
}

/// A product: its group and its card.
#[derive(Deserialize)]
struct ProductEntry {
    group: Group,
    #[serde(flatten)]
    card: CardEntry,
}

/// What a card shows.
#[derive(Deserialize)]
struct CardEntry {
    id: String,
    name: String,
    url: String,
    tagline: String,
    /// A light plate behind an icon drawn for light backgrounds (a dark
    /// mark on transparent).
    #[serde(default)]
    plate: bool,
}

/// A card as the list draws it.
struct Card {
    /// Its `id` in `products.json`: what a host names itself by.
    id: SharedString,
    name: SharedString,
    url: SharedString,
    /// The address without its scheme (`smbact.com`; the studio's with its
    /// path, `giglabo.com/agency`, as the site shows it).
    address: SharedString,
    tagline: SharedString,
    icon: Arc<Image>,
    plate: bool,
}

/// The list's content, read once.
struct Products {
    products: Vec<(Group, Card)>,
    studio: Card,
}

fn products() -> &'static Products {
    static PRODUCTS: OnceLock<Products> = OnceLock::new();
    PRODUCTS.get_or_init(|| {
        let list: Listing = serde_json::from_str(LIST).expect("products.json lists the products");
        Products {
            products: list
                .products
                .into_iter()
                .map(|entry| (entry.group, Card::new(entry.card, host)))
                .collect(),
            studio: Card::new(list.studio, without_scheme),
        }
    })
}

impl Card {
    fn new(entry: CardEntry, address: fn(&str) -> &str) -> Self {
        Self {
            address: address(&entry.url).to_owned().into(),
            icon: icon(&entry.id),
            id: entry.id.into(),
            name: entry.name.into(),
            url: entry.url.into(),
            tagline: entry.tagline.into(),
            plate: entry.plate,
        }
    }
}

/// The icon `id` names, decoded once by whoever draws it.
fn icon(id: &str) -> Arc<Image> {
    ICONS
        .iter()
        .find(|(icon, _)| *icon == id)
        .map(|(_, png)| Arc::new(Image::from_bytes(ImageFormat::Png, png.to_vec())))
        .unwrap_or_else(|| panic!("no icon for `{id}`"))
}

/// `https://smbact.com/x` → `smbact.com/x`.
fn without_scheme(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    rest.trim_end_matches('/')
}

/// `https://smbact.com/x` → `smbact.com`.
fn host(url: &str) -> &str {
    let rest = without_scheme(url);
    rest.split('/').next().unwrap_or(rest)
}

/// The sections the list shows, in the site's order: each group with its
/// cards and their places in the whole list, the card `current` names left
/// out, and a group left with no card left out with it.
fn sections<'a>(
    products: &'a Products,
    current: Option<&str>,
) -> Vec<(Group, Vec<(usize, &'a Card)>)> {
    Group::ALL
        .into_iter()
        .map(|group| {
            let cards = products
                .products
                .iter()
                .enumerate()
                .filter(|(_, (of, card))| *of == group && Some(card.id.as_ref()) != current)
                .map(|(ix, (_, card))| (ix, card))
                .collect::<Vec<_>>();
            (group, cards)
        })
        .filter(|(_, cards)| !cards.is_empty())
        .collect()
}

/// Whether [`ProductsButton`] draws nothing.
#[derive(Default)]
struct Hidden(bool);

impl Global for Hidden {}

/// Hides [`ProductsButton`] in every window (or shows it again).
pub fn set_hidden(cx: &mut App, hidden: bool) {
    cx.set_global(Hidden(hidden));
}

/// Whether [`ProductsButton`] is hidden: by [`set_hidden`], or by
/// `GIGLABO_PRODUCTS=off` in the environment, which `set_hidden(cx, false)`
/// does not override.
pub fn is_hidden(cx: &App) -> bool {
    hidden_by_environment() || cx.try_global::<Hidden>().is_some_and(|hidden| hidden.0)
}

/// The environment variable that hides [`ProductsButton`] when it is `off`.
const ENVIRONMENT_VARIABLE: &str = "GIGLABO_PRODUCTS";

/// Whether the environment hides [`ProductsButton`], read once: the button
/// renders every frame.
fn hidden_by_environment() -> bool {
    static HIDDEN: OnceLock<bool> = OnceLock::new();
    *HIDDEN.get_or_init(|| hides(std::env::var(ENVIRONMENT_VARIABLE).ok().as_deref()))
}

/// Whether a value of `GIGLABO_PRODUCTS` hides the button: only exactly
/// `off`, the one spelling the tooling writes. `OFF`, ` off `, any other
/// value, a value that is not Unicode and no variable at all leave the
/// button shown.
fn hides(value: Option<&str>) -> bool {
    value == Some("off")
}

/// What a click on [`ProductsButton`] does, the host's.
type OnClick = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// The button's label beside the studio's logo, a button in the theme's
/// primary colour (outlined, so it never outweighs the window's own primary
/// action) whose click is the host's ([`ProductsButton::on_click`]: it
/// opens the host's dialog of [`ProductsList`]); nothing when hidden
/// ([`is_hidden`]). Small unless sized otherwise.
#[derive(IntoElement)]
pub struct ProductsButton {
    label: SharedString,
    size: Size,
    on_click: Option<OnClick>,
}

impl ProductsButton {
    /// The button, reading `label` — "More from GigLabo", in the host's
    /// language.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            size: Size::Small,
            on_click: None,
        }
    }

    /// What a click does: the host opens its dialog.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl Sizable for ProductsButton {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for ProductsButton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let shown = !is_hidden(cx);
        let Self {
            label,
            size,
            on_click,
        } = self;
        div().flex_none().when(shown, |this| {
            this.child(
                Button::new("giglabo-products")
                    .debug_selector(|| "giglabo-products".to_owned())
                    .with_size(size)
                    .primary()
                    .outline()
                    .child(
                        h_flex()
                            .gap_1p5()
                            .child(render_logo(
                                &products().studio,
                                BUTTON_LOGO_SIZE,
                                px(1.),
                                px(3.),
                            ))
                            .child(label),
                    )
                    .when_some(on_click, |button, handler| {
                        button.on_click(move |event, window, cx| handler(event, window, cx))
                    }),
            )
        })
    }
}

/// Every word [`ProductsList`] shows but the products' own, in the host's
/// language: the line over the cards, each group's heading and the
/// studio's. A word not given is left blank.
#[derive(Clone, Default)]
pub struct Words {
    intro: SharedString,
    desktop: SharedString,
    ai: SharedString,
    web: SharedString,
    studio: SharedString,
}

impl Words {
    pub fn new() -> Self {
        Self::default()
    }

    /// The line over the cards: what the host is, and what these are.
    pub fn with_intro(mut self, text: impl Into<SharedString>) -> Self {
        self.intro = text.into();
        self
    }

    /// The heading over the desktop applications.
    pub fn with_desktop(mut self, text: impl Into<SharedString>) -> Self {
        self.desktop = text.into();
        self
    }

    /// The heading over the AI tools.
    pub fn with_ai(mut self, text: impl Into<SharedString>) -> Self {
        self.ai = text.into();
        self
    }

    /// The heading over what runs on the web.
    pub fn with_web(mut self, text: impl Into<SharedString>) -> Self {
        self.web = text.into();
        self
    }

    /// The heading over the studio's card.
    pub fn with_studio(mut self, text: impl Into<SharedString>) -> Self {
        self.studio = text.into();
        self
    }

    fn group(&self, group: Group) -> SharedString {
        match group {
            Group::Desktop => self.desktop.clone(),
            Group::Ai => self.ai.clone(),
            Group::Web => self.web.clone(),
        }
    }
}

/// What the products' dialog shows under its title: the line over the
/// cards, the products' sections and the studio's card, as tall as they
/// are — the host's dialog scrolls it. Each card carries the debug selector
/// `giglabo-product-<id>`, its `id` in `products.json`.
#[derive(IntoElement)]
pub struct ProductsList {
    words: Words,
    current: Option<SharedString>,
}

impl ProductsList {
    pub fn new(words: Words) -> Self {
        Self {
            words,
            current: None,
        }
    }

    /// Leaves the card of `id` (its `id` in `products.json`) out of the
    /// list: an application does not offer itself.
    pub fn with_current(mut self, id: impl Into<SharedString>) -> Self {
        self.current = Some(id.into());
        self
    }
}

impl RenderOnce for ProductsList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let products = products();
        let words = self.words;
        v_flex()
            .child(
                div()
                    .flex_none()
                    .pb_2()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(words.intro.clone()),
            )
            .children(sections(products, self.current.as_deref()).into_iter().map(
                |(group, cards)| {
                    render_section(
                        words.group(group),
                        div().grid().grid_cols(2).gap_3().children(
                            cards
                                .into_iter()
                                .map(|(ix, card)| render_card(ix, card, cx)),
                        ),
                        cx,
                    )
                },
            ))
            .child(render_section(
                words.studio.clone(),
                render_studio(&products.studio, cx),
                cx,
            ))
    }
}

/// A section: its title over `content`.
fn render_section(title: SharedString, content: impl IntoElement, cx: &App) -> AnyElement {
    v_flex()
        .flex_none()
        .gap_2()
        .pt_3()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(cx.theme().foreground)
                .child(title),
        )
        .child(content)
        .into_any_element()
}

/// A product's card; a click opens its site.
fn render_card(ix: usize, card: &'static Card, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let hover = theme.list_hover;
    let url = card.url.clone();
    let id = card.id.clone();
    h_flex()
        .id(("giglabo-product", ix))
        .debug_selector(move || format!("giglabo-product-{id}"))
        .items_start()
        .gap_3()
        .p_3()
        .border_1()
        .border_color(theme.border)
        .rounded(theme.radius_lg)
        .cursor_pointer()
        .hover(move |this| this.bg(hover))
        .on_click(move |_, _, cx| cx.open_url(&url))
        .child(render_logo(card, ICON_SIZE, px(4.), theme.radius))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child(card.name.clone()),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(card.tagline.clone()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(card.address.clone()),
                ),
        )
}

/// The studio's card, as the site draws it: as wide as the list, in the
/// theme's primary colour.
fn render_studio(card: &'static Card, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let (normal, hover) = (theme.primary.opacity(0.1), theme.primary.opacity(0.18));
    let url = card.url.clone();
    h_flex()
        .id("giglabo-studio")
        .debug_selector(|| "giglabo-studio".to_owned())
        .gap_4()
        .px_4()
        .py_3()
        .border_1()
        .border_color(theme.primary)
        .rounded(theme.radius_lg)
        .bg(normal)
        .cursor_pointer()
        .hover(move |this| this.bg(hover))
        .on_click(move |_, _, cx| cx.open_url(&url))
        .child(render_logo(card, LOGO_SIZE, px(5.), theme.radius_lg))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child(card.name.clone()),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(card.tagline.clone()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(card.address.clone()),
                ),
        )
}

/// A card's icon, `size` square, on a white plate (`inset` wide) when it is
/// drawn for light backgrounds.
fn render_logo(card: &'static Card, size: f32, inset: Pixels, radius: Pixels) -> impl IntoElement {
    div()
        .flex_none()
        .size(px(size))
        .rounded(radius)
        .overflow_hidden()
        .when(card.plate, |this| this.bg(white()).p(inset))
        .child(img(card.icon.clone()).size_full())
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use gpui::{Context, Modifiers, Render, TestAppContext};

    use super::*;

    #[test]
    fn every_card_has_an_icon_and_every_icon_a_card() {
        let list: Listing = serde_json::from_str(LIST).unwrap();
        let ids: Vec<&str> = list
            .products
            .iter()
            .map(|entry| entry.card.id.as_str())
            .chain([list.studio.id.as_str()])
            .collect();
        assert_eq!(ids.len(), ICONS.len());
        for (id, _) in ICONS {
            assert!(ids.contains(&id), "the icon `{id}` names no card");
        }
        // Decodes the list and finds every card's icon.
        assert_eq!(products().products.len(), list.products.len());
    }

    #[test]
    fn every_group_has_products() {
        for group in Group::ALL {
            assert!(
                products().products.iter().any(|(of, _)| *of == group),
                "{group:?} is empty"
            );
        }
    }

    #[test]
    fn an_address_is_the_host_and_the_studio_s_its_path() {
        assert_eq!(host("https://smbact.com"), "smbact.com");
        assert_eq!(
            host("https://github.com/GigLaboCom/wipemark-app/releases/latest"),
            "github.com"
        );
        assert_eq!(host("moa.giglabo.com/x"), "moa.giglabo.com");
        assert_eq!(
            without_scheme("https://giglabo.com/agency/"),
            "giglabo.com/agency"
        );
        assert_eq!(products().studio.address, "giglabo.com/agency");
    }

    #[test]
    fn only_off_hides() {
        assert!(!hides(None));
        assert!(hides(Some("off")));
        for value in ["", "on", "0", "false", "OFF", "Off", " off ", "off\n"] {
            assert!(!hides(Some(value)), "{value:?} hides the button");
        }
    }

    #[test]
    fn links_are_https() {
        let products = products();
        for card in products
            .products
            .iter()
            .map(|(_, card)| card)
            .chain([&products.studio])
        {
            assert!(card.url.starts_with("https://"), "{}", card.url);
        }
    }

    /// The ids of every card `sections` lists for `current`, in order.
    fn listed(current: Option<&str>) -> Vec<String> {
        sections(products(), current)
            .into_iter()
            .flat_map(|(_, cards)| cards)
            .map(|(_, card)| card.id.to_string())
            .collect()
    }

    /// A host is in nobody's list but its own: whichever product it is,
    /// its card goes and every other stays; no host, or an id no card has,
    /// leaves the list whole.
    #[test]
    fn the_host_is_left_out_of_its_own_list() {
        let everyone = listed(None);
        assert_eq!(everyone.len(), products().products.len());
        for id in &everyone {
            let others: Vec<String> = everyone.iter().filter(|of| *of != id).cloned().collect();
            assert_eq!(listed(Some(id)), others, "the list of `{id}`");
        }
        assert_eq!(listed(Some("nobody")), everyone);
    }

    /// A group whose one product is the host has no heading over nothing:
    /// the section goes with its last card.
    #[test]
    fn a_group_left_with_no_card_is_not_shown() {
        for (group, card) in &products().products {
            let alone = products()
                .products
                .iter()
                .filter(|(of, _)| of == group)
                .count()
                == 1;
            let shown = sections(products(), Some(&card.id))
                .iter()
                .any(|(of, _)| of == group);
            assert_eq!(shown, !alone, "{group:?} without `{}`", card.id);
        }
    }

    /// A toolbar with the button in it, counting its clicks.
    struct Toolbar {
        clicks: Rc<Cell<usize>>,
    }

    impl Render for Toolbar {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let clicks = self.clicks.clone();
            div().size_full().child(
                ProductsButton::new("More from GigLabo")
                    .on_click(move |_, _, _| clicks.set(clicks.get() + 1)),
            )
        }
    }

    /// The button is painted, and its click is the host's; hidden, it is
    /// not painted at all, and shown again it is back.
    #[gpui::test]
    fn the_button_is_painted_unless_hidden(cx: &mut TestAppContext) {
        assert!(
            !hidden_by_environment(),
            "{ENVIRONMENT_VARIABLE}=off is set; this test needs the button shown"
        );
        cx.update(gpui_component::init);
        let clicks = Rc::new(Cell::new(0));
        let counted = clicks.clone();
        let (_, cx) = cx.add_window_view(move |_, _| Toolbar { clicks: counted });
        cx.run_until_parked();

        let button = cx
            .debug_bounds("giglabo-products")
            .expect("the button is painted");
        cx.simulate_click(button.center(), Modifiers::none());
        assert_eq!(clicks.get(), 1, "the click did not reach the host");

        for hidden in [true, false] {
            cx.update(|window, cx| {
                set_hidden(cx, hidden);
                window.refresh();
            });
            cx.run_until_parked();
            assert_eq!(
                cx.debug_bounds("giglabo-products").is_none(),
                hidden,
                "hidden: {hidden}"
            );
        }
    }

    /// The list as a host shows it, naming itself.
    struct Dialog;

    impl Render for Dialog {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                ProductsList::new(
                    Words::new()
                        .with_intro("Wipemark is one of the things built at GigLabo.")
                        .with_desktop("Desktop apps")
                        .with_ai("AI tools")
                        .with_web("On the web")
                        .with_studio("The studio"),
                )
                .with_current("wipemark"),
            )
        }
    }

    /// Painted, the list has every card but the host's and the studio's,
    /// and a click on a card opens that product's site.
    #[gpui::test]
    fn the_list_leaves_the_host_out_and_a_card_opens_its_site(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (_, window) = cx.add_window_view(|_, _| Dialog);
        window.run_until_parked();

        assert!(
            window.debug_bounds("giglabo-product-wipemark").is_none(),
            "the host is in its own list"
        );
        for id in [
            "giglabo-product-smbact",
            "giglabo-product-watchword",
            "giglabo-studio",
        ] {
            assert!(window.debug_bounds(id).is_some(), "{id} is not painted");
        }

        let card = window
            .debug_bounds("giglabo-product-smbact")
            .expect("SMBAct's card is painted");
        window.simulate_click(card.center(), Modifiers::none());
        assert_eq!(cx.opened_url().as_deref(), Some("https://smbact.com"));
    }
}
