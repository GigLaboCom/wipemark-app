//! `consistency_px` on real files (E12-R7, D305): the restored picture,
//! blended back with the map and the logo it was restored with, against
//! the stored input — the identity of the inverse, held on the owner's
//! own Gemini crops (`fixtures/image/gemini/`) as `wipemark_picture::clean`
//! runs them, PNG, JPEG and WebP alike.

use std::path::PathBuf;

use wipemark_picture::{clean, PictureOptions, Visible};
use wipemark_pixels::Catalogue;

fn shipped() -> PictureOptions<'static> {
    PictureOptions {
        scope: wipemark_image::Scope::AiProvenance,
        catalogue: Some(Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"))),
    }
}

/// Every committed crop: whatever is restored — on a lossless source and
/// on a lossy one, 4:4:4 and 4:2:0, at a row and by the search — is
/// consistent with its input to the rounding of one level at the 95th
/// percentile. The map is fitted (D245) and the logo measured (D242), so
/// none of it is `exact`; the measure does not ask whether the map is the
/// vendor's, only whether the inverse is still an inverse of the data.
#[test]
fn an_exact_inverse_is_consistent_to_rounding() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/image/gemini");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| [".png", ".jpg", ".webp"].iter().any(|x| n.ends_with(x)))
        .collect();
    names.sort();
    assert_eq!(names.len(), 14, "{names:?}");
    let (mut restored, mut lossy) = (0, 0);
    for name in &names {
        let bytes = std::fs::read(dir.join(name)).unwrap();
        let Ok((_, report)) = clean(&bytes, &shipped()) else {
            continue;
        };
        let Visible::Examined { report, .. } = &report.visible else {
            continue;
        };
        for r in &report.restored {
            // `--nocapture` prints the figures E12-R7's report gives.
            println!(
                "{name}: consistency_px {:.3}, excluded {}, clamped {}, holes {}, lossy {}",
                r.consistency_px, r.consistency_excluded, r.clamped, r.holes, r.lossy
            );
            assert!(r.consistency_px <= 1.0, "{name}: {r:?}");
            assert_eq!(
                r.consistency_excluded,
                r.clamped + 3 * r.holes,
                "{name}: {r:?}"
            );
            assert_eq!(r.consistency_dct, None, "{name}");
            restored += 1;
            lossy += usize::from(r.lossy);
        }
    }
    // Not vacuous: most crops are restored, the lossy ones among them.
    assert!(
        restored >= 9 && lossy >= 5,
        "{restored} restored, {lossy} lossy"
    );
}
