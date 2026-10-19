# Wipemark 0.1.0 — the first release: what ships, what is accepted, what waits

*Coordinator, 2026-10-11, on the owner's word: "choose everything needed to
deploy the first version, start accepting it; everything else is documented
and goes to the next iteration, stage two". Decisions for the release are
D580–D589. `feat/e0-e6-shell` at `e648dc6` is the release candidate's base.*

**Released 2026-10-18.** The tag `wipemark-v0.1.0` is on `main` at `5df1564`
(PR #4's merge). Release run 39167096061 built the draft in 16 minutes. The
owner published it at 21:38 UTC, and the update run 39168719980 put the same
three files on releases.giglabo.com, signed with the working key
`24b967e7176fb3b2` and read back:

| file | bytes | sha256 |
|---|---|---|
| `Wipemark-0.1.0-arm64.dmg` | 25 244 765 | `49cffe62…6b1f45` |
| `Wipemark-0.1.0-x86_64.AppImage` | 46 279 160 | `15665a4b…419e17` |
| `wipemark_0.1.0_amd64.deb` | 28 766 776 | `ee16e06a…92ddab` |

The `.dmg` is notarized and stapled, and Gatekeeper accepts it with a
quarantine flag ("Notarized Developer ID"). The bundled
`wipemark-cli update --check` reads the live service and says 0.1.0 is the
newest verified version. Release page:
https://github.com/GigLaboCom/wipemark-app/releases/tag/wipemark-v0.1.0.

**0.1.1, 2026-10-19.** The main window's table scrolls sideways (D811–D813),
and gpui-kit is back on upstream `next` (D814). The tag `wipemark-v0.1.1` is
on `main` at `f0f2715` (PR #5's merge). Release run 39250367785 built the
draft in 17 minutes. The owner published it at 12:36 UTC, and update run
39253382415 put the files on releases.giglabo.com:

| file | bytes | sha256 |
|---|---|---|
| `Wipemark-0.1.1-arm64.dmg` | 25 232 916 | `9ef3eea4…93652c` |
| `Wipemark-0.1.1-x86_64.AppImage` | 46 262 776 | `99ec0a07…3eb81e` |
| `wipemark_0.1.1_amd64.deb` | 28 766 380 | `ff615c5b…b2220f` |

Every file was hashed from both GitHub and the service; the bytes are the
same. Released 0.1.0's own `wipemark-cli update --check` says "Wipemark
0.1.1 is available, and its signature verified". A self-update install from
0.1.0 has not been run yet. Notes: `docs/release/0.1.1.md`. Release page:
https://github.com/GigLaboCom/wipemark-app/releases/tag/wipemark-v0.1.1.

**0.1.2, 2026-10-19 — the version to advertise.** Update and restart no
longer ends Wipemark (D815): in 0.1.0 and 0.1.1 every way into an install
crashed before the network, so those install 0.1.2 by hand. The tag
`wipemark-v0.1.2` is on `main` at `aa6cd11` (PR #6's merge). Release run
39261984381 built the draft in 15 minutes; the owner published it at 14:51
UTC, and update run 39272695290 put the files on releases.giglabo.com:

| file | bytes | sha256 |
|---|---|---|
| `Wipemark-0.1.2-arm64.dmg` | 25 229 238 | `f5cc4fa9…bd92cc` |
| `Wipemark-0.1.2-x86_64.AppImage` | 46 266 872 | `cbb89694…cb4956` |
| `wipemark_0.1.2_amd64.deb` | 28 769 996 | `82bbda58…eadd5` |

The same bytes on GitHub and the service; released 0.1.0's and 0.1.1's
`update --check` find 0.1.2. Its notes, `docs/release/0.1.2.md`, are the
whole of 0.1.0's with what changed since. A self-update install is first
possible from 0.1.2 to the next version. Release page:
https://github.com/GigLaboCom/wipemark-app/releases/tag/wipemark-v0.1.2.

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
2. **Подпись macOS — готова, секреты на месте.** Подпись Developer ID и
   нотаризация идут в GitHub Actions при релизе (`release.yml`, D775–D779);
   на этом Mac подпись проверена, приложение с ней запускается и находит
   Metal. Шесть секретов `APPLE_*` добавлены в `GigLaboCom/wipemark-app`
   2026-10-16: сертификат и ключ App Store Connect проверены перед записью,
   соглашение Apple Developer Program принято, `notarytool` отвечает.
3. **Обновления (D780–D789).** Worker giglabo уже знает `wipemark`
   (`heretic-releases-api` `310f54f`). 2026-10-17: ключи сделаны и
   открытые половины закоммичены (0.1.0 — первый выпуск с обновлением),
   окружение `releases` заведено (только теги `wipemark-v*`), в нём
   `WIPEMARK_UPDATE_SIGNING_KEY` и `WIPEMARK_UPDATE_SIGNING_KEY_PASSWORD`;
   оба закрытых ключа с паролями — файл в Watchword
   `wipemark-update-signing-keys-2026-10-17` и
   `~/Documents/cert/wipemark-update-keys/`. `PUBLISH_TOKEN` сервиса
   сменён (старый из `mnemovi-publish-token` сервис уже не принимал): оба
   Worker, Woodpecker MnemoVi и `RELEASES_PUBLISH_TOKEN` здесь; значение —
   Watchword `giglabo-releases-publish-token-2026-10-17` и
   `~/Documents/cert/giglabo-releases-publish-token.txt`. Все три секрета
   обновления на месте. Пошагово — [packaging.md](../architecture/packaging.md).
4. **Подтвердить, что 0.1.0 бесплатная.**

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
  The signing and notarization run in the release lane on GitHub (the owner,
  2026-10-16: "notarization is done in the repo when we release"; D775–D779).

Windows, Flatpak and a glibc 2.36 build are stage two.

## 2. What has to be built for delivery (task `wipemark-task-v1-release-lane-2026-10-11`)

| | |
|---|---|
| **P1** | `.github/workflows/release.yml`, on a `wipemark-v*` tag and by hand (`workflow_dispatch`, which builds without publishing). It builds `wipemark` and `wipemark-cli` with `--features llama-native --locked --release` on `ubuntu-24.04` and `macos-14`. **Linux job: written** (D609) — the builder image, the AppImage and the .deb, both install gates. **macOS job: written** (D775–D779) — the build on `macos-14`, `bundle.sh`, and with the six `APPLE_*` secrets `sign-and-notarize.sh`; without them an ad hoc `.dmg` and a summary that says so; with only some of them a red job. lazy-shot's shape (D778): one release job per platform, each attaching its files to one draft release on a tag (`softprops/action-gh-release`), so one platform failing does not hold the other back; by hand, a platform to choose and the files in the run. `actionlint` passes; neither job has run on GitHub yet. |
| **P2** | **The Linux packages — built** (`e10/appimage`, D600–D609; [report](reports/E10-appimage-2026-10-12.md)). `Wipemark-0.1.0-x86_64.AppImage` and `wipemark_0.1.0_amd64.deb`, one tree: <ul><li>`wipemark` and `wipemark-cli` in `/usr/lib/wipemark/` beside llama.cpp's libraries and backends, rpath `$ORIGIN`, links in `/usr/bin/`;</li><li>the `.desktop` file, the full hicolor set and AppStream metainfo;</li><li>NOTICE and THIRD-PARTY.md.</li></ul> The AppImage carries `libgomp`, `libxkbcommon-x11` and `libxcb-xkb` and runs the CLI by `--cli` or a link named `wipemark-cli`; the .deb declares them instead, its Depends computed from what the binaries NEED (`libgtk-3-0t64`, `libxkbcommon-x11-0`, `libvulkan1`, `libgomp1` among them) plus `libayatana-appindicator3-1`. Open: the windows' `app_id` (D605). |
| **P3** | **The macOS bundle — built** (`scripts/release/macos/bundle.sh`, the acceptance's A14). `Wipemark.app`: <ul><li>llama.cpp's dylibs in `Contents/Frameworks`, ggml's backends beside the programs;</li><li>`wipemark-cli` beside the app binary;</li><li>the icon from `assets/icon/icon.icns`;</li><li>NOTICE, LICENSE.md and THIRD-PARTY.md (D779).</li></ul> It ships as a `.dmg` (`dmg.sh`). **Signing — written and proved locally** (D776, D777): `sign-and-notarize.sh` signs every Mach-O inside out with the Developer ID, hardened runtime and a secure timestamp, no entitlements; notarizes and staples the app, then the `.dmg`; Gatekeeper must say `Notarized Developer ID`. It runs only when the secrets exist; `bundle.sh` alone signs ad hoc. On this Mac the signed bundle verifies `--deep --strict`, starts under the hardened runtime and registers `BLAS, MTL0, CPU` from its own files; notarization waits for the secrets ([packaging.md](../architecture/packaging.md), macOS). |
| **P4** | `tests/standalone.rs`'s check run on the **packaged** binaries: `wipemark --version` finds llama.cpp without cargo's loader path. **Linux: done** — `scripts/release/linux/build.sh` runs both `--version`s from the packed AppImage, the gates below from the AppImage and the installed .deb. |
| **P5** | **Licences — done for Linux** (D606) **and macOS** (D779: `bundle.sh` writes it into `Contents/Resources/` for `aarch64-apple-darwin`). `THIRD-PARTY.md` is generated from `Cargo.lock` by `scripts/release/third-party.py` (from `cargo metadata`, no extra tool) and shipped beside `NOTICE`; it adds llama.cpp (MIT), the carried system libraries' copyright files, and says that model weights are downloaded, not shipped. |
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
