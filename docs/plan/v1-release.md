# Wipemark 0.1.0 — the first release: what ships, what is accepted, what waits

*Coordinator, 2026-10-11, on the owner's word: "choose everything needed to
deploy the first version, start accepting it; everything else is documented
and goes to the next iteration, stage two". Decisions for the release are
D580–D589. `feat/e0-e6-shell` at `e648dc6` is the release candidate's base.*

## Для владельца

**Первая версия — это всё, что уже работает, без новых функций.** Сейчас
нужно только упаковать программу и принять её: собрать установщики,
проверить на чистой машине, пройти по списку приёмки.

**Что я выбрал** (меняется одним словом):

- **Платформы.** Linux x86_64 (AppImage и `.deb`, D609) и macOS на Apple Silicon (`.dmg`).
  Windows уходит во второй этап.
- **Без лицензии и активации.** Версия 0.1.0 бесплатная. Лицензия (E9) уходит
  во второй этап. Очистку лицензия не закрывала бы и так.
- **Решения, собранные на значениях по умолчанию, остаются как есть.** Это
  D447, D448, D450, D451, D419, D361 и вопросы Q-C2…Q-C6.
- **Профиль «Сохранить голос»** выбирается, но стандартным не становится.
- **Черновик DFlash** остаётся скрытым.
- **Знаки на картинках.** Gemini и Grok из X — как сейчас.

**Что нужно от вас:**

1. **«Да» на слияние PR #1 в `main`** и на тег `wipemark-v0.1.0`. Без вашего
   слова я `main` не трогаю.
2. **Подпись macOS.** Нужен сертификат Developer ID и данные для нотаризации
   в секретах GitHub. Если их нет, `.dmg` выйдет без подписи, и macOS при
   первом запуске это скажет. Тогда либо так, либо macOS позже.
3. **Подтвердить, что 0.1.0 бесплатная.**

## 1. What ships (D580)

Version **0.1.0**: the tree at the release candidate, nothing added but
packaging. In it:

| area | in 0.1.0 |
|---|---|
| Layer A, text | inspect / clean / audit; windows, panel, CLI, MCP |
| Layer B, rewrite | local engine (llama.cpp prebuilt `b10731`, Vulkan on Linux, Metal on macOS) and endpoints (Ollama, OpenAI-compatible); the batch queue, the journal, the Rewriting page, template profiles (Shipped default, Keep voice choosable) |
| pictures | metadata in PNG/JPEG/WebP; visible marks: Gemini V1/V2, Grok on X (D570–D573) |
| windows | main window, panel, Compare with Save/autosave, Settings (eight sections), first-launch walk-through, tray and shortcut (macOS; Linux under X11) |
| models | the catalogue (Qwen3 4B, Gemma 4 12B, Qwen3.8 27B), verified downloads, models the person adds |
| CLI | `inspect`, `clean`, `audit`, `rewrite`, `models` |

Everything the windows cannot do yet already says so out loud. Nothing is
hidden for the release except what is hidden today: the DFlash draft (D504).

**D581 — no licence in 0.1.0.** `wipemark-license` stays types-only. No
activation, no trial, nothing locked. E9 is stage two.

**D582 — the defaults stand.** D361, D419, D447, D448, D450 and D451 ship as
built. So do Q-C2 (AI provenance only), Q-C3 (Layer A's defaults), Q-C4 (refuse
and offer Replace), Q-C5 (strict) and Q-C6 (not built). Keep voice is not the
default.

**D583 — platforms.** Linux x86_64 and macOS arm64:

- **Linux.** An **AppImage** and a `.deb` (D609, the owner on 2026-10-12: an
  AppImage like MnemoVi's — it replaces the `.tar.gz`). Built on Ubuntu 24.04
  in a pinned image, because the prebuilt llama.cpp needs glibc 2.38; the
  floor the artifacts state is glibc 2.39 (D604). How:
  [packaging.md](../architecture/packaging.md).
- **macOS.** A `.app` inside a `.dmg`. Signed and notarized when the
  `APPLE_*` secrets exist; otherwise unsigned, and the release notes say so.

Windows, Flatpak and a glibc 2.36 build are stage two.

## 2. What has to be built for delivery (task `wipemark-task-v1-release-lane-2026-10-11`)

| | |
|---|---|
| **P1** | `.github/workflows/release.yml`, on a `wipemark-v*` tag and by hand (`workflow_dispatch`, which builds without publishing). It builds `wipemark` and `wipemark-cli` with `--features llama-native --locked --release` on `ubuntu-24.04` and `macos-14`. **Linux job: written** (D609) — the builder image, the AppImage and the .deb, both install gates, a draft release on a tag; not yet run on GitHub. macOS joins it. |
| **P2** | **The Linux packages — built** (`e10/appimage`, D600–D609; [report](reports/E10-appimage-2026-10-12.md)). `Wipemark-0.1.0-x86_64.AppImage` and `wipemark_0.1.0_amd64.deb`, one tree: <ul><li>`wipemark` and `wipemark-cli` in `/usr/lib/wipemark/` beside llama.cpp's libraries and backends, rpath `$ORIGIN`, links in `/usr/bin/`;</li><li>the `.desktop` file, the full hicolor set and AppStream metainfo;</li><li>NOTICE and THIRD-PARTY.md.</li></ul> The AppImage carries `libgomp`, `libxkbcommon-x11` and `libxcb-xkb` and runs the CLI by `--cli` or a link named `wipemark-cli`; the .deb declares them instead, its Depends computed from what the binaries NEED (`libgtk-3-0t64`, `libxkbcommon-x11-0`, `libvulkan1`, `libgomp1` among them) plus `libayatana-appindicator3-1`. Open: the windows' `app_id` (D605). |
| **P3** | **The macOS bundle.** `Wipemark.app`: <ul><li>llama.cpp's dylibs in `Contents/Frameworks`;</li><li>`wipemark-cli` beside the app binary;</li><li>the icon from `assets/icon/icon.icns`.</li></ul> It ships as a `.dmg`. `codesign` with hardened runtime and `notarytool` run only when the secrets exist. |
| **P4** | `tests/standalone.rs`'s check run on the **packaged** binaries: `wipemark --version` finds llama.cpp without cargo's loader path. **Linux: done** — `scripts/release/linux/build.sh` runs both `--version`s from the packed AppImage, the gates below from the AppImage and the installed .deb. |
| **P5** | **Licences — done for Linux** (D606). `THIRD-PARTY.md` is generated from `Cargo.lock` by `scripts/release/third-party.py` (from `cargo metadata`, no extra tool) and shipped beside `NOTICE`; it adds llama.cpp (MIT), the carried system libraries' copyright files, and says that model weights are downloaded, not shipped. |
| **P6** | **Smoke test on a clean machine**, as a script under `scripts/verify/release/` — **Linux: done** (D607): `appimage-gate.sh` (32 checks, the CLI on a bare ubuntu:24.04, the application on the declared floor under Xvfb) and `deb-gate.sh` (7 checks). The plan as written: Install the `.deb` in a fresh `ubuntu:24.04` container, then: <ul><li>`wipemark --version`;</li><li>`wipemark-cli inspect` and `clean` over `fixtures/clean-parity` and one Gemini and one Grok fixture, exits as the parity table says;</li><li>`wipemark` started under Xvfb for ten seconds with a scratch `WIPEMARK_DATA_DIR`: it starts, binds MCP, writes its beacon and exits on SIGTERM.</li></ul> On macOS the same CLI part on the runner. |
| **P7** | **Release notes**, `docs/release/0.1.0.md`: <ul><li>what it does;</li><li>install on each platform (and the unsigned-macOS step if it applies);</li><li>where data lives;</li><li>known limitations, which is §4's list in plain words.</li></ul> README gets an "Install" section. |
| **P8** | **The size of each artifact**, recorded in the report. No budget gate yet. |

Rules for P1–P8 are the repository's (CLAUDE.md):

- human authors only;
- nothing loosened;
- every script with its header;
- gates once at the end;
- `main` untouched by the agent; the tag and the merge of PR #1 are the
  coordinator's, after the owner's yes.

## 2a. The product's own licence — decided 2026-10-12 (D610)

**Decided:** `FSL-1.1-ALv2`, Copyright 2026 GigLabo and Denis Esakov (`LICENSE.md`, `NOTICE`, README). The repository stays public. Still open: where 0.1.0 is downloaded from, and the packages' metainfo and `.deb` copyright moved to the new licence when `e10/appimage` is merged. What follows is how it stood when found.

| | |
|---|---|
| **What is declared** | `Cargo.toml`: `license = "LicenseRef-Proprietary"` ("the EULA text itself lands with epic E9"). The AppStream metainfo on `e10/appimage`: `LicenseRef-proprietary`. `NOTICE`: "Copyright 2026 Wipemark contributors". |
| **What is missing** | No `LICENSE` or EULA text exists anywhere, so the AppImage and the `.deb` ship with no terms of use. The copyright holder is not a legal entity. |
| **The repository is public** | `GigLaboCom/wipemark-app` is **PUBLIC** on GitHub, with no licence: every line of source is readable and forkable. In law it is all rights reserved, but in practice it is open. MnemoVi's repository is private, and it publishes through its own release service (`scripts/ci/releases-publish.sh` in `mnemoria-lvkb`). A release on a public repository's GitHub Releases is public; on a private one it is not downloadable at all. |
| **Dependencies** | No copyleft binds the binary. <ul><li>The GPL-licensed crates are dual-licensed (`self_cell`: Apache-2.0 OR GPL-2.0; `r-efi`: MIT/Apache/LGPL).</li><li>MPL-2.0: `option-ext` ships unmodified; `cbindgen` is a build tool; `dwrote` is Windows only.</li><li>llama.cpp is MIT; the icons are CC BY 4.0 (`NOTICE`).</li><li>Model weights are not shipped.</li><li>`THIRD-PARTY.md`: 729 crates.</li></ul> |
| **Owner decisions** | <ol><li>The repository: private (like MnemoVi), or public on purpose (source-available).</li><li>Where 0.1.0 is downloaded from: GitHub Releases, or the release service MnemoVi uses.</li><li>The copyright holder's legal name.</li><li>The EULA for 0.1.0: proprietary, free of charge, no warranty. The coordinator drafts it and the owner (or a lawyer) approves; it ships as `LICENSE` in the packages and is shown in the About/first launch later (E9).</li></ol> |

## 3. Acceptance (the owner, on the packaged build)

Run on the `.deb` installed on the owner's host, and on the `.dmg` if there
is a Mac. Each line is pass or a finding. A finding is fixed before the tag
only if it is a crash, data loss, or a sentence that promises more than the
product does. Everything else goes to stage two.

| # | check |
|---|---|
| A1 | **Five minutes to the first rewrite.** Fresh data directory: install, open, walk through the first launch, download Qwen3 4B, then a first rewrite of a short text. Time it (E10's gate). |
| A2 | **Text clean.** Drop a text with zero-width characters: <ul><li>the row says what it found;</li><li>Clean writes `name.cleaned.ext` beside it;</li><li>Compare shows the difference;</li><li>Report… shows the three shelves;</li><li>Copy as Markdown works.</li></ul> |
| A3 | **Picture clean.** One Gemini sticker, one Grok picture from X and a photo with EXIF: <ul><li>the marks come off;</li><li>the photo keeps its camera data;</li><li>the Report names the vendor.</li></ul> |
| A4 | **Rewrite.** <ul><li>A row's Rewrite.</li><li>Rewrite all, with its price asked first.</li><li>Pause, Resume and Cancel.</li><li>The status bar says how far it has got.</li><li>The result is written to `name.rewritten.ext`.</li></ul> |
| A5 | **Endpoint.** Ollama on localhost on duty: the Check answers, a rewrite runs, and the Engine banner says where the text goes. |
| A6 | **Compare.** <ul><li>Edit, Save, autosave, and close with edits.</li><li>The gutters face the middle; switch to left and back.</li><li>Scroll sync.</li><li>Undo of Reset.</li></ul> These are the pending window checklists of E7-9, E7-10 and the Compare follow-ups. |
| A7 | **Settings.** <ul><li>Every section opens.</li><li>Choose the Keep voice profile and back.</li><li>Add a GGUF from the models folder.</li><li>Move the models folder.</li><li>Change the language to ru and de and back.</li></ul> |
| A8 | **Panel, tray, shortcut** (Linux under X11, and macOS). <ul><li>Summon the panel.</li><li>Drop on it, then Clean.</li><li>Close the main window, then Show from the tray.</li><li>Quit.</li></ul> |
| A9 | **CLI.** `inspect`, `clean`, `audit`, `rewrite` and `models list` with their exit codes. `rewrite` with the application running says it was served by the application. |
| A10 | **MCP.** Claude Code connected to `127.0.0.1:5056`; `inspect`, `clean`, `clean_image` and `rewrite` answer, and each is a journal row. |
| A11 | **Survives a restart.** <ul><li>The journal is still there after a quit and a launch.</li><li>A `kill -9` during a rewrite: it resumes from the chunk it reached.</li></ul> |
| A12 | **Uninstall.** Removing the package leaves the data directory and the models alone, and the release notes say where they are. |

## 4. Stage two — documented, not in 0.1.0

Each item already has its home in `docs/plan/README.md` §7. This list is the
release notes' "known limitations" and the next iteration's backlog.

- **E9, licensing**: activation, trial, offline grace, the signed manifest.
- **E10, the rest of packaging:**
  - Windows (tray, shortcut, installer);
  - Flatpak;
  - a build for glibc older than 2.38;
  - Wayland's close button and shortcut;
  - auto-update;
  - the size budget as a gate.
- **E7, workspace:** the source editor with badges (S7.2), the streamed
  result (S7.3), the Inspector (S7.5).
- **E12-8:** Compare for pictures, and the batch queue's picture item.
- **E12-6 and E12-7:**
  - other vendors;
  - the inpainter for marks over opaque pixels (Q-R3);
  - for Grok: sizes X has not yet given (one flat capture each: 1728×1152,
    1600×1200 and 1152×1712 on white), the search at unseen sizes, and the
    weak resized mark said clean in 15 of 720 (D573).
- **E2:**
  - the DFlash slowdown on Vulkan;
  - an engine for 8 GB cards (Qwen3.5-9B, MoE experts in RAM).
- **E4:** keep-voice as the shipped default, after the owner's decision.
- **Redo a row** — **built** and merged 2026-10-13 (D703–D708, `feat/redo-a-row`, `reports/redo-a-row-2026-10-13.md`): *Clean again* and *Rewrite again* on a finished row, over the row's own result found by its journal id; an edited result asks first; a result written in place is not redone (the owner, 2026-10-12, acceptance F10).
- **The defaults the owner may switch:**
  - D447, D448, D450, D451, D419, D361;
  - Q-C2–Q-C6;
  - Q8 (a "sign" mode);
  - D3 (a lossless JPEG written back at 95);
  - D5.
- **E3:** a mirror for downloads, garbage collection of old models.
- **Coverage** read and acted on (`coverage.yml`).
