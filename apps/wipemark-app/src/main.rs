//! Wipemark — the GPUI desktop application.
//!
//! # Skeleton status
//!
//! Epic **E0**. This opens the real window with the real theme and lays
//! out the three panes the product is built around, so that the shell,
//! the pins and the bundle metadata are proven before any feature lands.
//! Every pane is a placeholder; epics E6–E8 fill them:
//!
//! * **left** — documents and the batch queue (E7 / S7.1)
//! * **centre** — Source | Result editors, invisible characters rendered
//!   as badges, streamed tokens, word diff (E7 / S7.2–S7.4)
//! * **right** — Inspector findings and the three-shelf report
//!   (E7 / S7.5–S7.6)
//! * **status bar** — engine, model, stage, cancel, RAM/VRAM (E6 / S6.1)
//!
//! # The one rule this file exists to protect
//!
//! GPUI runs on its own executor. `tokio` — which the downloader, the
//! HTTP engine and `llama-cpp-2` all need — runs on its own. They cannot
//! await each other. Every long operation therefore hands back a
//! `flume::Receiver` that the GPUI side polls from `cx.spawn`, and no
//! blocking call is ever made on the foreground thread (spec §1.2, §12).
//! A single `std::fs::read` of a 2 GB model on this thread is a frozen
//! window, and it will be blamed on GPUI rather than on the call.

use gpui::prelude::*;
use gpui::{
    div, px, size, App, Bounds, Context, Hsla, TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use gpui_component::{ActiveTheme, Root};

/// Root view of the main window.
struct Shell;

/// One placeholder pane: a title and the epic that will replace it.
fn pane(title: &'static str, body: &'static str, muted: Hsla) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .child(div().text_sm().child(title))
        .child(div().text_sm().text_color(muted).child(body))
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let border = theme.border;
        let muted = theme.muted_foreground;

        div()
            .flex()
            .flex_col()
            .size_full()
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h(px(0.0))
                    .child(
                        div()
                            .w(px(260.0))
                            .border_r_1()
                            .border_color(border)
                            .child(pane(
                                "Documents",
                                "Drop files or paste text. Queue and per-document progress land in E7.",
                                muted,
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .child(pane(
                                "Source | Result",
                                "Invisible characters shown as badges on the left, streamed rewrite on the right, word diff once it finishes — E7.",
                                muted,
                            )),
                    )
                    .child(
                        div()
                            .w(px(320.0))
                            .border_l_1()
                            .border_color(border)
                            .child(pane(
                                "Inspector",
                                "Findings by class and confidence, then the report: verifiable / best-effort / not established — E7.",
                                muted,
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .p_3()
                    .border_t_1()
                    .border_color(border)
                    .text_sm()
                    .text_color(muted)
                    // Honest by default: with no engine configured the
                    // product is a Layer A tool, and it says so rather
                    // than implying a rewrite is available.
                    .child("Idle · no engine configured · Layer A only"),
            )
    }
}

fn main() {
    if let Err(error) = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init()
    {
        eprintln!("wipemark: logging init failed: {error}");
    }

    gpui_platform::application().run(|cx: &mut App| {
        // Must come before anything else touches a component or a theme.
        gpui_component::init(cx);

        let bounds = Bounds::centered(None, size(px(1280.0), px(800.0)), cx);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Wipemark".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                let shell = cx.new(|_| Shell);
                // The first level inside a window has to be a Root —
                // dialogs, sheets, notifications and tooltips all mount
                // through it.
                cx.new(|cx| Root::new(shell, window, cx).bg(cx.theme().background))
            },
        );

        if let Err(error) = opened {
            tracing::error!(%error, "failed to open the main window");
            return;
        }
        cx.activate(true);
    });
}
