//! The committed UCD files themselves, and the public face of the
//! tables built from them: `UNICODE_VERSION` and `name_of`.
//!
//! The build checks that the files are well formed and of one version;
//! these tests check that they are the bytes `scripts/fetch-ucd.sh`
//! wrote. The data files are `-diff` in `.gitattributes`, so an edit to
//! one is invisible in review — the checksum test is where it shows.

use wipemark_core::{name_of, UNICODE_VERSION};

/// The nine data files, as `SHA256SUMS` lists them, with their bytes.
const FILES: [(&str, &[u8]); 9] = [
    (
        "DerivedCoreProperties.txt",
        include_bytes!("../ucd/DerivedCoreProperties.txt"),
    ),
    (
        "DerivedNormalizationProps.txt",
        include_bytes!("../ucd/DerivedNormalizationProps.txt"),
    ),
    (
        "NormalizationTest.txt",
        include_bytes!("../ucd/NormalizationTest.txt"),
    ),
    ("PropList.txt", include_bytes!("../ucd/PropList.txt")),
    ("Scripts.txt", include_bytes!("../ucd/Scripts.txt")),
    (
        "StandardizedVariants.txt",
        include_bytes!("../ucd/StandardizedVariants.txt"),
    ),
    ("UnicodeData.txt", include_bytes!("../ucd/UnicodeData.txt")),
    ("confusables.txt", include_bytes!("../ucd/confusables.txt")),
    (
        "emoji/emoji-data.txt",
        include_bytes!("../ucd/emoji/emoji-data.txt"),
    ),
];

const SHA256SUMS: &str = include_str!("../ucd/SHA256SUMS");

#[test]
fn every_table_comes_from_one_unicode_version() {
    let headed: [(&str, &str); 6] = [
        (
            "DerivedCoreProperties",
            include_str!("../ucd/DerivedCoreProperties.txt"),
        ),
        ("PropList", include_str!("../ucd/PropList.txt")),
        ("Scripts", include_str!("../ucd/Scripts.txt")),
        (
            "StandardizedVariants",
            include_str!("../ucd/StandardizedVariants.txt"),
        ),
        (
            "DerivedNormalizationProps",
            include_str!("../ucd/DerivedNormalizationProps.txt"),
        ),
        (
            "NormalizationTest",
            include_str!("../ucd/NormalizationTest.txt"),
        ),
    ];
    for (stem, text) in headed {
        let first = text.lines().next().unwrap_or("");
        let version = first
            .strip_prefix("# ")
            .and_then(|rest| rest.strip_prefix(stem))
            .and_then(|rest| rest.strip_prefix('-'))
            .and_then(|rest| rest.strip_suffix(".txt"));
        assert_eq!(
            version,
            Some(UNICODE_VERSION),
            "{stem}.txt line 1: {first:?}"
        );
    }
    let versioned: [(&str, &str); 2] = [
        (
            "emoji/emoji-data.txt",
            include_str!("../ucd/emoji/emoji-data.txt"),
        ),
        ("confusables.txt", include_str!("../ucd/confusables.txt")),
    ];
    for (path, text) in versioned {
        let version: Vec<&str> = text
            .lines()
            .take_while(|line| line.starts_with('#'))
            .filter_map(|line| line.strip_prefix("# Version: "))
            .collect();
        assert_eq!(version, [UNICODE_VERSION], "{path}");
    }
}

/// A §1 pins the version; a bump is a deliberate edit of this test.
#[test]
fn the_pinned_version_is_18_0_0() {
    assert_eq!(UNICODE_VERSION, "18.0.0");
}

#[test]
fn the_ucd_readme_names_the_version_the_tables_were_built_from() {
    assert!(include_str!("../ucd/README.md").contains(UNICODE_VERSION));
}

/// SHA-256 (FIPS 180-4), std only: the crate may not depend on a hash
/// crate, even for a test.
fn sha256(bytes: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    let mut message = bytes.to_vec();
    let length = u64::try_from(bytes.len()).expect("length") * 8;
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&length.to_be_bytes());
    for block in message.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(choice)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(majority);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (state, value) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *state = state.wrapping_add(value);
        }
    }
    let mut digest = [0u8; 32];
    for (chunk, word) in digest.chunks_exact_mut(4).zip(h) {
        chunk.copy_from_slice(&word.to_be_bytes());
    }
    digest
}

fn hex(digest: &[u8]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn the_checksum_function_is_sha256() {
    assert_eq!(
        hex(&sha256(b"")),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        hex(&sha256(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

/// The counterpart of `-diff`: an edit to a data file is invisible in
/// review, and the build only checks structure — a silent change of one
/// value (`Latin` to `Cyrillic` on one line of `Scripts.txt`) builds
/// green and fails here.
#[test]
fn every_ucd_file_matches_its_checksum() {
    for line in SHA256SUMS.lines() {
        let (listed, path) = line.split_once("  ").expect("<hash>  <path>");
        let (_, bytes) = FILES
            .iter()
            .find(|(name, _)| *name == path)
            .unwrap_or_else(|| panic!("SHA256SUMS lists {path}, which this test does not read"));
        assert_eq!(
            hex(&sha256(bytes)),
            listed,
            "ucd/{path} is not the file SHA256SUMS hashed; re-run scripts/fetch-ucd.sh"
        );
    }
}

#[test]
fn sha256sums_lists_exactly_the_nine_files() {
    let lines: Vec<&str> = SHA256SUMS.lines().collect();
    assert_eq!(lines.len(), 9, "{lines:?}");
    let mut paths = Vec::new();
    for line in lines {
        let (hash, path) = line.split_once("  ").expect("<hash>  <path>");
        assert!(
            hash.len() == 64
                && hash
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "{line:?}"
        );
        paths.push(path);
    }
    let mut expected: Vec<&str> = FILES.iter().map(|(path, _)| *path).collect();
    // `LC_ALL=C` order is byte order.
    expected.sort_unstable();
    assert_eq!(paths, expected);
}

#[test]
fn the_public_api_names_a_zero_width_space() {
    assert_eq!(name_of('\u{200B}').as_deref(), Some("ZERO WIDTH SPACE"));
    assert_eq!(name_of('\u{E000}').as_deref(), Some("<private-use-E000>"));
}
