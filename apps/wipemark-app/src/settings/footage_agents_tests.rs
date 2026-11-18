//! Settings › MCP drawn headless for the agents video
//! (`dev/video/agents/storyboard.md`, 16.5–19.8 s): the page as it reads
//! while the server answers on its default address, the pointer clicking
//! through the four client tabs, and each tab's snippet.
//!
//! **Staged:** no server is started. The test turns Serve over MCP on in
//! the preferences' own field and hands them the event a listening server
//! sends (`server::Event::Listening` on 127.0.0.1:5056, through
//! `Preferences::server_said`, the supervisor's road) — so the banner, the
//! switch and the snippet are what the page draws for a server listening
//! there, and no port is bound.
//!
//! The tabs' rectangles are their hitboxes, felt for by clicks before the
//! clip (a tab of gpui-component's `TabBar` takes no wrapper to note its
//! bounds): a click at a point either selects a tab or selects none.
//!
//!   WIPEMARK_FOOTAGE=<dir> cargo test -p wipemark-app --bin wipemark -- \
//!     --ignored settings::footage_agents_tests --exact --nocapture \
//!     --test-threads=1
//!
//! It writes `<dir>/mcp-tabs/` (the clicks) and `<dir>/mcp-claude-code/`,
//! `mcp-claude-desktop/`, `mcp-n8n/`, `mcp-generic/` (one frame each, no
//! pointer).

use std::path::PathBuf;

use gpui::{
    point, px, size, AnyWindowHandle, AppContext as _, Bounds, Entity, HeadlessAppContext,
    InputEvent as _, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels,
    Point,
};
use gpui_component::Root;
use serde_json::json;

use super::{Preferences, Section, SettingsView};
use crate::footage::{curve_move, Clip, FRAME};
use crate::mcp::{server, Client, Endpoint, DEFAULT_PORT};
use crate::retention::Homes;

/// The Settings window, in points; frames are twice as large.
fn settings_size() -> gpui::Size<Pixels> {
    size(px(1100.0), px(900.0))
}

fn send(cx: &mut HeadlessAppContext, window: AnyWindowHandle, input: gpui::PlatformInput) {
    cx.update_window(window, |_, window, cx| window.dispatch_event(input, cx))
        .expect("dispatched");
}

fn redraw(cx: &mut HeadlessAppContext, window: AnyWindowHandle) {
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
        .expect("drawn");
    cx.run_until_parked();
}

/// A click at `at`, drawn after, with nothing recorded.
fn click(cx: &mut HeadlessAppContext, window: AnyWindowHandle, at: Point<Pixels>) {
    send(
        cx,
        window,
        MouseMoveEvent {
            position: at,
            pressed_button: None,
            modifiers: Modifiers::default(),
        }
        .to_platform_input(),
    );
    send(
        cx,
        window,
        MouseDownEvent {
            position: at,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: 1,
            first_mouse: false,
        }
        .to_platform_input(),
    );
    send(
        cx,
        window,
        MouseUpEvent {
            position: at,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: 1,
        }
        .to_platform_input(),
    );
    redraw(cx, window);
}

fn client_of(cx: &mut HeadlessAppContext, view: &Entity<SettingsView>) -> Client {
    cx.update(|cx| view.read(cx).client)
}

fn choose(cx: &mut HeadlessAppContext, view: &Entity<SettingsView>, client: Client) {
    view.update(cx, |view, cx| {
        view.client = client;
        view.copied = false;
        cx.notify();
    });
}

/// The tab a click at `at` selects, if any: asked twice, from two
/// different tabs selected, so a click that selects nothing is told apart
/// from a click on the tab already selected.
fn tab_at(
    cx: &mut HeadlessAppContext,
    window: AnyWindowHandle,
    view: &Entity<SettingsView>,
    at: Point<Pixels>,
) -> Option<Client> {
    let mut answers = Vec::new();
    for before in [Client::ClaudeCode, Client::Generic] {
        choose(cx, view, before);
        redraw(cx, window);
        click(cx, window, at);
        answers.push((before, client_of(cx, view)));
    }
    match answers[..] {
        [(a, after_a), (b, after_b)] if after_a == a && after_b == b => None,
        [(_, after_a), (_, after_b)] if after_a == after_b => Some(after_a),
        _ => panic!("a click at {at:?} answered {answers:?}"),
    }
}

/// The edge between `inside` (where `wanted` answers) and `outside`
/// (where it does not), to 1/64 point.
fn edge(
    cx: &mut HeadlessAppContext,
    window: AnyWindowHandle,
    view: &Entity<SettingsView>,
    wanted: Client,
    mut inside: Point<Pixels>,
    mut outside: Point<Pixels>,
) -> Point<Pixels> {
    while (inside.x - outside.x).abs() + (inside.y - outside.y).abs() > px(1.0 / 64.0) {
        let middle = point((inside.x + outside.x) / 2.0, (inside.y + outside.y) / 2.0);
        if tab_at(cx, window, view, middle) == Some(wanted) {
            inside = middle;
        } else {
            outside = middle;
        }
    }
    inside
}

/// Each tab's hitbox, felt for by clicks along and across the tab bar.
fn tab_bounds(
    cx: &mut HeadlessAppContext,
    window: AnyWindowHandle,
    view: &Entity<SettingsView>,
    bar: Bounds<Pixels>,
) -> Vec<(Client, Bounds<Pixels>)> {
    let y = bar.center().y;
    // A coarse walk along the bar, two points a step.
    let mut seen: Vec<(Client, Pixels)> = Vec::new();
    let mut x = bar.origin.x + px(1.0);
    while x < bar.origin.x + bar.size.width {
        if let Some(client) = tab_at(cx, window, view, point(x, y)) {
            if seen.last().is_none_or(|(last, _)| *last != client) {
                seen.push((client, x));
            }
        }
        x += px(2.0);
    }
    assert_eq!(
        seen.iter().map(|(client, _)| *client).collect::<Vec<_>>(),
        Client::ALL.to_vec(),
        "the walk did not meet the four tabs in order"
    );
    seen.iter()
        .map(|(client, x)| {
            let inside = point(*x, y);
            let left = edge(
                cx,
                window,
                view,
                *client,
                inside,
                point(bar.origin.x - px(20.0), y),
            );
            let right = edge(
                cx,
                window,
                view,
                *client,
                inside,
                point(bar.origin.x + bar.size.width + px(20.0), y),
            );
            let middle = (left.x + right.x) / 2.0;
            let top = edge(
                cx,
                window,
                view,
                *client,
                point(middle, y),
                point(middle, bar.origin.y - px(20.0)),
            );
            let bottom = edge(
                cx,
                window,
                view,
                *client,
                point(middle, y),
                point(middle, bar.origin.y + bar.size.height + px(20.0)),
            );
            (
                *client,
                Bounds {
                    origin: point(left.x, top.y),
                    size: size(right.x - left.x, bottom.y - top.y),
                },
            )
        })
        .collect()
}

#[test]
#[ignore]
fn agents_video_mcp() {
    let out = PathBuf::from(std::env::var("WIPEMARK_FOOTAGE").expect("WIPEMARK_FOOTAGE"));
    wipemark_i18n::init(
        &wipemark_i18n::LanguagePreference::Explicit("en-US".parse().expect("a tag")),
        wipemark_i18n::Rendering::Ui,
    );
    let scratch = std::env::temp_dir().join(format!(
        "wipemark-footage-agents-settings-{}",
        std::process::id()
    ));
    let homes = Homes {
        results: scratch.join("results"),
        kept: scratch.join("kept"),
    };

    let mut cx = crate::footage::context();
    cx.update(gpui_component::init);
    cx.update(crate::hover::init);
    cx.update(crate::dialog::init);
    cx.update(super::init);
    cx.update(|cx| cx.set_reduce_motion(true));
    type Built = (Entity<Preferences>, Entity<SettingsView>);
    let slot: std::rc::Rc<std::cell::RefCell<Option<Built>>> = std::rc::Rc::default();
    let held = slot.clone();
    let window = cx
        .open_window(settings_size(), move |window, cx| {
            crate::theme::ThemePreference::Light.apply(Some(window), cx);
            let preferences = cx.new(|cx| Preferences::for_tests(homes, cx));
            // Staged: Serve over MCP on, and the supervisor's word that a
            // server listens on the default address — no port bound.
            preferences.update(cx, |preferences, cx| {
                preferences.serving = true;
                preferences.server_said(
                    server::Event::Listening {
                        endpoint: Endpoint::default(),
                        wanted: DEFAULT_PORT,
                    },
                    cx,
                );
            });
            let view = cx.new(|cx| {
                SettingsView::new(preferences.clone(), None, Some(Section::Mcp), window, cx)
            });
            *held.borrow_mut() = Some((preferences, view.clone()));
            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("a window");
    let window: AnyWindowHandle = window.into();
    let (preferences, view) = slot.take().expect("the window builder ran");
    cx.update_window(window, |_, window, _| window.activate_window())
        .expect("activated");
    redraw(&mut cx, window);
    cx.update(|cx| {
        let preferences = preferences.read(cx);
        assert!(preferences.mcp().0, "Serve over MCP is off");
        assert!(matches!(
            preferences.status(),
            super::Status::Listening { .. }
        ));
    });

    // The tabs, felt for; then the page put back as it opened.
    let bar = *crate::spot::spots()
        .get("mcp-tabs")
        .expect("the tab bar was drawn");
    let tabs = tab_bounds(&mut cx, window, &view, bar);
    for (client, bounds) in &tabs {
        eprintln!("tab {}: {bounds:?}", client.id());
    }
    choose(&mut cx, &view, Client::ClaudeCode);
    let away = point(px(-40.0), px(-40.0));
    send(
        &mut cx,
        window,
        MouseMoveEvent {
            position: away,
            pressed_button: None,
            modifiers: Modifiers::default(),
        }
        .to_platform_input(),
    );
    redraw(&mut cx, window);
    let center = |client: Client| {
        tabs.iter()
            .find(|(c, _)| *c == client)
            .map(|(_, bounds)| bounds.center())
            .expect("a tab")
    };

    // The clicks: 3.3 s, the storyboard's 16.5–19.8 s, its clicks at
    // 18.1, 18.6 and 19.1 s — frames 48, 63 and 78. The pointer starts
    // inside the snippet block, which the video pushes in toward.
    let names = [
        "mcp-banner",
        "mcp-rows",
        "mcp-endpoint",
        "mcp-tabs",
        "mcp-snippet",
        "mcp-copy",
    ];
    let mut clip = Clip::new(&out, "mcp-tabs", window, settings_size(), &["mcp-"]);
    let snippet = *crate::spot::spots()
        .get("mcp-snippet")
        .expect("the snippet");
    let start = point(
        snippet.origin.x + snippet.size.width * 0.62,
        snippet.origin.y + snippet.size.height * 0.6,
    );
    clip.point(&mut cx, start);
    clip.mark("claude-code");
    let mut at = start;
    let mut states = vec![json!({ "frame": 0, "client": "claude-code" })];
    for (client, press_frame) in [
        (Client::ClaudeDesktop, 48usize),
        (Client::N8n, 63),
        (Client::Generic, 78),
    ] {
        let to = center(client);
        // Ten frames of the move, ending two frames before the press.
        let moves = if press_frame == 48 { 14 } else { 10 };
        while clip.next() < press_frame - moves - 2 {
            clip.frame(&mut cx);
        }
        let from = at;
        for step in 1..=moves {
            let e = curve_move(step as f32 / moves as f32);
            clip.point(
                &mut cx,
                point(from.x + (to.x - from.x) * e, from.y + (to.y - from.y) * e),
            );
            clip.frame(&mut cx);
        }
        at = to;
        while clip.next() < press_frame {
            clip.frame(&mut cx);
        }
        clip.mark(&format!("press-{}", client.id()));
        clip.press(&mut cx);
        clip.frame(&mut cx);
        clip.release(&mut cx);
        clip.mark(client.id());
        let switched = clip.next();
        clip.frame(&mut cx);
        assert_eq!(
            client_of(&mut cx, &view),
            client,
            "the click did not switch"
        );
        states.push(json!({ "frame": switched, "client": client.id() }));
    }
    while clip.next() < 99 {
        clip.frame(&mut cx);
    }
    for name in names {
        clip.region(name.trim_start_matches("mcp-"), name, 0);
    }
    for (client, frame) in [("claude-desktop", 48), ("n8n", 63), ("generic", 78)] {
        clip.region(&format!("snippet-{client}"), "mcp-snippet", frame + 2);
    }
    for (client, bounds) in &tabs {
        clip.measure(&format!("tab-{}", client.id()), *bounds);
    }
    clip.note("states", json!(states));
    clip.note("staged", json!("Serve over MCP set on and Listening on 127.0.0.1:5056 handed to the preferences by the test; no server ran"));
    clip.note("theme", json!("light"));
    clip.finish("agents video: Settings › MCP, the pointer through the four client tabs");

    // One frame of each tab, no pointer.
    send(
        &mut cx,
        window,
        MouseMoveEvent {
            position: away,
            pressed_button: None,
            modifiers: Modifiers::default(),
        }
        .to_platform_input(),
    );
    for client in Client::ALL {
        choose(&mut cx, &view, client);
        cx.run_until_parked();
        cx.advance_clock(FRAME);
        let mut still = Clip::new(
            &out,
            &format!("mcp-{}", client.id()),
            window,
            settings_size(),
            &["mcp-"],
        );
        still.frame(&mut cx);
        for name in names {
            still.region(name.trim_start_matches("mcp-"), name, 0);
        }
        for (client, bounds) in &tabs {
            still.measure(&format!("tab-{}", client.id()), *bounds);
        }
        still.note("client", json!(client.id()));
        still.note("label", json!(client.label()));
        still.note("snippet", json!(client.snippet(Endpoint::default())));
        still.note("staged", json!("Serve over MCP set on and Listening on 127.0.0.1:5056 handed to the preferences by the test; no server ran"));
        still.note("theme", json!("light"));
        still.finish("agents video: Settings › MCP, one client tab");
    }
    std::fs::remove_dir_all(&scratch).ok();
}
