//! Settings › Engine and Settings › Models drawn headless as footage for
//! the rewrite video (`dev/video/rewrite/storyboard.md`, "Who rewrites"
//! and "The catalogue"): the pages of a 16 GB Apple silicon Mac with Gemma
//! 4 12B in its models folder, staged by [`Preferences::stage_scan`] — the
//! machine and what the scan would have found, never a model loaded or a
//! memory figure measured — on the catalogue of the release the video
//! names. Every frame is GPUI's own renderer (`crate::footage`), every
//! rectangle the one GPUI laid the part out at (`crate::spot`).
//!
//! Run (ignored, one at a time: they set the process's language):
//!   WIPEMARK_FOOTAGE=<dir> WIPEMARK_FOOTAGE_CATALOGUE=<models.v1.json> \
//!   [WIPEMARK_FOOTAGE_DARK=1] [WIPEMARK_FOOTAGE_FOLDER=<folder>] \
//!   cargo test -p wipemark-app --bin wipemark -- --ignored \
//!     settings::footage_tests::<test> --exact --nocapture --test-threads=1
//!
//! - `rewrite_video_engine`: `E1`, *Who rewrites: This machine* with Gemma
//!   4 12B chosen; `E2`, *The endpoint*: Ollama at a documentation address
//!   (`192.0.2.10`, RFC 5737), `llama3.1:8b`, *Allow a remote endpoint*
//!   off. Both as tall as the page, at one size.
//! - `rewrite_video_models`: `M1`, the catalogue with nothing chosen yet —
//!   the page then names the catalogue's pick for this machine — and
//!   `M1-chosen`, Gemma 4 12B chosen as on the Engine page. As tall as the
//!   page.
//!
//! `WIPEMARK_FOOTAGE_CATALOGUE` is the catalogue the stills show, read as
//! this build reads its own: `manifests/models.v1.json` at the tag
//! (`git show wipemark-v0.1.4:manifests/models.v1.json`). The models folder
//! is `<folder>/models`, `<folder>` being `/Users/Shared/Wipemark
//! Demo/rewrite` unless given; nothing is written to it.

use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use gpui::{px, size, AnyWindowHandle, AppContext as _, Entity, HeadlessAppContext, Pixels, Size};
use gpui_component::Root;
use serde_json::{json, Value};
use wipemark_models::host::Host;
use wipemark_models::manifest::{Manifest, Role, Status};

use super::{engine_banner, models_banner, Preferences, Section, SettingsView, Shelf};
use crate::duty::Serves;
use crate::engine::{BaseUrl, Provider};
use crate::footage::Clip;
use crate::models;
use crate::retention::Homes;

/// The model on this machine, by its catalogue id.
const GEMMA: &str = "gemma-4-12b-it-qat-ud-q4";

/// The endpoint E2 names: an address reserved for documentation, so no
/// host name and no real machine.
const ENDPOINT: &str = "http://192.0.2.10:11434";
const REMOTE_MODEL: &str = "llama3.1:8b";

/// A Mac with 16 GB of memory shared by its processors and its GPU. What
/// is free right now is never shown and never refuses anything; half.
fn mac() -> Host {
    Host {
        total_ram_mb: 16_384,
        available_ram_mb: 8_192,
        vram_mb: None,
        unified_memory: true,
    }
}

/// The catalogue the stills show, and a check that it is this build's
/// less the entries marked experimental since.
fn catalogue() -> Manifest {
    let path = std::env::var_os("WIPEMARK_FOOTAGE_CATALOGUE").expect("WIPEMARK_FOOTAGE_CATALOGUE");
    let text = std::fs::read_to_string(path).expect("the catalogue");
    let shown = Manifest::parse(&text).expect("a catalogue this build reads");
    let built = Manifest::embedded().expect("this build's catalogue");
    let ids = |manifest: &Manifest, stable: bool| -> Vec<String> {
        manifest
            .models
            .iter()
            .filter(|entry| !stable || entry.status != Status::Experimental)
            .map(|entry| entry.id.clone())
            .collect()
    };
    assert_eq!(
        ids(&shown, false),
        ids(&built, true),
        "the catalogue shown is not this build's less its experimental entries"
    );
    shown
}

/// Where the models folder is said to be: a folder that names nobody.
fn models_folder() -> PathBuf {
    std::env::var_os("WIPEMARK_FOOTAGE_FOLDER")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/Users/Shared/Wipemark Demo/rewrite"))
        .join("models")
}

/// Who rewrites, as the Engine page's first row sets it.
#[derive(Clone, Copy)]
enum Duty {
    /// *This machine*, Gemma 4 12B.
    Machine,
    /// *The endpoint*: Ollama at [`ENDPOINT`], [`REMOTE_MODEL`], a remote
    /// endpoint not allowed.
    Endpoint,
}

struct Bench {
    cx: HeadlessAppContext,
    window: AnyWindowHandle,
    preferences: Entity<Preferences>,
    theme: &'static str,
    _homes: Homes,
}

/// The Settings window on `section`, `css` points large, over preferences
/// staged as `duty` says, Gemma 4 12B chosen for rewriting when `chosen`:
/// English, the theme asked for, reduced motion; laid out.
fn bench(section: Section, css: Size<Pixels>, duty: Duty, chosen: bool) -> Bench {
    let dark = std::env::var_os("WIPEMARK_FOOTAGE_DARK").is_some();
    wipemark_i18n::init(
        &wipemark_i18n::LanguagePreference::Explicit("en-US".parse().expect("a tag")),
        wipemark_i18n::Rendering::Ui,
    );
    let catalogue = catalogue();
    let scratch = std::env::temp_dir().join(format!(
        "wipemark-settings-footage-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let homes = Homes {
        results: scratch.join("results"),
        kept: scratch.join("kept"),
    };

    let mut cx = crate::footage::context();
    cx.update(gpui_component::init);
    cx.update(crate::hover::init);
    cx.update(crate::dialog::init);
    cx.update(|cx| cx.set_reduce_motion(true));

    let slot: Rc<std::cell::RefCell<Option<Entity<Preferences>>>> = Rc::default();
    let held = slot.clone();
    let kept = homes.clone();
    let window = cx
        .open_window(css, move |window, cx| {
            let theme = if dark {
                crate::theme::ThemePreference::Dark
            } else {
                crate::theme::ThemePreference::Light
            };
            theme.apply(Some(window), cx);
            let preferences = cx.new(|cx| Preferences::for_tests(kept, cx));
            preferences.update(cx, |preferences, cx| {
                preferences.stage_scan(models_folder(), mac(), Some(catalogue), &[GEMMA], cx);
                if chosen {
                    preferences.select_rewrite_model(Some(GEMMA.to_owned()), cx);
                }
                match duty {
                    Duty::Machine => preferences.select_serves(Serves::MachineOnly, cx),
                    Duty::Endpoint => {
                        preferences.select_serves(Serves::EndpointOnly, cx);
                        preferences.select_provider(Provider::Ollama, cx);
                        preferences
                            .select_endpoint(BaseUrl::parse(ENDPOINT).expect("a base URL"), cx);
                        preferences.select_model(REMOTE_MODEL.to_owned(), cx);
                        preferences.allow_remote(false, cx);
                    }
                }
            });
            *held.borrow_mut() = Some(preferences.clone());
            let view = cx.new(|cx| SettingsView::new(preferences, None, Some(section), window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("a window");
    let window: AnyWindowHandle = window.into();
    let preferences = slot.take().expect("the window builder ran");
    for _ in 0..4 {
        cx.run_until_parked();
        cx.advance_clock(Duration::from_millis(400));
        cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
            .expect("drawn");
    }
    cx.run_until_parked();
    // Staged, and nothing loaded: the engine host the application runs is
    // not here, so no page can say a model is in memory.
    cx.update(|cx| {
        assert!(crate::engine_host::hosted(cx).is_none(), "an engine host");
        let preferences = preferences.read(cx);
        assert_eq!(preferences.host(), Some(mac()));
        assert!(matches!(
            preferences.model_state(GEMMA),
            wipemark_models::store::State::Present { .. }
        ));
    });
    Bench {
        cx,
        window,
        preferences,
        theme: if dark { "dark" } else { "light" },
        _homes: homes,
    }
}

/// How tall the page is: the lowest part of it drawn, and the page's own
/// padding under it.
fn page_height() -> Pixels {
    crate::spot::spots()
        .iter()
        .filter(|(name, _)| name.starts_with("settings-") && *name != "settings-sidebar")
        .map(|(_, bounds)| bounds.bottom())
        .fold(px(0.0), Pixels::max)
        + px(24.0)
}

/// A line as the composition reads it: Fluent's isolates taken out.
fn plain(text: String) -> String {
    text.replace(['\u{2068}', '\u{2069}'], "")
}

/// One frame of the window as it stands, every Settings part named.
fn still(bench: &mut Bench, out: &std::path::Path, name: &str, css: Size<Pixels>) -> Clip {
    let name = if bench.theme == "dark" {
        format!("{name}-dark")
    } else {
        name.to_owned()
    };
    let mut clip = Clip::new(out, &name, bench.window, css, &["settings-"]);
    clip.frame(&mut bench.cx);
    for spot in crate::spot::spots().keys() {
        if let Some(part) = spot.strip_prefix("settings-") {
            clip.region(part, spot, 0);
        }
    }
    clip.note("theme", json!(bench.theme));
    clip.note(
        "staged",
        json!({
            "host": {
                "total_ram_mb": mac().total_ram_mb,
                "available_ram_mb": mac().available_ram_mb,
                "vram_mb": Value::Null,
                "unified_memory": mac().unified_memory,
                "gpu_backend": true,
            },
            "models_folder": models_folder().display().to_string(),
            "on_disk": [GEMMA],
            "on_disk_as": "a file found at the models folder's top, not downloaded by Wipemark (D302)",
            "loaded": false,
            "memory_measured": false,
            "catalogue": "WIPEMARK_FOOTAGE_CATALOGUE: manifests/models.v1.json at the tag",
        }),
    );
    clip
}

/// E1 and E2: who rewrites, this machine and then an endpoint on another
/// machine that the page refuses until a remote endpoint is allowed.
#[test]
#[ignore]
fn rewrite_video_engine() {
    let out = PathBuf::from(std::env::var("WIPEMARK_FOOTAGE").expect("WIPEMARK_FOOTAGE"));
    let probe = size(px(1280.0), px(720.0));
    // Both as tall as the taller page, so the two stills stand at one size.
    let mut tallest = px(720.0);
    for duty in [Duty::Machine, Duty::Endpoint] {
        let mut bench = bench(Section::Engine, probe, duty, true);
        bench
            .cx
            .update_window(bench.window, |_, window, cx| window.draw(cx).clear(cx))
            .expect("drawn");
        tallest = tallest.max(page_height());
    }
    let css = size(px(1280.0), tallest.ceil());
    for (name, duty) in [("E1", Duty::Machine), ("E2", Duty::Endpoint)] {
        let mut bench = bench(Section::Engine, css, duty, true);
        let preferences = bench.preferences.clone();
        let said = bench.cx.update(|cx| {
            let preferences = preferences.read(cx);
            let (_, _, lines) = engine_banner(&preferences.duty(Role::Rewrite), false);
            let engine = preferences.engine();
            json!({
                "banner": lines.into_iter().map(plain).collect::<Vec<_>>(),
                "serves": format!("{:?}", preferences.serves()),
                "provider": engine.provider.id(),
                "endpoint": engine.base_url.to_string(),
                "model": engine.model,
                "allow_remote": engine.allow_remote,
                "rewrite_model": preferences.rewrite_model(),
            })
        });
        match duty {
            Duty::Machine => assert!(
                said["banner"][0]
                    .as_str()
                    .is_some_and(|line| line.contains("Gemma 4 12B") && line.contains("runs here")),
                "{said}"
            ),
            Duty::Endpoint => assert!(
                said["banner"][0].as_str().is_some_and(
                    |line| line.contains("192.0.2.10") && line.contains("not this machine")
                ),
                "{said}"
            ),
        }
        let mut clip = still(&mut bench, &out, name, css);
        clip.note("said", said);
        clip.finish(&format!(
            "rewrite video, Settings › Engine, {}",
            match duty {
                Duty::Machine => "who rewrites: this machine, Gemma 4 12B",
                Duty::Endpoint =>
                    "who rewrites: an Ollama endpoint on another machine, not allowed",
            }
        ));
    }
}

/// M1 and M1-chosen: the catalogue on a 16 GB Mac with Gemma 4 12B in the
/// models folder, nothing chosen yet and then Gemma chosen.
#[test]
#[ignore]
fn rewrite_video_models() {
    let out = PathBuf::from(std::env::var("WIPEMARK_FOOTAGE").expect("WIPEMARK_FOOTAGE"));
    let probe = size(px(1280.0), px(720.0));
    let mut tallest = px(720.0);
    for chosen in [false, true] {
        let mut bench = bench(Section::Models, probe, Duty::Machine, chosen);
        bench
            .cx
            .update_window(bench.window, |_, window, cx| window.draw(cx).clear(cx))
            .expect("drawn");
        tallest = tallest.max(page_height());
    }
    let css = size(px(1280.0), tallest.ceil());
    for (name, chosen) in [("M1", false), ("M1-chosen", true)] {
        let mut bench = bench(Section::Models, css, Duty::Machine, chosen);
        let preferences = bench.preferences.clone();
        let said = bench.cx.update(|cx| {
            let preferences = preferences.read(cx);
            let (_, lines) = models_banner(
                preferences.host(),
                preferences.models_scanned(),
                Shelf {
                    dir: preferences.models_dir(),
                    installed: preferences.installed_count(),
                    folder: preferences.folder(),
                },
                None,
                None,
            );
            let recommended = models::recommended(
                preferences.catalogue(),
                Role::Rewrite,
                preferences.host(),
                preferences.rewrite_model(),
            )
            .map(|entry| entry.id.clone());
            let cards: Vec<Value> = preferences
                .catalogue()
                .models
                .iter()
                .filter(|entry| wipemark_models::DRAFTS_OFFERED || !entry.serves(Role::Draft))
                .map(|entry| {
                    let found = preferences.model_found_at(&entry.id);
                    let card = models::card(
                        entry,
                        preferences.host(),
                        &preferences.model_state(&entry.id),
                        preferences.downloading(&entry.id),
                        found.as_deref(),
                    );
                    let action = card.availability.action().map(wipemark_i18n::t);
                    json!({
                        "id": entry.id,
                        "region": format!("model-{}", entry.id),
                        "name": card.display,
                        "details": plain(format!(
                            "{} · {} · {}",
                            card.size,
                            card.needs,
                            card.roles.iter().map(|role| role.id()).collect::<Vec<_>>().join(", ")
                        )),
                        "line": plain(card.line()),
                        "chosen": preferences.rewrite_model() == Some(entry.id.as_str()),
                        "recommended": recommended.as_deref() == Some(entry.id.as_str()),
                        "button": action.map(plain),
                        "label": match card.availability {
                            models::Availability::Found { .. } => Some(wipemark_i18n::t(
                                wipemark_i18n::Message::SettingsModelsInstalled,
                            )),
                            _ => None,
                        },
                    })
                })
                .collect();
            json!({
                "banner": lines.into_iter().map(plain).collect::<Vec<_>>(),
                "recommended_words": plain(wipemark_i18n::t(wipemark_i18n::Message::SettingsModelsRecommended)),
                "cards": cards,
                "rewrite_model": preferences.rewrite_model(),
            })
        });
        let mut clip = still(&mut bench, &out, name, css);
        clip.note("said", said);
        clip.finish(&format!(
            "rewrite video, Settings › Models, the catalogue on a 16 GB Mac with Gemma 4 12B here, {}",
            if chosen { "Gemma 4 12B chosen" } else { "nothing chosen yet" }
        ));
    }
}
