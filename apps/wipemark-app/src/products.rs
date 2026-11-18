//! **More from GigLabo** (D836): the toolbar's button and the dialog it
//! opens — the studio's other products as cards, a click on one opening its
//! site in the browser.
//!
//! The button and the cards are `giglabo-products`' (gpui-kit-grid's crate
//! of that name, with the list and the icons as its data); what this module
//! adds is what makes them Wipemark's:
//!
//! * **its words**: the button's label and tooltip, the dialog's title, the
//!   line over the cards, the sections' headings and Close come from the
//!   catalogue, in every language; the products' names and their one-line
//!   descriptions are the list's, in English;
//! * **its tooltip**, attached through [`Tip`] like every other toolbar
//!   button's, never gpui-component's `Button::tooltip` (D796);
//! * **Wipemark not in its own list** ([`CURRENT`]);
//! * **the dialog**: an element in the main window's own tree, with
//!   `dialog.rs`'s walls — never the component's `Root` dialog, which the
//!   crate's original opens (`dialog.rs` says why).
//!
//! `GIGLABO_PRODUCTS=off` in the environment hides the button, for the
//! screenshots and recordings that should not show it.

use giglabo_products::{ProductsButton, ProductsList, Words};
use gpui::prelude::*;
use gpui::{
    div, px, relative, App, ClickEvent, Context, EventEmitter, FocusHandle, Focusable,
    SharedString, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::{h_flex, v_flex, ActiveTheme, Sizable as _, StyledExt as _};
use wipemark_i18n::{t, Message};

use crate::dialog::{self, Accept, Answer, Dismiss, Next, Previous};
use crate::hover::Tip;

/// Wipemark's `id` in the crate's `products.json`: the card left out of
/// the list.
const CURRENT: &str = "wipemark";

/// The dialog's size; a smaller window shrinks it, and the cards scroll.
const WIDTH: f32 = 680.0;
const HEIGHT: f32 = 640.0;

/// The toolbar's button, in the catalogue's words, with its tooltip
/// attached as every other toolbar button's is ([`Tip`], D796); the toolbar
/// says what a click does.
pub fn button() -> ProductsButton {
    ProductsButton::new(SharedString::from(t(Message::ProductsButton)))
        .with_button(|button| button.tip(SharedString::from(t(Message::ProductsButtonTooltip))))
        .small()
}

/// Every word of the cards' dialog that is the catalogue's.
fn words() -> Words {
    Words::new()
        .with_intro(t(Message::ProductsIntro))
        .with_desktop(t(Message::ProductsDesktop))
        .with_ai(t(Message::ProductsAi))
        .with_web(t(Message::ProductsWeb))
        .with_studio(t(Message::ProductsStudio))
}

/// The dialog: the title, the cards and Close. It answers nothing but
/// [`Answer::Dismissed`], once — Escape, Enter, the backdrop and Close
/// are four roads to it.
pub struct ProductsDialog {
    focus: FocusHandle,
    armed: bool,
}

impl EventEmitter<Answer> for ProductsDialog {}

impl ProductsDialog {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        Self { focus, armed: true }
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        if !self.armed {
            return;
        }
        self.armed = false;
        cx.emit(Answer::Dismissed);
    }
}

impl Focusable for ProductsDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for ProductsDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        // The same walls as every dialog's — see `dialog.rs`.
        dialog::backdrop(cx)
            .id("products-backdrop")
            .on_action(cx.listener(|view, _: &Next, window, cx| {
                cx.stop_propagation();
                dialog::hold(&view.focus, window, cx);
            }))
            .on_action(cx.listener(|view, _: &Previous, window, cx| {
                cx.stop_propagation();
                dialog::hold(&view.focus, window, cx);
            }))
            .on_action(cx.listener(|view, _: &Accept, _, cx| {
                cx.stop_propagation();
                view.close(cx);
            }))
            .on_action(cx.listener(|view, _: &Dismiss, _, cx| {
                cx.stop_propagation();
                view.close(cx);
            }))
            .on_click(cx.listener(|view, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                view.close(cx);
            }))
            .child(
                v_flex()
                    .id("products-panel")
                    .debug_selector(|| "products-panel".to_owned())
                    .track_focus(&self.focus)
                    .key_context(dialog::CONTEXT)
                    .flex_none()
                    .gap_3()
                    .w(px(WIDTH))
                    .max_w(relative(0.9))
                    .h(px(HEIGHT))
                    .max_h(relative(0.9))
                    .p_4()
                    .rounded(theme.radius)
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.popover)
                    .text_color(theme.popover_foreground)
                    .shadow_lg()
                    .on_click(|_: &ClickEvent, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(theme.foreground)
                            .child(SharedString::from(t(Message::ProductsTitle))),
                    )
                    .child(
                        div()
                            .id("products-body")
                            .flex_1()
                            .min_h(px(0.0))
                            .overflow_y_scroll()
                            .child(ProductsList::new(words()).with_current(CURRENT)),
                    )
                    .child(
                        h_flex().w_full().justify_end().child(
                            Button::new("products-close")
                                .small()
                                .ghost()
                                .label(SharedString::from(t(Message::ProductsClose)))
                                .on_click(cx.listener(|view, _, _, cx| view.close(cx))),
                        ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;

    use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext};

    use super::*;

    /// The dialog, alone in a window.
    fn opened(cx: &mut TestAppContext) -> (Entity<ProductsDialog>, &mut VisualTestContext) {
        cx.update(gpui_component::init);
        let slot: Rc<RefCell<Option<Entity<ProductsDialog>>>> = Rc::default();
        let held = slot.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| ProductsDialog::new(window, cx));
            *held.borrow_mut() = Some(view.clone());
            gpui_component::Root::new(view, window, cx)
        });
        cx.run_until_parked();
        let view = slot.take().expect("the window builder ran");
        (view, cx)
    }

    /// Painted, the dialog shows every product but Wipemark, and the
    /// studio: [`CURRENT`] is Wipemark's id in the list. Misspell it and
    /// Wipemark offers itself — red.
    #[gpui::test]
    fn the_dialog_lists_every_product_but_wipemark(cx: &mut TestAppContext) {
        let (_, cx) = opened(cx);
        assert!(cx.debug_bounds("products-panel").is_some());
        assert!(
            cx.debug_bounds("giglabo-product-wipemark").is_none(),
            "Wipemark offers itself"
        );
        for id in [
            "giglabo-product-smbact",
            "giglabo-product-mnemovi",
            "giglabo-studio",
        ] {
            assert!(cx.debug_bounds(id).is_some(), "{id} is not painted");
        }
    }

    /// The toolbar's button alone in a window.
    struct Bar;

    impl Render for Bar {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().p_4().child(button())
        }
    }

    /// The pointer resting on the button brings up its tooltip, one of
    /// ours ([`Tip`]) like every other toolbar button's. Take
    /// `with_button` out of [`button`] and none comes up — red.
    #[gpui::test]
    fn the_button_has_a_tooltip(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (_, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| Bar);
            gpui_component::Root::new(view, window, cx)
        });
        cx.run_until_parked();
        let bounds = cx
            .debug_bounds("giglabo-products")
            .expect("the button is painted");
        assert!(cx.debug_bounds(crate::hover::TIP).is_none());

        cx.simulate_mouse_move(bounds.center(), None, Modifiers::default());
        for _ in 0..3 {
            cx.executor().advance_clock(Duration::from_secs(1));
            cx.run_until_parked();
        }
        cx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        let tip = cx
            .debug_bounds(crate::hover::TIP)
            .expect("no tooltip over More from GigLabo");
        assert!(
            tip.left() <= bounds.right() && tip.right() >= bounds.left(),
            "the tooltip is not at the button {bounds:?}: {tip:?}"
        );
    }

    /// Escape, Enter, the backdrop and Close are four roads to one answer,
    /// given once.
    #[gpui::test]
    fn the_dialog_closes_once(cx: &mut TestAppContext) {
        let (view, cx) = opened(cx);
        let heard = Rc::new(RefCell::new(Vec::new()));
        let recorder = heard.clone();
        let subscription = cx.update(|_, cx| {
            cx.subscribe(&view, move |_, answer: &Answer, _| {
                recorder.borrow_mut().push(*answer);
            })
        });
        cx.dispatch_action(Dismiss);
        cx.run_until_parked();
        assert_eq!(*heard.borrow(), vec![Answer::Dismissed], "Escape");
        cx.dispatch_action(Accept);
        cx.update(|_, cx| view.update(cx, |view, cx| view.close(cx)));
        cx.run_until_parked();
        drop(subscription);
        assert_eq!(
            *heard.borrow(),
            vec![Answer::Dismissed],
            "the dialog answered more than once"
        );
    }
}
