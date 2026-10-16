//! An AppImage that integrates itself into the desktop (D651).
//!
//! GNOME matches a window to its application by the window's `WM_CLASS` /
//! Wayland `app_id` (`wipemark`, D605) and a `wipemark.desktop` file on
//! the data path, and takes the icon from that file. Since mutter 45 a
//! window's own `_NET_WM_ICON` (D640) is no longer shown for a window no
//! desktop file claims. A `.deb` installs the file; an AppImage run as it
//! is installs nothing, so the dock and Alt+Tab showed a generic icon
//! (acceptance F13).
//!
//! When this process runs from an AppImage (`$APPIMAGE` is set), it
//! writes, on a thread of its own:
//!
//! * `<data home>/applications/wipemark.desktop`, the packaged desktop file
//!   with `Exec` and `TryExec` naming this AppImage, and a marker line
//!   saying the file is ours;
//! * the 32, 128 and 512 px icons as
//!   `<data home>/icons/hicolor/<n>x<n>/apps/wipemark.png`.
//!
//! A desktop file there without the marker is somebody's and is left
//! alone. Ours is rewritten only when it differs, so an AppImage moved to
//! another folder is followed by the next launch. Nothing is written for
//! a `.deb` (no `$APPIMAGE`) or when the data home cannot be found.

use std::path::{Path, PathBuf};

/// The line that says a desktop file is this module's to rewrite.
const MARKER: &str = "X-Wipemark-AppImage=true";

/// The desktop file the packages install, as built (D603).
const PACKAGED: &str = include_str!("../../../packaging/linux/wipemark.desktop");

/// The icons, by side.
const ICONS: [(u32, &[u8]); 3] = [
    (32, include_bytes!("../assets/icon/32x32.png")),
    (128, include_bytes!("../assets/icon/128x128.png")),
    (512, include_bytes!("../assets/icon/icon.png")),
];

/// Integrate in the background when this is an AppImage.
pub fn start() {
    let Some(appimage) = std::env::var_os("APPIMAGE").map(PathBuf::from) else {
        return;
    };
    let Some(home) = data_home() else {
        return;
    };
    let spawned = std::thread::Builder::new()
        .name("wipemark-integrate".to_owned())
        .spawn(move || match integrate(&home, &appimage) {
            Ok(true) => tracing::info!("the desktop file and icons now name this AppImage"),
            Ok(false) => {}
            Err(error) => tracing::warn!(%error, "the AppImage could not integrate itself"),
        });
    if let Err(error) = spawned {
        tracing::warn!(%error, "no integration thread");
    }
}

/// `$XDG_DATA_HOME`, or `~/.local/share`.
fn data_home() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME").filter(|dir| !dir.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
}

/// Write what differs. `Ok(true)` when anything was written.
fn integrate(home: &Path, appimage: &Path) -> std::io::Result<bool> {
    let mut wrote = false;
    let desktop = home.join("applications/wipemark.desktop");
    let wanted = desktop_file(appimage);
    match std::fs::read_to_string(&desktop) {
        Ok(current) if !current.lines().any(|line| line == MARKER) => {
            tracing::info!("a wipemark.desktop that is not ours is left alone");
        }
        Ok(current) if current == wanted => {}
        _ => {
            write(&desktop, wanted.as_bytes())?;
            wrote = true;
        }
    }
    let hicolor = home.join("icons/hicolor");
    let mut icons_written = false;
    for (side, png) in ICONS {
        let path = hicolor.join(format!("{side}x{side}/apps/wipemark.png"));
        if std::fs::read(&path).ok().as_deref() != Some(png) {
            write(&path, png)?;
            icons_written = true;
        }
    }
    if icons_written || cache_misses_us(&hicolor) {
        refresh_cache(&hicolor);
        wrote = true;
    }
    Ok(wrote || icons_written)
}

/// Whether an `icon-theme.cache` in `hicolor` predates our icons. Another
/// program's cache — an older AppImage's — makes GTK and the shell look
/// for `wipemark` in a list that does not have it, and the dock shows a
/// generic icon (acceptance F13): the cache stays trusted until the
/// `hicolor` folder itself changes, which writing into its subfolders
/// does not do.
fn cache_misses_us(hicolor: &Path) -> bool {
    let modified = |path: &Path| std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let Some(cache) = modified(&hicolor.join("icon-theme.cache")) else {
        return false;
    };
    ICONS.iter().any(|(side, _)| {
        modified(&hicolor.join(format!("{side}x{side}/apps/wipemark.png")))
            .is_some_and(|icon| icon > cache)
    })
}

/// Make an existing `icon-theme.cache` list our icons: rebuild it with
/// `gtk-update-icon-cache` when the machine has it, otherwise mark the
/// `hicolor` folder changed so the stale cache is no longer trusted.
/// Nothing when there is no cache — GTK then reads the folders.
fn refresh_cache(hicolor: &Path) {
    if !hicolor.join("icon-theme.cache").exists() {
        return;
    }
    let rebuilt = std::process::Command::new("gtk-update-icon-cache")
        .args(["-q", "-f", "-t"])
        .arg(hicolor)
        .status()
        .is_ok_and(|status| status.success());
    if rebuilt {
        tracing::info!("the user's icon cache was rebuilt to list wipemark");
        return;
    }
    let touched = std::fs::File::open(hicolor)
        .and_then(|folder| folder.set_modified(std::time::SystemTime::now()));
    match touched {
        Ok(()) => tracing::info!(
            "the user's icon cache is stale now: the hicolor folder was marked changed"
        ),
        Err(error) => tracing::warn!(%error, "a stale icon cache may hide wipemark's icon"),
    }
}

/// The packaged desktop file with `Exec` and `TryExec` naming `appimage`,
/// and the marker. Pure.
fn desktop_file(appimage: &Path) -> String {
    let path = appimage.to_string_lossy();
    let quoted = format!("\"{}\"", path.replace('\\', "\\\\").replace('"', "\\\""));
    let mut out = String::new();
    for line in PACKAGED.lines() {
        if line.starts_with("Exec=") {
            out.push_str(&format!("Exec={quoted}\n"));
            out.push_str(&format!("TryExec={path}\n"));
        } else if !line.starts_with("TryExec=") {
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push_str(MARKER);
    out.push('\n');
    out
}

/// Write `bytes` at `path` atomically, creating the folders.
fn write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let staging = path.with_extension("wipemark-tmp");
    std::fs::write(&staging, bytes)?;
    std::fs::rename(&staging, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh folder of this test's own under the system's temporary one.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("wipemark-integrate-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a folder");
        dir
    }

    #[test]
    fn the_desktop_file_names_the_appimage_and_is_marked_ours() {
        let file = desktop_file(Path::new("/home/me/Apps/Wipemark 0.1.0.AppImage"));
        assert!(file.contains("Exec=\"/home/me/Apps/Wipemark 0.1.0.AppImage\"\n"));
        assert!(file.contains("TryExec=/home/me/Apps/Wipemark 0.1.0.AppImage\n"));
        assert!(file.contains("StartupWMClass=wipemark\n"));
        assert!(file.contains("Icon=wipemark\n"));
        assert_eq!(
            file.matches("Exec=").count(),
            2,
            "Exec and TryExec, once each"
        );
        assert!(file.ends_with(&format!("{MARKER}\n")));
    }

    #[test]
    fn a_desktop_file_that_is_not_ours_is_left_alone() {
        let home = scratch("theirs");
        let desktop = home.join("applications/wipemark.desktop");
        std::fs::create_dir_all(desktop.parent().unwrap()).unwrap();
        std::fs::write(&desktop, "[Desktop Entry]\nExec=mine\n").unwrap();
        integrate(&home, Path::new("/a/W.AppImage")).expect("integrates");
        assert_eq!(
            std::fs::read_to_string(&desktop).unwrap(),
            "[Desktop Entry]\nExec=mine\n"
        );
        assert!(home
            .join("icons/hicolor/128x128/apps/wipemark.png")
            .exists());
    }

    #[test]
    fn ours_follows_the_appimage_and_writes_nothing_twice() {
        let home = scratch("ours");
        assert!(integrate(&home, Path::new("/a/W.AppImage")).unwrap());
        assert!(!integrate(&home, Path::new("/a/W.AppImage")).unwrap());
        assert!(integrate(&home, Path::new("/b/W.AppImage")).unwrap());
        let file = std::fs::read_to_string(home.join("applications/wipemark.desktop")).unwrap();
        assert!(file.contains("Exec=\"/b/W.AppImage\""));
    }

    #[test]
    fn a_cache_older_than_our_icons_is_seen_as_missing_them() {
        let home = scratch("cache");
        let hicolor = home.join("icons/hicolor");
        std::fs::create_dir_all(&hicolor).unwrap();
        std::fs::write(hicolor.join("icon-theme.cache"), b"theirs").unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        std::fs::File::options()
            .write(true)
            .open(hicolor.join("icon-theme.cache"))
            .unwrap()
            .set_modified(old)
            .unwrap();
        assert!(!cache_misses_us(&hicolor), "no icon of ours yet");
        integrate(&home, Path::new("/a/W.AppImage")).unwrap();
        let cache = std::fs::metadata(hicolor.join("icon-theme.cache")).unwrap();
        let folder = std::fs::metadata(&hicolor).unwrap();
        assert!(
            cache.modified().unwrap() > old || folder.modified().unwrap() > old,
            "the stale cache was rebuilt or the folder marked changed"
        );
    }
}
