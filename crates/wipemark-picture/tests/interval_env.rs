//! The product's refinement (D472, the owner, 2026-10-10): `clean`
//! refines a restoration on a lossy JPEG by DCT-POCS, by default and
//! whatever the environment says — `WIPEMARK_INTERVAL`, which a
//! `planar-preview` build read until the decisions landed, is read by
//! nothing now. A test binary of its own, because it sets a process
//! variable.

use std::path::PathBuf;

use wipemark_picture::{clean, PictureOptions, Visible, REFINE};
use wipemark_pixels::{Catalogue, Method, Refine};

#[test]
fn the_product_refines_by_dct_pocs_whatever_the_environment_says() {
    assert_eq!(REFINE, Refine::Dct);
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
    for value in [
        None,
        Some("none"),
        Some("dct"),
        Some("pixel"),
        Some("wiener"),
        Some("blur"),
    ] {
        assert_eq!(interval(value), Some(Method::Dct), "{value:?}");
    }
    std::env::remove_var("WIPEMARK_INTERVAL");
}
