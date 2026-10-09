//! `WIPEMARK_INTERVAL` (E12-R8) is read by a `planar-preview` build
//! alone: the product's own `clean` never refines, whatever the
//! environment says. A test binary of its own, because it sets a process
//! variable every `clean` reads in that build.

use std::path::PathBuf;

use wipemark_picture::{clean, PictureOptions, Visible};
use wipemark_pixels::{Catalogue, Method};

#[test]
fn the_product_refines_only_in_the_preview_and_only_when_asked() {
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/image/gemini/torch-1025-q95-444.jpg"),
    )
    .unwrap();
    let options = PictureOptions {
        scope: wipemark_image::Scope::AiProvenance,
        catalogue: Some(Catalogue::shipped().unwrap()),
    };
    let interval = |value: Option<&str>| {
        match value {
            Some(v) => std::env::set_var("WIPEMARK_INTERVAL", v),
            None => std::env::remove_var("WIPEMARK_INTERVAL"),
        }
        let (_, report) = clean(&bytes, &options).unwrap();
        let Visible::Examined { report, .. } = report.visible else {
            panic!("not examined");
        };
        report.restored[0].interval.map(|i| i.method)
    };
    assert_eq!(interval(None), None);
    assert_eq!(interval(Some("none")), None);
    assert_eq!(interval(Some("blur")), None);
    let preview = cfg!(feature = "planar-preview");
    for (word, method) in [
        ("dct", Method::Dct),
        ("pixel", Method::Pixel),
        ("wiener", Method::Wiener),
    ] {
        assert_eq!(interval(Some(word)), preview.then_some(method), "{word}");
    }
    std::env::remove_var("WIPEMARK_INTERVAL");
}
