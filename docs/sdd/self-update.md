# Self-update: how Wipemark learns of a new version and replaces itself

This document studies how lazy-shot finds out that a new version exists, and
the release service behind it on `giglabo.com`. It then designs the same
thing for Wipemark, which is not a Tauri application: a check of signed
JSON metadata, a verified download, and an install that replaces the
program on macOS and in an AppImage. It is research and design only; no
product code changes with it.

**Provenance of this survey.**

* Written 2026-10-16 on branch `sdd/self-update`, from `feat/e0-e6-shell`
  at `d25ac4e`. The owner asked on 2026-10-16: "we need a check for a new
  version so the program can update itself… through giglabo, the lazy-shot
  way — how to know there is an update, through JSON metadata."
* Decisions **D780–D789** are reserved for this line. Every one is
  ***proposed*** until the owner says yes (§9).
* Read-only, nothing modified in any of them:
  * `heretic-lazy-shot` at **`620de44`** (2026-09-23),
    `~/self/heretic-lazy-shot`.
  * `heretic-releases-api` at **`35f9673`** (2026-07-21),
    `~/self/heretic-releases-api`. This is the Cloudflare Worker behind
    `releases.giglabo.com`. **The deployed Worker is newer than this
    clone.** Its manifest carries `format_preference` and `format_tokens`,
    and MnemoVi's publish script names `src/format-token.ts` and a
    `replace` flag at finalize. None of those is in this clone (§1.6).
  * `mnemoria-lvkb` (MnemoVi) at **`c53c721`** (2026-09-11).
  * Watchword: `heretic-lazy-shot-autoupdate-investigation` (2026-06-25),
    `heretic-lazy-shot-update-check-explained` (2026-07-11) and
    `owner-questions-mnemovi-update-check-2026-09-24`.
  * `gpui-pre` 0.3.8 from crates.io and the carried `gpui-pre-linux`, for
    `App::restart`.
* Fetched once each on 2026-10-16 at 11:08 UTC, without authentication:
  `https://releases.giglabo.com/heretic-lazy-shot/manifest.json` and
  `https://releases.giglabo.com/downloads/mnemovi` (§1.4, §1.6).
* Not read: Zed's own updater (there is no checkout here) and Sparkle's
  source. What §6.2 says of Sparkle comes from its documentation. Check it
  before relying on it.

---

## Для владельца

**Как сделано у lazy-shot.** Это не updater Tauri: в конфиге нет
`plugins.updater` и `pubkey`, а файлы `.sig` не собираются. Приложение раз в
сутки скачивает публичный JSON
`https://releases.giglabo.com/heretic-lazy-shot/manifest.json` и сравнивает
версии. Если есть новее, оно показывает окно «Download / Later / Skip».
Download открывает `.dmg` в браузере, а ставит человек сам. JSON собирает
Worker `heretic-releases-api`: по вебхуку GitHub `release: published` он
зеркалирует файлы релиза в R2. Ни JSON, ни файлы не подписаны: доверие
держится только на HTTPS и домене. MnemoVi устроен так же, но публикует
прямо из CI (`/publish/*`). В его манифесте есть sha256 каждого файла, но
клиент его не проверяет.

**Что предлагаю для Wipemark:**

1. **Тот же сервис и тот же JSON:** `https://releases.giglabo.com/downloads/wipemark`,
   по способу MnemoVi (`direct`). JSON пишет новый workflow в момент, когда
   вы нажимаете Publish у черновика релиза, а не при теге. Иначе обновление
   получат раньше, чем вы опубликовали релиз.
2. **Подпись.** Каждый файл подписан ключом Ed25519. Подписывается короткая
   «справка о выпуске»: версия, платформа, имя файла, размер и sha256.
   Публичный ключ вшит в программу. Приложение не скажет «есть новая
   версия» и ничего не скачает, пока подпись не сойдётся. На Mac
   дополнительно проверяются Developer ID команды `M8AM5QVKBL` и ответ
   Gatekeeper «Notarized Developer ID».
3. **Согласие.** Автопроверка выключена, пока человек не ответит «да».
   Вопрос задаётся один раз, на последнем шаге первого запуска. Кнопка
   «Проверить сейчас» есть всегда. Запрос — голый GET без идентификаторов
   и без номера версии. Фразу «ничего не уходит с машины» нужно
   переписать: сервер увидит IP-адрес.
4. **Установка.**
   * Mac: скачать `.dmg`, проверить его, положить новую копию рядом с
     `/Applications/Wipemark.app`, атомарно поменять их местами и
     перезапуститься.
   * AppImage заменяет себя на том же пути.
   * `.deb` только сообщает о новой версии: им владеет apt.
   * Ничего не ставится, пока идёт очистка, переписывание или загрузка
     модели.
   * Sparkle не берём. Он только для Mac, у него свой интерфейс и свои
     строки, а для Linux всё равно нужен свой код.
5. **Worker.** В `heretic-releases-api` нужно две правки: зарегистрировать
   `wipemark` и передавать поле `sig` в манифест (тип для него уже есть,
   `PlatformMeta.sig`). Это ваш репозиторий, я его не трогал.
6. **Версия 0.1.0** обновляться не умеет. 0.2.0 люди поставят руками, а
   дальше обновления придут сами.

**Что нужно от вас:** ответы в §10. Главные — Q1 (умолчание), Q2 (когда
публиковать) и Q5 (ключи).

---

## 0. The answer in one page

* **lazy-shot has no updater in Tauri's sense.** It does not use
  `tauri-plugin-updater`, `plugins.updater`, a `pubkey` or a `latest.json`.
  Its "Route A" is notify-only:
  * fetch a public JSON once a day;
  * compare the version with semver;
  * open the matching `.dmg` in the browser.

  Nothing is signed or verified. Trust is TLS plus the
  `releases.giglabo.com` origin. Its own comment says so
  (`updateChecker.ts:5-10`). The JSON is written by a Cloudflare Worker,
  `heretic-releases-api`, which mirrors published GitHub releases into R2.
  `TAURI_SIGNING_PRIVATE_KEY` is passed to the build, but nothing the
  build makes is signed with it (§1.5).
* **MnemoVi took the same client and a newer server route, "direct":**
  * CI publishes each artifact itself (`/publish/*`, a bearer token, a
    presigned PUT to R2);
  * the Worker assembles the manifest on read, with a `sha256` per asset.

  The Worker's types already reserve an Ed25519 `sig` per asset
  (`types.ts:95-96`). Nothing fills it, and nothing serves it.
* **For Wipemark, the "how do I know" part stays lazy-shot's.** It is the
  same service and the same JSON shape, at
  `https://releases.giglabo.com/downloads/wipemark`, on MnemoVi's `direct`
  route (D780). What Wipemark adds is what the owner's other products
  lack:
  * **A signature the application checks before it says anything** —
    Ed25519 over a short per-artifact *release statement*. The statement
    holds the version, platform, file, size and sha256, and the public key
    is compiled into the program (D781, D782).
  * **On macOS**, the new bundle must also satisfy the Developer ID
    requirement of team `M8AM5QVKBL`, and Gatekeeper must say `Notarized
    Developer ID` (D786).
* **The metadata is published when a person publishes the release, not at
  the tag.** The release lane leaves a draft, and a person publishes it
  (D778). A new workflow on `release: published` signs the release's own
  files and pushes them to giglabo. Then it reads the served manifest back
  and verifies it with the committed public key (D783).
* **The update check is off until the person says yes.** It is the first
  time the application would reach the network on its own. It asks once,
  in the walk-through's last step. "Check now" always works, because it
  is an explicit act like a model download. The request is a bare GET with
  a fixed user agent and no version. The printed promise "the only
  connections Wipemark makes… are the model downloads you start and the
  endpoint you configure" is reworded to name the check (D784).
* **What the person sees.** A status-bar line and a tray item when a
  verified version is waiting, and a row in Settings › General. There is
  no modal for an automatic check (D785).
* **How it installs:**
  * **macOS** (D786): download the notarized `.dmg`, verify it, mount it,
    stage `Wipemark.app` beside the installed one, check it, swap the two
    atomically (`renamex_np(RENAME_SWAP)`), and relaunch through GPUI's own
    `App::restart`. We do not use Sparkle.
  * **AppImage** (D787): replace the file at `$APPIMAGE` by `rename`, and
    relaunch the same path.
  * **.deb** (D787): notify only; apt owns `/usr/lib/wipemark`.
  * **Every other case is refused, and the app only notifies.** That
    covers no write access, a translocated or unsigned copy, and a
    development build.
* **Never under work (D788).** Update is greyed out while any of these
  runs, and the greyed item says why:
  * a clean;
  * a rewrite;
  * a model download;
  * a Compare edit that autosave would not keep.

  The restart goes through the quit path that shuts the engine down
  (D740–D744). `wipemark-cli update --check` reads the same signed
  metadata (exits 0, 1 or 2). The CLI installs nothing, and MCP gets no
  tool for it.
* **The code (D789).** One new leaf crate, `wipemark-update`, with no
  workspace dependency and **no new package in `Cargo.lock`**. `ring` is
  already there through rustls, and so are `ureq`, `sha2` and `semver`.
  The application gets `update.rs`, the CLI gets a subcommand, and
  `scripts/check-dep-direction.sh` gets one entry. Every test in §8.7
  must first be seen red.

---

## 1. What lazy-shot does

### 1.1 It is not Tauri's updater

| where | what is there |
|---|---|
| `packages/client/src-tauri/tauri.conf.json:36-40` | `plugins` holds only `sql`. There is **no `plugins.updater`**, so no `endpoints` and no `pubkey`. |
| `packages/client/src-tauri/tauri.conf.json:26` | The release CSP's `connect-src` names `https://releases.giglabo.com`. This is the only trace of updates in the Tauri configuration. The webview's `fetch` needs it (`updateChecker.ts:12-14`). |
| `packages/client/src-tauri/Cargo.toml:9-16, 36` | The plugins are sql, opener, shell, os, global-shortcut, dialog, autostart, single-instance and drag. **No `tauri-plugin-updater`.** |
| anywhere under `packages/` or `.github/` | No `createUpdaterArtifacts`, so Tauri's bundler never makes `.app.tar.gz` + `.sig` update bundles. |

The Watchword investigation of 2026-06-25 names two routes:

* **Route A** — "a lightweight custom Yes/No modal" over the existing
  manifest.
* **Route B** — `tauri-plugin-updater`, "a DIFFERENT, SIGNED manifest"
  with minisign signatures and update bundles.

It chose A: "NO signature verification … a plain download manifest". A is
what shipped.

### 1.2 The check, step by step (client)

All paths are under `packages/client/src/`.

| step | file:line | what it does |
|---|---|---|
| URL | `services/updateChecker.ts:16-17` | `MANIFEST_URL = 'https://releases.giglabo.com/heretic-lazy-shot/manifest.json'`. The fallback is a downloads page (`:20-21`). |
| trust | `services/updateChecker.ts:5-10` | "No silent self-update, no signature verification — this is a plain download manifest". |
| fetch | `services/updateChecker.ts:72-86` | GET, `cache: 'no-store'`, `accept: application/json`. It throws on a non-2xx status or a wrong shape. The shape check is only "`versions` is an array of `{version: string, assets: array}`" (`:56-65`). |
| newer? | `services/updateChecker.ts:89-110` | A semver of its own. It strips a leading `v` and drops everything after `-` or `+`, so **`0.1.0-rc.1` equals `0.1.0`**. |
| newest | `services/updateChecker.ts:169-174` | The maximum by semver; it does not trust the manifest's order. |
| offer? | `services/updateChecker.ts:177-185` | Strictly newer, and not the version the person skipped. |
| which file | `services/updateChecker.ts:49-54, 151-166` | It filters by `os` and `arch`, then a preferred format (`linux: ['appimage', 'deb']`), and otherwise the first match. |
| when | `components/InitProvider.tsx:121-131` | One check once init is done, then a tick every hour that re-evaluates a 24 h gate. |
| gate | `stores/updateStore.ts:18, 63-70` | `UPDATE_CHECK_INTERVAL_MS = 24h`. An automatic check is skipped if the last *successful* fetch is under 24 h old. A manual check always runs. |
| state | `stores/settingsStore.ts:30-33, 126-127, 280-296` | Two keys: `update_last_check_at` (ISO time) and `update_skipped_version`. **There is no on/off preference.** The automatic check always runs. |
| skip | `stores/updateStore.ts:88-89, 141-148` | A skipped version is not offered automatically again. A manual check ignores the skip. |
| errors | `stores/updateStore.ts:121-124` | An automatic check fails silently. A manual one shows an error modal. |
| download | `stores/updateStore.ts:130-139` | `openUrl(asset.url)` hands the `.dmg` to the browser. The person installs it. |

### 1.3 What the person sees

* **The modal** (`components/UpdateModal.tsx:10-99`) has three modes.
  * `available` — "Update available" / "Version {{version}} is available.
    Download now?", with **Skip this version**, **Later** and
    **Download** (`:65-85`).
  * `uptodate` and `error` — only after a manual check.
* **The strings** are in `i18n/locales/en/common.json:262-275`, with ru
  and de beside them.
* **Settings › General** (`components/settings/GeneralSettings.tsx:284-303`)
  has one row, "Updates", showing "Current version: {{version}}" and a
  **Check for Updates** button (`i18n/locales/en/settings.json:41-44`).

### 1.4 Where the JSON comes from: `heretic-releases-api`, the mirror route

`packages/cf-worker/` in lazy-shot is a different Worker: the wallpaper
media API on `heretic-lazyshot-media.giglabo.com` (`README.md:1-7`,
`wrangler.toml:1-14`). The release JSON comes from the separate repository
`heretic-releases-api`:

* **The deployment.** `README.md:20-29`: the Worker
  `heretic-releases-api.denis-yesakov.workers.dev`, R2 bucket
  `heretic-releases` on the public domain `releases.giglabo.com`, an hourly
  cron, and "Releases" webhooks on the lazy-shot and telegram-post-editor
  repositories. The Worker's routes on that domain are `/downloads/*`,
  `/publish/*`, `/proxy/*` and `/dl/*` (`wrangler.toml:17-24`). Everything
  else, such as `/heretic-lazy-shot/manifest.json`, is served by R2
  directly.
* **The trigger.** `src/index.ts:136-151` acts only on `release` +
  `published`. It verifies the GitHub HMAC (`src/webhook.ts:24-41`), then
  runs `syncUntilVersionReady`, which retries because "GitHub's REST
  release list can lag the `published` webhook".
* **The sync** (`src/sync.ts:23-95`):
  * list the releases;
  * **drop drafts** and prereleases;
  * sort by semver and keep N (`keep: 3`, `src/apps.ts:21`);
  * never prune to empty;
  * mirror assets by HEAD-before-PUT;
  * write `<slug>/manifest.json` **last** (`:68-74`);
  * POST the site's cache reset only when something changed (`:80-86`,
    `src/site-cache.ts:24-55`).
* **Matching assets.** `DEFAULT_ASSET_REGEX` is
  `^.+_(\d+\.\d+\.\d+)_(aarch64|x86_64|x64|amd64)[^.]*\.(dmg|msi|exe|deb|AppImage|rpm)$`
  (`src/apps.ts:16-17`). **It matches none of Wipemark's macOS or
  AppImage names.** Neither `Wipemark-0.1.0-arm64.dmg` nor
  `Wipemark-0.1.0-x86_64.AppImage` has `_<version>_`, and `arm64` is not
  in the group. Only `wipemark_0.1.0_amd64.deb` matches.
* **The manifest built** (`src/manifest.ts:13-34`) has `{os, arch, format,
  filename, url, size}` per asset. It has **no sha256 and no signature**.
* **The version order** (`src/semver.ts:5-23`) also ignores suffixes.

lazy-shot's release job also POSTs the site's reset itself
(`.github/workflows/build-client.yml:331-346`). That is best-effort, and so
is the Worker's own reset.

**The real JSON**, fetched 2026-10-16 at 11:08 UTC, shown here with only
the first of its three versions:

```
HTTP/2 200
content-type: application/json
content-length: 2189
etag: "21b751f252a763086cb1759b51af7920"
last-modified: Fri, 16 Oct 2026 11:01:02 GMT
cf-cache-status: DYNAMIC
```

```json
{
  "slug": "heretic-lazy-shot",
  "updated_at": "2026-10-16T11:01:02.056Z",
  "versions": [
    {
      "version": "0.0.10",
      "released_at": "2026-09-23T11:41:38Z",
      "assets": [
        {
          "os": "macos",
          "arch": "aarch64",
          "format": "dmg",
          "filename": "Heretic.Lazy.Shot_0.0.10_aarch64.dmg",
          "url": "https://releases.giglabo.com/heretic-lazy-shot/0.0.10/Heretic.Lazy.Shot_0.0.10_aarch64.dmg",
          "size": 18500445
        },
        {
          "os": "macos",
          "arch": "x64",
          "format": "dmg",
          "filename": "Heretic.Lazy.Shot_0.0.10_x64.dmg",
          "url": "https://releases.giglabo.com/heretic-lazy-shot/0.0.10/Heretic.Lazy.Shot_0.0.10_x64.dmg",
          "size": 19461075
        }
      ]
    }
  ]
}
```

The other two entries are 0.0.9 (2026-08-12) and 0.0.8 (2026-07-25), each
with the same two assets. The object has no `cache-control`. Its
`last-modified` is the last hourly cron: `putManifest` runs on every sync,
changed or not (`src/sync.ts:68-74`).

### 1.5 How CI produces the artifacts (lazy-shot)

`.github/workflows/build-client.yml`, on a `v*` tag or by hand:

* **macOS** (`:23-216`), a matrix of aarch64 and x86_64:
  * `tauri-action` builds `bundle.targets: ["app"]` only (`:147`);
  * it is given `TAURI_SIGNING_PRIVATE_KEY` and `_PASSWORD` (`:143-144`),
    but no updater bundle is configured, so **no `.sig` is made**;
  * `dylibbundler`, then `codesign` of every dylib and of the app with
    `--deep` (`:169-182`);
  * `create-dmg`, then sign, notarize and staple **the .dmg only**
    (`:187-210`);
  * the upload is `*.dmg` and nothing else (`:212-216`).
* **Windows** (`:219-303`, disabled `if: false`). It would upload
  `.exe.sig` and `.msi.sig` (`:301-303`): the one place Tauri's updater
  signature would have appeared.
* **Release** (`:309-346`): `softprops/action-gh-release` with
  **`draft: false`** and generated notes (`:327-329`), so the release is
  public at once. That fires the `published` webhook, and the reset POST
  follows.
* **The reference document disagrees with the workflow.**
  `docs/reference-github-actions.md` §5 says a tag creates a **draft** that
  a person publishes (`:272-304`), but the workflow publishes. Its §4
  "Tauri Updater Signing" says the key "signs update manifests"
  (`:259-266`); nothing in the workflow does that. The secrets table names
  `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` for
  an "Ed25519 private key" from `tauri signer generate` (`:383-388`).

### 1.6 The newer route: direct publish (MnemoVi)

In `heretic-releases-api` at `35f9673`:

* **Two sources** (`src/types.ts:9-16`):
  * `'mirror'` — pull, the webhook above;
  * `'direct'` — push: "Woodpecker agents publish each OS artifact
    independently via /publish/*, and the manifest is DERIVED on read".
* **The registry** (`src/apps.ts:31-37`) registers `mnemovi` as `direct`,
  with `expected_platforms`.
* **The write side** (`src/index.ts:280-313`, `src/publish.ts`):
  * `POST /publish/:slug/:version/open` writes a write-once `release.json`
    intent with `channel` (default `stable`), `notes_url`, `git_tag` and a
    deadline (`src/publish.ts:55-102`);
  * then each platform's artifact goes in, either by `PUT` or by a
    presign → direct-to-R2 PUT → finalize;
  * the Worker computes **`sha256`** and writes a `meta.json` beside the
    file (`src/publish.ts:110-164`);
  * every write is bearer-authenticated with **`PUBLISH_TOKEN`**, one
    token for every direct app (`src/types.ts:199-203`).
* **The read side** (`src/assemble.ts:73-160`):
  * a projection of whatever per-platform objects exist;
  * `status` is building, complete or partial;
  * `latest_by_platform` is "the newest version that actually carries an
    asset for each key" (`:143-151`);
  * served at `GET /downloads/:slug` with `cache-control: public,
    max-age=60` (`src/index.ts:168-174`).
* **The signature that is designed but not wired.** `PlatformMeta.sig` is
  "std-base64 raw Ed25519 detached signature, if the app signs artifacts"
  (`src/types.ts:95-96`). Nothing writes it: neither the upload nor the
  finalize accepts one. `assemble.ts:110-119` does not copy it, and
  `ManifestAsset` has no such field (`src/types.ts:131-143`). The Worker
  can verify Ed25519 (`src/verify.ts:41-59`), but only for its
  third-party download proxy.

MnemoVi's lane is `scripts/ci/releases-publish.sh` in `mnemoria-lvkb`:

* open, presign, PUT, finalize (`:90-135`);
* `FORMAT` is a canonical, case-sensitive token (`:74-85`);
* finalize refuses a second filename for one `(os, arch, format)` slot
  unless `REPLACE=1` is set (`:119-134`). That is the deployed Worker's
  behaviour, newer than the clone read here;
* then **verify what is served** (`:137-192`). The manifest must list
  this filename and sha256, the public download path must resolve to this
  file, and a HEAD must return the declared size.

`scripts/ci/published-artifact-gate.sh:1-24` goes one step further and
sweeps the published artifact as a user receives it. Its stated reason is
that on 2026-08-05 a notarized, Gatekeeper-accepted `.dmg` "cannot start
on any machine", and every gate had checked only the file just built.

MnemoVi's client (`packages/client/src/services/updateChecker.ts`) is
lazy-shot's Route A "ported verbatim" (`:1-13`). Its header records that
the URL was wrong for the app's whole life (`:25-49`): the slug and the
shape were both wrong, so the check "has … never worked in any published
build". It now reads `https://releases.giglabo.com/downloads/mnemovi`
(`:49`). It still ignores the per-asset `sha256` (`:36-39`), and "Trust is
anchored in HTTPS + the releases.giglabo.com origin" (`:11`). It also
stopped falling back across formats: "NAMING A FORMAT GETS THAT FORMAT OR
NOTHING" (`:212-267`).

The owner's open questions on it (Watchword,
`owner-questions-mnemovi-update-check-2026-09-24`):

* **UC-Q3:** `channel` is filtered by neither side.
* **UC-Q4:** both sides drop prerelease suffixes, so an `rc` would be
  offered instead of its release.

**The real JSON**, fetched once on 2026-10-16 at 11:08 UTC:
`cache-control: public, max-age=60`, 11 versions. The top level, and the
first entry:

```json
{
  "slug": "mnemovi",
  "updated_at": "2026-10-16T11:09:15.288Z",
  "latest_version": "0.3.2",
  "latest_by_platform": { "macos-aarch64": "0.3.2", "windows-x64": "0.1.1", "linux-x64": "0.3.0" },
  "format_preference": {
    "linux-x64": ["AppImage", "deb"], "linux-aarch64": ["AppImage", "deb"],
    "macos-aarch64": ["dmg"], "macos-x64": ["dmg"], "windows-x64": ["nsis", "msi"]
  },
  "format_tokens": ["AppImage", "deb", "rpm", "dmg", "pkg", "nsis", "msi", "exe"],
  "versions": [
    {
      "version": "0.3.2",
      "released_at": "2026-08-21T20:27:22.976Z",
      "assets": [
        {
          "os": "macos", "arch": "aarch64", "format": "dmg",
          "filename": "MnemoVi_0.3.2_aarch64.dmg",
          "url": "https://releases.giglabo.com/mnemovi/0.3.2/macos-aarch64/MnemoVi_0.3.2_aarch64.dmg",
          "size": 103955834,
          "sha256": "a7afb60f0575c45fc2139e76f182cca58d9b4aa1cbe75e06a86ac258453b8743",
          "built_at": "2026-08-21T20:27:32.938Z"
        }
      ],
      "channel": "stable",
      "status": "complete",
      "expected": ["macos-aarch64"]
    }
  ]
}
```

### 1.7 What we take, and what we reject

| from lazy-shot / MnemoVi | Wipemark |
|---|---|
| One public JSON per product on `releases.giglabo.com`, written by the Worker | **taken** (D780) |
| The `direct` route: CI holds only a publish token, the bytes go to R2 by presign, and the manifest is assembled on read with a sha256 | **taken** (D780, D783) |
| Verify the served document after publishing (`releases-publish.sh:137-192`), and gate what is served, not what was built | **taken**, with signature verification added (D783) |
| At most one automatic check a day, a manual check that always runs, skip a version, silent automatic failures | **taken** (D784, D785) |
| A format named gets that format or nothing (MnemoVi `:212-267`) | **taken**: the installation decides the format, with no fallback (D781) |
| Compute the newest version yourself; do not trust `latest_by_platform` (UC-Q2) | **taken** |
| Trust is HTTPS plus the origin; no signature; sha256 ignored | **rejected**: an updater that installs needs a root of trust independent of TLS and of whoever holds the Cloudflare account (D781, D782) |
| The check runs from the first launch, with no preference to turn it off | **rejected**: it is off until the person says yes (D784) |
| A modal pops up when an automatic check finds a version | **rejected**: a status-bar line and a tray item (D785) |
| Publish at the tag (`draft: false`) | **rejected**: the draft stays a draft (D778), and metadata goes out at Publish (D783) |
| Semver that drops prerelease suffixes | **rejected**: `semver::Version` ordering, and stable refuses a prerelease outright (D781) |
| Mirror route with `DEFAULT_ASSET_REGEX` | **not used**: it matches neither of our macOS or AppImage file names, and it has no sha256 (§1.4) |

---

## 2. The metadata (D780, D781, D783)

### 2.1 Where it lives (D780)

| | |
|---|---|
| **manifest** | `https://releases.giglabo.com/downloads/wipemark`, the Worker's derived manifest for slug **`wipemark`**, registered `source: 'direct'`, `expected_platforms: ['macos-aarch64', 'linux-x64']`, `keep: 5` |
| **artifacts** | `https://releases.giglabo.com/wipemark/<version>/<platform>/<file>`, the shape `assemble.ts:115` writes; R2's public domain serves them, with no redirect |
| **platforms and formats** | `macos-aarch64` + `dmg` (`Wipemark-<v>-arm64.dmg`); `linux-x64` + `AppImage` (`Wipemark-<v>-x86_64.AppImage`); `linux-x64` + `deb` (`wipemark_<v>_amd64.deb`). These are all canonical tokens of the deployed Worker (§1.6), so the Worker needs no new token. |
| **the URL in the program** | a constant in `wipemark-update`. A release build has no way to change it. A debug build may point it at a loopback server for the live check (§8.8) |

The alternative is **lazy-shot's exact route** (mirror): register
`wipemark` as a `mirror` app with `repo: 'GigLaboCom/wipemark-app'` and an
`assetRegex` for our names, and add a "Releases" webhook. It needs no CI
change, and a person's Publish is its trigger by construction. Its cost:

* no sha256 in the manifest;
* the signature would have to travel as extra release assets, paired by
  the Worker, which is more Worker code than the `direct` change;
* the GitHub App must also be installed on `wipemark-app`.

Recommended: `direct`, which is what the owner's newest product uses (Q3).

### 2.2 The release statement (D781)

A signature over the whole manifest is impossible on the `direct` route,
because the Worker assembles the document on every read. What is signed
is therefore **one statement per artifact**. The statement is a few lines
of text that bind everything the application acts on:

```
wipemark-update 1
channel stable
version 0.2.0
platform macos-aarch64
format dmg
file Wipemark-0.2.0-arm64.dmg
size 24117248
sha256 9f2c…(64 lowercase hex)
```

The exact form:

* UTF-8, these eight lines in this order, one space after the name, each
  line ending in `\n`, with no other whitespace;
* `file` is a basename: no `/`, no control character;
* `version` parses as `semver::Version`, and on `stable` it carries no
  prerelease.

The signature is **Ed25519 over exactly those bytes** (D782). It travels
in the manifest as one new string per asset:

```json
"sig": "wm1:<key id, 16 hex>:<base64 of the 64-byte signature>"
```

**The application never reads a statement from the network.** It
rebuilds the statement from the manifest's own fields:

| statement line | taken from |
|---|---|
| `channel` | the version entry's `channel` |
| `version` | the version entry's `version` |
| `platform` | the asset's `os` and `arch`, joined as `<os>-<arch>` |
| `format` | the asset's `format` |
| `file` | the asset's `filename` |
| `size` | the asset's `size` |
| `sha256` | the asset's `sha256` |

It then verifies the signature against that text. A manifest altered in
any of those fields fails, and so does one with a field missing. **The
URL is not in the statement.** The application builds the URL itself as
`https://releases.giglabo.com/wipemark/<version>/<platform>/<file>`, and
refuses an asset whose `url` differs. The bytes are pinned by the sha256,
so the URL check is not there for integrity. It is there for privacy: a
forged manifest must not be able to send this computer to another host.

What the application reads, and what it ignores:

| read and covered by the signature | read, not signed | ignored |
|---|---|---|
| `versions[].version`, `channel`, `assets[].os`, `arch`, `format`, `filename`, `size`, `sha256`, `sig` | `assets[].url` (it must equal the URL the program builds) | `latest_version`, `latest_by_platform`, `status`, `expected`, `missing`, `failed`, `format_preference`, `format_tokens`, `updated_at`, `released_at`, `built_at` |

No date is shown, because none is signed. The release notes link is not
read from the manifest either. The program builds it from the version:
the public GitHub release page
`https://github.com/GigLaboCom/wipemark-app/releases/tag/wipemark-v<version>`,
or a giglabo page if the owner prefers (Q8).

### 2.3 How the application chooses

1. GET the manifest, refusing a body over 256 KiB.
2. Parse it with serde. A wrong shape is a refusal, never a guess.
3. Keep the entries with `channel == "stable"` and a version **strictly
   greater** than the running one (`env!("CARGO_PKG_VERSION")`), and sort
   them in descending order.
4. In each entry, look for the asset of **this installation's** platform
   and format (§6.0). There is no fallback to another format.
5. The first asset whose statement verifies is **the offer**. An entry
   that fails is logged as a warning (`an announced version did not
   verify`, with its version and the reason) and skipped; it is never
   shown as an offer.
6. A version the person skipped is not offered by an automatic check. A
   manual check offers it again.

### 2.4 Versions and channels

* **Versions** are the workspace version, the tag `wipemark-v<version>`
  and `CFBundleShortVersionString`. They are already one number
  (`bundle.sh:134`).
* **Channel `stable` only** for now. `beta` is a later decision. It needs
  the Worker's `latest_by_platform` to respect the channel (UC-Q3), and
  prerelease ordering on the Worker side (UC-Q4). The application already
  orders prereleases correctly and filters by channel itself, so a beta
  needs a row and no change to the format.

### 2.5 Who writes it, and when (D783)

**On the person's Publish, not on the tag.** `release.yml` leaves a draft,
and "a person publishes, with `docs/release/<version>.md` as the notes"
(packaging.md, D778). If the release lane published to giglabo at the tag,
every installation would be offered the version before the person had
published it. A new workflow therefore listens for that click.

`.github/workflows/publish-update.yml`:

* **Triggers:**
  * `release: types: [published]` — a person's Publish in the UI fires
    it; a release made with `GITHUB_TOKEN` would not;
  * `workflow_dispatch` with a tag, to run it again by hand.
* **The job.** It runs on `ubuntu-24.04` in the **GitHub Environment
  `releases`**, whose deployment rule admits only tags `wipemark-v*`, so a
  branch cannot reach the secrets. Its steps:
  1. `gh release download <tag>`: the `.dmg`, the AppImage, the `.deb`,
     `SHA256SUMS` and the `.dmg.sha256`. All three artifacts must be
     present, or the job stops.
  2. Recompute each sha256 and compare it with the sums the build jobs
     wrote.
  3. **Recommended:** check that the bytes came from this repository's
     own build. Each build job adds an `actions/attest-build-provenance`
     step for its artifacts (`id-token: write`, `attestations: write`),
     and this job runs `gh attestation verify <file> -R
     GigLaboCom/wipemark-app` before it signs. Someone who could replace
     a release asset could also replace `SHA256SUMS`; they cannot forge
     the attestation.
  4. Open the version on giglabo:
     `POST /publish/wipemark/<v>/open {channel, git_tag, expected_platforms}`.
  5. For each artifact:
     * write the statement (§2.2);
     * sign it (D782);
     * presign, PUT and finalize with `sig`, as MnemoVi's
       `releases-publish.sh:108-135` does.
  6. **Verify what is served:**
     * fetch `/downloads/wipemark`;
     * rebuild each statement from the served fields;
     * verify its `sig` with the **committed public key**, using
       `openssl pkeyutl -verify -rawin`, an implementation apart from
       the one that signed;
     * HEAD each URL for its size, then download each artifact and check
       its sha256 (MnemoVi's lesson, §1.6).
  7. **Recommended:** run `scripts/verify/release/appimage-gate.sh` on
     the AppImage *as downloaded from giglabo*, as MnemoVi's
     `published-artifact-gate.sh` does.
* **Scripts** (each with a header, as the repository requires):
  * `scripts/release/update/statement.sh` — the statement for one file;
  * `scripts/release/update/publish.sh` — open, presign, PUT, finalize,
    adapted from MnemoVi's;
  * `scripts/release/update/verify-served.sh` — step 6, runnable by hand
    against production at any time.
* **Secrets**, names only, all in the `releases` environment:

  | secret | holds |
  |---|---|
  | `RELEASES_PUBLISH_TOKEN` | the Worker's `PUBLISH_TOKEN` (the value MnemoVi's Woodpecker holds as `publish_token`) |
  | `WIPEMARK_UPDATE_SIGNING_KEY` | the Ed25519 private key, PKCS#8 PEM, encrypted |
  | `WIPEMARK_UPDATE_SIGNING_KEY_PASSWORD` | its password |

  No Cloudflare token or R2 key ever reaches GitHub: the presign flow
  keeps them in the Worker (`releases-publish.sh:7-8`).

**The giglabo side** is the owner's repository, unchanged here. It needs
three changes, listed in §8.5: register `wipemark`, pass `sig` through,
and, optionally, a publish token per slug.

---

## 3. Trust (D782)

### 3.1 What each check proves

| check | proves | who could defeat it |
|---|---|---|
| TLS to `releases.giglabo.com` (rustls with the platform verifier, as the model downloader uses) | the bytes came from whoever holds the domain's certificate | Cloudflare, the account holder, a misissued certificate |
| the URL is the one the program builds; no redirect is followed | this computer talks to no other host because of the manifest | nobody, short of the program itself |
| **Ed25519 over the statement**, with a key compiled into the program | GigLabo's release process said: *this* file, of *this* size and sha256, is version V for platform P on channel C | only the holder of the signing key |
| the sha256 and the size of the download | these bytes are the ones the statement names | nobody, given the signature |
| version strictly newer and channel `stable` | no downgrade, no beta on stable | nobody |
| **macOS:** `codesign --verify --deep --strict` against the Developer ID requirement for `com.GigLabo.wipemark`, team `M8AM5QVKBL` | the new bundle is signed by the same Apple team, and its seal is whole | the holder of the Developer ID certificate's private key |
| **macOS:** `spctl -a -t exec -vv` says `accepted` and `source=Notarized Developer ID` | Apple notarized this bundle; the stapled ticket answers offline | Apple |
| the staged copy's own `--version` (and, on macOS, `CFBundleShortVersionString`) | it starts, finds its libraries, and is the version offered (P4, and MnemoVi's 2026-08-05 lesson) | nobody |

On macOS **two independent keys** must both be stolen before anything
installs: the update key and the Developer ID. On Linux the update key is
the only root of trust, because the Linux files carry no signature of
their own yet. That is stage two in packaging.md, "What is not here".

The requirement string is compiled in:

```
anchor apple generic
  and identifier "com.GigLabo.wipemark"
  and certificate 1[field.1.2.840.113635.100.6.2.6]
  and certificate leaf[field.1.2.840.113635.100.6.1.13]
  and certificate leaf[subject.OU] = "M8AM5QVKBL"
```

The checks run the same `/usr/bin/codesign` and `/usr/sbin/spctl` that
the release lane runs (packaging.md, "Checking a .dmg"), by absolute path.
Security.framework (`SecStaticCodeCheckValidity`) is the later
alternative, if spawning a tool proves awkward.

### 3.2 The keys

* **The primitive.** Ed25519, the same one as Tauri's updater, Sparkle's
  EdDSA and the Worker's `verifyEd25519`.
* **Verification in the program** uses **`ring`**. `ring` 0.17 is already
  in `Cargo.lock` through `rustls`, `rustls-webpki` and `quinn-proto`, so
  it is not a new package. The call is
  `ring::signature::UnparsedPublicKey::new(&ED25519, key).verify(statement, sig)`.
* **Signing in CI** uses **OpenSSL 3** on `ubuntu-24.04`:
  `openssl pkeyutl -sign -rawin -inkey key.pem -passin env:…`. No tool is
  installed or pinned for it.
* **Key ids.** The id is the first 8 bytes of the sha256 of the raw
  32-byte public key, written as 16 hex digits. The program carries a
  small table of `(id, key)`. The public keys are also committed as PEM
  under `crates/wipemark-update/keys/`, and a test holds the table and the
  files equal.
* **Two keys from the first release:**
  * **the signing key** lives in the `releases` environment as
    `WIPEMARK_UPDATE_SIGNING_KEY`, an encrypted PEM, with its password
    beside it;
  * **the backup key** never touches GitHub. The owner keeps it offline,
    in a password manager. It exists so that a leaked signing key can be
    dropped: a release signed with the backup key carries a table without
    the leaked key.
* **Generation** happens once, on the owner's Mac:
  `openssl genpkey -algorithm ed25519 -aes-256-cbc -out wipemark-update-<id>.key`
  for the private key, and `openssl pkey -in … -pubout` for the public
  one.
* **Rotation.**
  1. Release N ships the next key's public half beside the current one.
  2. Release N+1 signs with the next key.
  3. Release N+2 drops the old key from the table.

  An installation that skipped a release still verifies, as long as it is
  not more than one rotation behind. If it is, the check says the
  signature did not verify, and the person downloads by hand.
* **Not reused:** lazy-shot's `TAURI_SIGNING_PRIVATE_KEY`. Each product
  has its own key, so a leak in one product signs nothing for another.

The alternative is **minisign**, the format of Tauri's signer:

* through the `minisign-verify` crate (no dependencies, MIT), which would
  be a new package;
* signed by the `minisign` CLI, pinned by sha256 as the builder's tools
  are;
* it brings its own key ids, trusted comments and encrypted secret keys.

Recommended: raw Ed25519 over our own statement, which needs no new crate
and no new tool. minisign is a fair second choice (Q5).

### 3.3 What none of this proves — the third shelf

* **That the version offered is the newest one.** Whoever controls
  `releases.giglabo.com` can withhold a release, and the program will say
  it is up to date (a freeze). The Settings row says when the last check
  succeeded, never "this is the newest version there is".
* **That a release is good.** A signed release can still be broken. The
  previous copy is kept until the new one has started once (§6.6), and
  the service keeps the last five versions for a manual step back.
* **That GitHub's runner is honest.** The key is used on a hosted runner.
  On macOS the Developer ID is a second, independent bar. On Linux it is
  not.

---

## 4. Privacy and consent (D784)

### 4.1 What changes in the product's promise

The application reaches the network today in exactly three ways, and each
is something the person started or set up:

* a model download;
* the endpoint the person configured;
* `Check` on the Engine and Rewriting pages.

The printed promise says so:

* README.md:251-253: "**No telemetry.** The only connections Wipemark
  makes beyond this machine are the model downloads you start and the
  endpoint you configure."
* the site: "Wipemark goes online only to download a model you pick… and
  to reach an endpoint you set up. There is no telemetry and no account."
  (`site/index.html:301-303`), and "It has no telemetry and no automatic
  updates." (`:576-577`).
* the release notes: "**Nothing leaves your machine** unless you choose
  an endpoint…" (`docs/release/0.1.0.md:74-77`).

An automatic check is the first connection the person did not start. It
carries nothing about the person or a document. Even so, the server
learns something:

* **What the request carries:**
  * `GET /downloads/wipemark` over TLS, SNI `releases.giglabo.com`;
  * headers `Host`, `Accept: application/json` and
    **`User-Agent: wipemark`** — fixed, with no version and no operating
    system;
  * no query, no cookie, no body, no conditional headers.

  The server cannot tell one installation from another by the request.
  The platform and version are chosen on this computer.
* **What the server sees all the same:**
  * the IP address and the time;
  * at most once a day from this address, so a rhythm;
  * that a copy of Wipemark runs there.

  Cloudflare's logs and the R2 analytics hold that. The DNS resolver sees
  the lookup. **This is a usage signal**, and the wording must not call
  it nothing.
* **What is never sent:** documents, file names, the journal, settings,
  an identifier, the version, the model, crash data.
* **A download happens only after a click.** The automatic part is the
  GET of a 2 KB document and nothing more.

### 4.2 The default — options

| option | what happens | for | against |
|---|---|---|---|
| **A. Ask once** (recommended) | Row `updates.check`, default **off**. A new install sees a switch on the walk-through's last step, with the sentence in §4.3. An install that gains the feature in an update sees one quiet notice in the main window, once ("Check for new versions once a day? **Yes** / **No**"); either answer ends it. "Check now" always works. | Every install made under the 0.1.0 promise keeps it until the person agrees. Sparkle's documented default is the same: it asks on the second launch before checking on its own. The audience is people removing provenance marks. | Fewer installs will update. |
| B. On by default, said on screen | lazy-shot's and MnemoVi's way: the check runs from the first launch. | The most installs stay current. | It breaks the printed promise for everyone who installed 0.1.0 under it: a privacy tool calling home by default. |
| C. Manual only | Only "Check now". | Nothing is automatic. | Almost nobody updates, and then self-update is barely worth building. |

**Recommended: A** (Q1). A one-time notice is not a modal: it sits where
the main window says things, and the panel never shows it.

### 4.3 The sentences that change

Proposed, for the owner to word:

* **Settings › General, the row's description (and the walk-through's
  switch):** "Once a day, Wipemark asks releases.giglabo.com which version
  is newest. The request carries no identifier, no version number and
  nothing about your documents; the server sees this computer's address,
  as any website does. Nothing is downloaded until you choose to."
* **README, site and release notes:**
  * **No telemetry.** The only connections Wipemark makes beyond this
    machine are the model downloads you start, the endpoint you
    configure, and — if you turn it on — a check once a day for a new
    version. That check sends nothing about you or your documents.
  * The site's "no automatic updates" (`:474`, `:577`) and the release
    notes' "Updates" limitation (`0.1.0.md:289-290`) go.
* **The status bar's "nothing leaves this machine"** (catalogue
  `status-idle-here`, `status-local-*`) **stays as it is.** Its subject is
  the document under a rewrite, and that remains true.

### 4.4 How often

* **At launch:** the first automatic check runs no sooner than one minute
  after start. It waits for the first models scan to land (D745–D747),
  so it never competes with startup.
* **While running:** a tick every hour re-evaluates a 24 h gate measured
  from the last *successful* check. This is lazy-shot's rhythm
  (`InitProvider.tsx:121-131`). A failed check is tried again at the next
  tick. Offline, that costs nothing, because there is no network to
  reach.
* **Only the application checks on its own.** The CLI checks when asked
  (§7), and MCP never does.
* **A development build** (`cargo run`) never checks on its own (§6.0).

---

## 5. What the person sees (D785)

Every string comes from the catalogue: en-US, then ru and de (§8.6). The
dialog is an element in the view's own tree (`dialog.rs`); it is never a
window, and never opened from inside `WindowHandle::update`. The check and
the download run off the GPUI thread and report through a
`flume::Receiver` polled from `cx.spawn`.

* **Settings › General — "New versions":**
  * the switch, with §4.3's description;
  * **Check now**, which runs even with the switch off, and ignores a
    skipped version;
  * a status line:

    | when | the line says |
    |---|---|
    | up to date | "This is 0.2.0, the newest. Checked today at 14:02." |
    | a version waits | "0.2.1 is available. **Update…**" |
    | the last check failed | "Could not check at 14:02: no connection." (reason in plain words) |
    | a newer announcement failed to verify | "A newer version was announced but its signature did not verify, so it is not offered." |
    | development build | "A development build does not check for new versions." |

* **The status bar**, only while a verified version waits: a quiet
  "0.2.1 is available" on its right. A click opens the dialog. No modal
  ever pops up for an automatic check.
* **The tray:** "Update to 0.2.1…" appears only while a version waits,
  the way Unload model is enabled only while a model is loaded
  (`tray.rs:110-113`). It brings the main window forward and opens the
  dialog there.
* **The dialog:**
  * Title "Wipemark 0.2.1". Body: "You have 0.2.0. Updating downloads
    23 MB from releases.giglabo.com, checks its signature and its Apple
    notarization, and restarts Wipemark."
  * **What's new** opens the notes link in the browser (§2.2).
  * Buttons: **Update and restart**, **Update when I quit**, **Later**,
    **Skip this version**.
  * Progress shows in place: "Downloading — 12 of 23 MB", "Checking", then
    "Ready".
  * Where the program cannot replace itself (§6.0), the dialog says why
    in one sentence, and the action becomes **Download…**.
* **Greyed, with the reason said** (the rule "every greyed item says
  why", D731):
  * "Waits for the rewrite to finish."
  * "Waits for the clean to finish."
  * "Waits for the model download to finish."
  * "Compare holds edits that are not saved. Save them or close Compare
    first." — a quit does not flush them when autosave is off
    (`compare.rs:2795-2799`).
* **What a restart drops** is said before the click, exactly as Quit
  would drop it: a cleaned pasted text lives only in memory and goes with
  the process (D419). If any exist: "2 cleaned pasted texts are only in
  memory and will be gone after the restart." **Update when I quit**
  avoids the surprise, because the person chose to quit anyway.

---

## 6. Installing (D786, D787, D788)

### 6.0 What kind of installation this is

The application reads where it runs from. `wipemark-update` decides,
under `Installation::detect`, and a test covers each case:

| installation | how it is known | gets |
|---|---|---|
| **macOS bundle, Developer ID**, in a writable folder | executable at `<X>.app/Contents/MacOS/wipemark`; the running bundle satisfies the requirement in §3.1; `access(<parent of X.app>, W_OK)` | check, download, **install** |
| macOS bundle, translocated (`…/AppTranslocation/…`) or on a read-only volume (run from the `.dmg`) | the path | check and **notify**: "Move Wipemark to Applications first." |
| macOS bundle, ad hoc (built without the secrets) or not writable | the requirement fails, or `W_OK` does | check and **notify**, with **Download…** |
| **AppImage** in a writable folder | `$APPIMAGE` is set, and its folder is writable | check, download, **install** |
| AppImage in a read-only folder | `W_OK` fails | check and **notify** |
| **.deb** | executable at `/usr/lib/wipemark/wipemark` | check and **notify** (§6.4) |
| **development build** | none of the above (`target/…`) | **no automatic check**; Check now says what it would offer |

### 6.1 macOS (D786)

The download is the **`.dmg`**. It is the file a person downloads by
hand. It is already signed, notarized, stapled and gated by the release
lane, and the Worker already knows its token. The alternative is a zip of
the stapled app. That would be a second artifact, a new format token
(`zip` is not one of the Worker's), and a third notarization. `ditto
-x -k` would save only the mount.

The order of the steps:

1. **Download** to `<data dir>/updates/<version>/<file>.part`.
   * https only, no redirect, a fixed user agent, and the proxy settings
     the model downloader uses;
   * the sha256 is computed while the bytes stream;
   * it stops at the stated size plus one byte;
   * on a mismatch the `.part` is deleted.

   The verified file is renamed into place. `updates/` joins the data
   directory's list in the release notes.
2. **Mount** with `hdiutil attach -nobrowse -readonly -noautoopen
   -mountpoint <tmp>`. hdiutil checks the image's own checksum.
3. **Stage.** `ditto <mount>/Wipemark.app <parent>/.Wipemark.app.update`,
   in the same folder as the installed bundle, so the swap is one
   rename on one volume. Then `hdiutil detach`.
4. **Check the staged copy:**
   * `codesign --verify --deep --strict -R '<requirement of §3.1>'`;
   * `spctl -a -t exec -vv` must say `accepted` and `source=Notarized
     Developer ID`;
   * `CFBundleIdentifier` must be `com.GigLabo.wipemark`, and
     `CFBundleShortVersionString` must be the version offered;
   * `Contents/MacOS/wipemark --version` and `wipemark-cli --version` run
     from the staged copy, as P4 runs them on a fresh bundle.

   Any miss deletes the staged copy, and the installed one is untouched.
5. **Wait for the moment** — **Update and restart** now, or **Update when
   I quit**. Either way the gates of §5 must hold.
6. **Unload the model** through `engine_host`, never under a running
   decode (the same road as the tray's Unload model).
7. **Swap** with `renamex_np(staged, installed, RENAME_SWAP)`. That is
   one atomic call on APFS. The old bundle now sits at the staged path,
   and is renamed to `<parent>/.Wipemark.app.previous`.
   * A process that is running keeps its open files. The model is
     unloaded and nothing else loads from the bundle on the way out.
   * Every path the person uses stays the same:
     `/Applications/Wipemark.app`, a Dock item, and a
     `~/.local/bin/wipemark-cli` link into `Contents/MacOS/`.
8. **Relaunch** with `cx.set_restart_path(installed)` then
   `cx.restart()`.
   * GPUI's macOS restart (`gpui-pre-macos` 0.3.8, `platform.rs:614-659`)
     spawns a detached `bash` that waits for this pid to end and then runs
     `open <path>`. It then quits, so the `on_app_quit` handlers run: the
     engine shut down (D743), Compare's flush, the beacon and the socket
     taken away (D717).
   * The new process starts only after this one has gone, so the
     single-instance handover (D646) cannot hand it back to the old one.
   * For **Update when I quit**, the swap happens in the quit path after
     the engine is shut down, and nothing relaunches.
9. **The new version, once its first window is up**, deletes
   `.Wipemark.app.previous` and `<data dir>/updates/`.

Cases to confirm in the live check, each refused with a sentence:

* **App Management** (macOS 13 and later). An app signed by the same team
  may replace its own bundle without a prompt. An ad hoc copy may be
  refused with `EPERM`; the swap fails, and the program notifies instead.
* **A bundle installed by another user.** The rename needs write access
  to the parent folder only, which §6.0 has checked.

### 6.2 Sparkle 2, or our own

| | Sparkle 2 | ours |
|---|---|---|
| licence | MIT | ours |
| platforms | macOS only, so Linux still needs ours: two mechanisms and two feeds (an appcast XML for the Mac, the JSON for Linux) | one mechanism, one JSON, both platforms |
| metadata | its own RSS appcast with `sparkle:edSignature` (EdDSA) and `SUPublicEDKey` in Info.plist; it does not read the giglabo manifest | the giglabo manifest, as the owner asked |
| interface and words | its own windows (AppKit, not GPUI) and its own localizations, which breaks "every string a person reads comes from the catalogue". A custom user driver can replace its interface, but then we write the interface anyway | GPUI, from the catalogue |
| bridging | Objective-C. We would drive `SPUStandardUpdaterController` through `objc2`, and link a framework | Rust and the system's own tools |
| signing | a framework with helper executables (an installer, an updater app, XPC services) to sign inside-out in `sign-and-notarize.sh`, and to check with its no-entitlements rule. Size: a few MB, to be measured if chosen | nothing new to sign |
| what it does better | years of edge cases: installing into a folder the user cannot write to, through an authorization prompt; translocation; delta updates; App Management | refuses those cases and notifies (§6.0) |
| consent | asks on the second launch by default | D784 |

**Recommended: our own** (D786). Sparkle's strengths are in the cases our
design refuses and turns into a notice. If the live checks show many
people in those cases, Sparkle on macOS is a fallback we can revisit.

### 6.3 Linux, AppImage (D787)

1. **Download** into the **AppImage's own folder** as
   `.Wipemark-update-<version>.AppImage.part`, verified as in §6.1 step 1.
   The same folder means the same filesystem, so the rename is atomic.
2. `chmod 0755`, then run `<staged> --version`. The AppImage gate does the
   same.
3. **Keep the previous copy:**
   `link($APPIMAGE, <folder>/.Wipemark.AppImage.previous)`, a hard link to
   the same bytes.
4. **Swap:** `rename(staged, $APPIMAGE)` replaces the file at the
   **same path**.
   * The running copy is served by its FUSE mount from the old inode,
     which stays alive while it is open.
   * The `.desktop` entry that `integrate.rs` wrote, the person's
     `wipemark-cli` link, and a pinned dock item keep working.
   * **The file name keeps the old version in it**, for example
     `Wipemark-0.1.0-x86_64.AppImage` now holds 0.2.0. Writing the new
     name beside it would break every link and the desktop entry (Q9).
5. **Relaunch** with `cx.set_restart_path($APPIMAGE)` then `cx.restart()`.
   This is necessary. GPUI's Linux restart runs `current_exe()` by default
   (`vendor/gpui-pre-linux/src/linux/platform.rs:639-690, 972-976`), and
   inside an AppImage that path sits in the FUSE mount, which disappears
   when this process exits.

AppImageUpdate with zsync is **not** used:

* it needs update information embedded at build time and `.zsync` files
  published beside each AppImage;
* it is a C++ tool or library to carry;
* its trust root is a GPG signature on the AppImage, which we do not
  make yet.

The AppImage is 43.4 MiB (`reports/E10-appimage-2026-10-12.md:209`), so a
delta saves little. packaging.md already lists "update information
(zsync)" under "What is not here".

### 6.4 Linux, .deb (D787)

apt owns `/usr/lib/wipemark`. Replacing files under it behind apt's back
would break `apt remove` and the next upgrade. The application therefore
**notifies**, with **Download…**:

* it downloads and verifies the `.deb` the same way, into the Downloads
  folder (the retention page's default for results without a file);
* then it says "Verified. Install it with:
  `sudo apt install ./wipemark_0.2.1_amd64.deb`", with **Copy** and
  **Show in folder**.

Verifying first is better than a browser download, which is checked
against nothing. The proper road for a `.deb` is a signed apt repository
(`deb [signed-by=…] https://releases.giglabo.com/apt stable main`). That
is stage two, and the owner's call (Q7).

### 6.5 Never under work (D788)

Update and restart, Update when I quit, and Download… are all **greyed**
while any of these holds:

* a row of the queue is running (`Standing::running()`, `queue.rs:1256`);
* the engine has a job (`EngineHandle::busy() > 0`, `engine_host.rs:582`);
* a model download runs;
* a Compare window holds edits autosave would not keep.

Each greyed item says which of these it is (§5). The check itself and the
download may run during work: neither touches the engine or a document.
The swap and the relaunch go through the quit path, so the engine rules
hold as they are:

* never unloaded under a running decode;
* no process reaches `exit` with a local model loaded (D740–D744).

### 6.6 Failure and stepping back

* **Before the swap**, every failure leaves the installed copy untouched:
  download, verification, mount, staging, the checks, a refused rename.
  The status line says what failed in one plain sentence. The log gets
  the detail, and never the URL's query or a header (there are none
  anyway).
* **The swap** is one call. It either happened or it did not.
* **After the swap:**
  * If the relaunch does not come up, the person starts Wipemark by hand.
  * The previous copy stays (`.Wipemark.app.previous`,
    `.Wipemark.AppImage.previous`) until the new version has opened its
    first window. If it never does, the previous copy is still there, and
    the log names it.
  * An automatic rollback would need a launcher apart from the program,
    which is not proposed.
* **Stepping back on purpose:** the service keeps the last five versions,
  so a person can download the previous one by hand.

---

## 7. The CLI and MCP (D788)

* **The CLI updates with the application.**
  * **macOS:** `wipemark-cli` lives in `Contents/MacOS/` and is swapped
    with the bundle. A link into the bundle keeps working.
  * **AppImage:** the CLI *is* the AppImage (`--cli`, or a link named
    `wipemark-cli`), so it is the same file.
  * **.deb:** apt.

  A CLI process running during the swap keeps its own open files.
* **`wipemark-cli --version`** already exists. It prints the version and
  exits before the database and the network.
* **`wipemark-cli update --check [--json]`** is proposed. It reads the
  same manifest with the same verification, and says what is installed,
  what is newest **and verified** for this installation's platform and
  format, and where to get it. The exits follow CLAUDE.md:

  | exit | means |
  |---|---|
  | `0` | up to date |
  | `1` | a newer verified version exists — a finding |
  | `2` | refused: no connection, a signature that does not verify, no build for this platform |

  The human output comes from the catalogue; the JSON does not. It runs
  only when typed. The CLI never checks on its own, because scripts must
  stay deterministic and offline.
* **No `wipemark-cli update` that installs.**
  * On macOS the CLI would be replacing the bundle it runs from, and the
    application may be running.
  * On a `.deb`, apt owns it.
  * On an AppImage it would be possible, but it would race the
    application's own update.

  A plain `wipemark-cli update` refuses with exit 2 and the sentence
  "Update from the application, or download: <url>".
* **MCP gets nothing.** No tool installs or checks, so an agent cannot
  start an install. The server says nothing about updates, and nothing
  it says comes from the catalogue. An install closes MCP's port with the
  process, as Quit does, and the beacon goes with it (D717).

---

## 8. What changes in the repository (D789)

### 8.1 A new crate: `wipemark-update`

It is a **leaf**, like `wipemark-store` and `wipemark-secret`:

* it is handed the running version, the installation, the clock and a
  base URL;
* it hands back an `Offer`, or a `Refusal` with a machine reason;
* it never reads the settings table (the application passes the
  preference) and never localizes (only applications do).

| module | owns |
|---|---|
| `manifest` | the served JSON's types (serde), the size cap, the choice of §2.3 |
| `statement` | the text of §2.2; Ed25519 verification with `ring` |
| `keys` | the `(id, key)` table; `keys/*.pem` committed beside it |
| `fetch` | the `ureq` agent: https only, no redirects, timeouts, the fixed user agent, the models downloader's proxy settings; the streamed sha256 with the size cap |
| `install` | `Installation::detect`; `macos` (hdiutil, ditto, codesign, spctl, `renamex_np`); `appimage` (`link`, `rename`). It is the only module that writes outside the data directory, as `wipemark-intake::inplace` is the only one that writes in place |

Its dependencies are `ureq`, `sha2`, `hex` and `serde_json` (workspace),
plus `ring` 0.17, `semver` 1 and `libc`. **All of them are already in
`Cargo.lock`**, so the lock gains our crate and no new package.

`scripts/check-dep-direction.sh` adds `"wipemark-update"` to `LIBS`, and
this entry to `ALLOWED`:

```python
# Whether a newer version exists, proved: the served manifest, the
# release statement and its Ed25519 signature, the download held to its
# sha256, and the swap. A leaf on purpose: it is handed the running
# version and the installation rather than finding them in a settings
# row, and it hands back a verdict rather than a sentence — the
# application words it, and the CLI words it differently.
"wipemark-update": set(),
```

Both applications depend on it. It is the one crate whose purpose is to
reach the network on the application's own initiative. Keeping it a leaf
keeps that initiative in one place, which is the reason `duty.rs` is the
one place that decides who rewrites.

### 8.2 The application

* **`apps/wipemark-app/src/update.rs`:**
  * the schedule of §4.4;
  * the state;
  * the status-bar item, the dialog and the tray command;
  * the gates of §6.5, read from `Queue`, `EngineHandle`, the model
    downloads and Compare;
  * the swap and `cx.restart()`;
  * the cleanup of `.previous` and `updates/` after the first window.
* **`config.rs`:** `UPDATES_CHECK_KEY = "updates.check"` (bool, default
  `false`), in `PERSISTED`. The **state** —
  `updates.last_check`, `updates.skipped` and `updates.asked` — stays out
  of `PERSISTED` and out of the rows, as window geometry does
  (`window_state.rs:6-16`: "a preference is asked for, state is
  observed").
* **`settings.rs`:** the General row (§5).
  `every_persisted_preference_has_a_row` then holds the key and the row
  together.
* **`setup.rs`:** the switch on the last step (D784). Finish writes it,
  and Skip leaves the default.
* **`tray.rs`:** `TrayCommand::Update`, shown only while an offer stands.
* **Debug builds only** (`cfg(debug_assertions)`), for the live check:
  * `WIPEMARK_UPDATE_BASE` — an `http://127.0.0.1:<port>` base URL;
    loopback only;
  * `WIPEMARK_UPDATE_TEST_KEY` — one extra public key.

  A release build has neither, so there is nothing to misuse.
  development.md documents both.

### 8.3 The CLI

`apps/wipemark-cli/src/update.rs`: `update --check [--json]` (§7), its
exits in `docs/architecture/cli.md`, and its words in the catalogue.

### 8.4 The release lane

* **`release.yml`:** the `linux` and `macos` jobs each add
  `actions/attest-build-provenance` over their artifacts. That needs
  `permissions: id-token: write, attestations: write` on those two jobs
  only. Nothing else changes, and the draft stays a draft.
* **`publish-update.yml`** (new): §2.5.
* **`scripts/release/update/`:** `statement.sh`, `publish.sh` and
  `verify-served.sh`, each with its header.
* **`crates/wipemark-update/keys/`:** the two public keys, as PEM.

### 8.5 The giglabo side (the owner's `heretic-releases-api`)

1. **`src/apps.ts`:** register
   `{ slug: 'wipemark', source: 'direct', keep: 5, expected_platforms: ['macos-aarch64', 'linux-x64'] }`.
2. **Finalize and PUT accept an optional `sig`:**
   * it must match `^wm1:[0-9a-f]{16}:[A-Za-z0-9+/]{86}==$`, or the call
     is answered 400;
   * it is stored in `meta.json` (`PlatformMeta.sig` exists,
     `types.ts:95-96`);
   * `assemble.ts` copies it to the asset, and `ManifestAsset` gains
     `sig?`.

   The change is additive, and MnemoVi's manifest stays as it is.
3. **Optional:** a publish token per slug. Today one `PUBLISH_TOKEN`
   writes every `direct` app (`types.ts:199-203`). For Wipemark, a leaked
   token can withhold or break releases, but it cannot make an install
   happen, because of the signature.

The clone read here is older than the deployed Worker (Provenance). The
change must be written against the deployed source (Q6).

### 8.6 Catalogue strings

These are new keys in `wipemark.ftl` (en-US, then ru and de), with the
wording proposed in §4.3 and §5:

* the row: `settings-updates-title`, `-description`, `-check-now`,
  `-current`, `-newest`, `-available`, `-failed`, `-refused`,
  `-development`;
* the walk-through: `setup-done-updates`;
* the one-time notice: `update-ask`, `update-ask-yes`, `update-ask-no`;
* the status bar: `status-update-available`;
* the tray: `tray-update`;
* the dialog:
  * the frame: `update-dialog-title`, `-body`;
  * the actions: `update-now`, `update-on-quit`, `update-later`,
    `update-skip`, `update-whats-new`;
  * progress: `update-downloading`, `update-checking`, `update-ready`;
* the reasons a button is greyed: `update-waits-rewrite`,
  `update-waits-clean`, `update-waits-download`, `update-waits-compare`,
  `update-drops-pastes`;
* the notify-only cases: `update-notify-deb`, `update-notify-readonly`,
  `update-notify-translocated`, `update-notify-unsigned`,
  `update-deb-verified`;
* the plain reasons a check or an install failed: `update-failed-*`;
* the CLI: `cli-update-*`.

### 8.7 Tests — red first

Each test must be seen red once with its protection deleted, then green.
The table says how to make each one red.

| test | guards | red by |
|---|---|---|
| `a_statement_signed_by_another_key_is_refused` | the signature is the authority (a forged signature) | verify against any key |
| `a_changed_field_breaks_the_signature` (version, size, sha256, platform, format, file, channel — each changed alone) | the statement covers what the application acts on | drop a line from the statement builder |
| `an_unknown_key_id_is_refused` | keys come only from the table | fall back to the first key |
| `a_version_not_newer_is_not_offered` (equal and older) | no downgrade, no re-install | compare with `>=` |
| `a_prerelease_is_not_offered_on_stable` | the channel | drop the prerelease check |
| `an_asset_for_another_platform_or_format_is_not_used` | no fallback across formats | take the first asset |
| `a_download_whose_sha256_differs_is_refused_and_removed` | the bytes are the statement's | skip the comparison |
| `a_download_longer_than_announced_is_cut_and_refused` | the size cap | read to the end |
| `an_asset_on_another_host_is_refused_before_any_request` (a loopback server counts connections: zero) | privacy: a manifest cannot steer the program to another host | trust `url` |
| `the_manifest_is_never_followed_through_a_redirect` (a 302 to another loopback port is refused) | the JSON comes only from its host | allow redirects |
| `a_plaintext_or_foreign_base_is_refused_in_a_release_build` | the base URL is a constant | read the variable in every build |
| `a_manifest_over_the_cap_is_refused` | memory | no cap |
| `the_check_sends_nothing_but_the_get` (the server records method, path, every header, the query and the body) | §4.1's promise, as a test | add the version to the user agent |
| `the_compiled_in_keys_are_the_committed_ones` | the key table and `keys/*.pem` agree | edit one byte of either |
| `the_installation_is_read_from_where_the_program_is` (each row of §6.0, from paths in a temporary directory) | install versus notify | treat every bundle as installable |
| `no_check_runs_while_the_row_is_off` / `an_automatic_check_waits_a_day` / `a_development_build_never_checks_on_its_own` (app) | D784 | remove the gate |
| `update_is_greyed_while_a_rewrite_runs` (and while a clean, a model download, an unsaved Compare edit) | D788 | drop the gate |
| `the_swap_is_one_rename` (macOS: `renamex_np` on temporary directories; Linux: `link` + `rename` on temporary files) | atomicity, and the previous copy kept | copy instead of rename |
| `an_ad_hoc_bundle_fails_the_requirement` (macOS gate job: a tiny bundle signed `codesign -s -` in a temporary directory) | the Developer ID bar | skip `-R` |
| `update_check_exits_0_1_2` (CLI, against a loopback server) | the exits | swap two arms |
| `a_live_manifest_verifies` (`#[ignore]`, run by hand: the real `/downloads/wipemark`, newest statement per platform) | the program and the service agree, as MnemoVi's live test does | — |

These run in `cargo test --workspace`, on both the Linux job and the
macOS job of `gate.yml`. A key pair for the tests is generated by `ring`
inside the test, and no private key is ever committed.

### 8.8 The live check, once, at the end

1. **Loopback** (debug build). A static server on `127.0.0.1` serves a
   manifest and a real `.dmg` or AppImage, signed with a throwaway key
   handed in through `WIPEMARK_UPDATE_TEST_KEY`. Check, Update and
   restart, and the new copy runs. Then try three bad cases:
   * one byte of the artifact flipped — refused, with the sentence;
   * the statement's version lowered — not offered;
   * an asset URL on `127.0.0.2` — refused, and the server logs no
     connection.
2. **Production.** After the first release that carries the updater, cut
   the next version.
   * While it is only a draft, nothing is offered.
   * After Publish, it is offered within one check.
   * Install on the owner's Mac (`/Applications`) and on Linux (an
     AppImage in `~/Applications`). The `.deb` says Download and verifies
     the file.
   * `scripts/release/update/verify-served.sh` is green.
3. Kill the application when done (CLAUDE.md: port 5056).

### 8.9 Documents that change when it is built

* **`packaging.md`:** "What is not here" drops automatic updates; a
  section on publishing the update is added.
* **The release notes:** the next version's notes drop the "Updates"
  limitation, add `updates/` to the data folder's list, and add §4.3's
  sentence.
* **README and the site:** §4.3.
* **`architecture/map.md`:** the new crate and `update.rs`.
* **`architecture/rules.md`**, proposed rules:
  * "An update is offered only once proved — the statement's signature,
    then the bytes' sha256, then on macOS the Developer ID requirement
    and Gatekeeper; nothing is said or downloaded before the signature."
  * "The application reaches the network on its own only for the update
    check, only with `updates.check` on, at most once a day, and the
    request carries nothing."
  * "Nothing is installed under work."
* **`architecture/cli.md`:** `update --check`.
* **`development.md`:** the two debug variables.
* **CLAUDE.md's index:** the coordinator's, after the owner's yes.

---

## 9. Decisions D780–D789 — all *proposed*

| | proposed | why |
|---|---|---|
| **D780** | **Where the metadata lives.** The giglabo release service (`heretic-releases-api`), slug `wipemark`, `source: 'direct'`, `keep: 5`, `expected_platforms: ['macos-aarch64', 'linux-x64']`. The program reads `https://releases.giglabo.com/downloads/wipemark`. Artifacts are `https://releases.giglabo.com/wipemark/<version>/<platform>/<file>`. Formats are `dmg`, `AppImage` and `deb`, all existing tokens. The URL is a constant; only a debug build may point at loopback. | The owner's ask ("through giglabo, the lazy-shot way"). It is the newest route, MnemoVi's: a sha256 per asset, CI holds only a publish token, and per-platform independence (D778). The mirror route's regex matches none of our macOS or AppImage names and carries no sha256 (§1.4). |
| **D781** | **What is signed.** One release statement per artifact (§2.2): eight lines binding the channel, version, platform, format, file, size and sha256. The signature travels as the asset's `sig` (`wm1:<key id>:<base64>`). The program rebuilds the statement from the manifest's fields and verifies it before it says anything or downloads anything. It builds the URL itself and refuses a different one. It chooses the newest verified version strictly above its own on `stable`, for its own platform and format, with no fallback. | The derived manifest cannot be signed whole. The statement covers every field the program acts on. Building the URL in the program keeps a forged manifest from steering this computer to another host. These are MnemoVi's lessons on formats and on `latest_by_platform`. |
| **D782** | **Keys.** Ed25519, verified with `ring` (already in `Cargo.lock`) and signed with OpenSSL 3 in CI. Two public keys are compiled in from the first release: the signing key, in the GitHub Environment `releases` (tags `wipemark-v*` only) as `WIPEMARK_UPDATE_SIGNING_KEY` with `_PASSWORD`, and an offline backup key kept by the owner. A key id is 8 bytes of the sha256 of the public key. Rotation takes one release ahead. Nothing is shared with lazy-shot's Tauri key. | A root of trust independent of TLS and of the Cloudflare account. No new crate and no new tool. The backup key is the only way to drop a leaked key without trusting the leaked one. |
| **D783** | **Who writes it, and when.** `.github/workflows/publish-update.yml` on `release: published` (the person's Publish) and by hand. The steps: download the release's own files, check their sums and the build attestations, open the version, sign each statement, presign, PUT, finalize with `sig`, then read the served manifest back and verify every signature with the committed public key, through OpenSSL, apart from the signer. The build jobs attest their artifacts. The secrets are `RELEASES_PUBLISH_TOKEN`, `WIPEMARK_UPDATE_SIGNING_KEY` and `_PASSWORD`. No Cloudflare credential reaches GitHub. | The draft stays a draft (D778), so a version is offered only once a person has published it. The signing key never sits beside `cargo build`. Check what is served, not what was built (MnemoVi, 2026-08-05). |
| **D784** | **Consent.** Row `updates.check` in Settings › General, default **off**, asked once: a switch on the walk-through's last step, or a one-time notice for an install that gains the feature. Check now always works. The automatic check runs only in the application, at most once a day, at least a minute after start and after the first models scan. The request is a bare GET with `User-Agent: wipemark`. README, the site and the release notes are reworded (§4.3); the status bar's sentence stays. | It is the first connection the person does not start. The printed promise of 0.1.0 holds until the person agrees. Sparkle's default is to ask. |
| **D785** | **What the person sees.** A status-bar line and a tray item while a verified version waits; no modal for an automatic check. A dialog with Update and restart, Update when I quit, Later, Skip, and What's new. Greyed actions say why. A restart names what it would drop (cleaned pastes, D419). Every word comes from the catalogue. | The Settings window's own rule: a preference is something you go and change, not something you trip over. D731. |
| **D786** | **macOS install.** The `.dmg`, verified; mounted; `Wipemark.app` staged beside the installed bundle; codesign against the compiled-in Developer ID requirement (team `M8AM5QVKBL`, `com.GigLabo.wipemark`); Gatekeeper `Notarized Developer ID`; the version and both `--version`s from the staged copy; `renamex_np(RENAME_SWAP)`; `cx.set_restart_path` and `cx.restart()`. The previous copy is kept until the new first window. Translocated, read-only, ad hoc or unwritable copies get notify-only. Sparkle is not used. | Two independent keys stand between a forged release and an install. One mechanism, one feed and one catalogue serve both platforms. GPUI already carries the relaunch. |
| **D787** | **Linux.** The AppImage is replaced at `$APPIMAGE` by `rename` from its own folder, with the previous copy hard-linked until the new first window; the restart path is `$APPIMAGE`. The `.deb` gets notify-only, with the `.deb` downloaded and verified and the apt command shown. No zsync or AppImageUpdate. Development builds never check on their own. | Links, the desktop entry and a dock pin keep working. apt owns `/usr/lib/wipemark`. The artifacts are small. GPUI's default restart path would point into a vanished FUSE mount. |
| **D788** | **Never under work; the CLI; MCP.** Install is greyed while a clean, a rewrite, a model download or an unsaved Compare edit is in progress. The swap goes through the quit path (D740–D744, D717). `wipemark-cli update --check [--json]` exits 0 (up to date), 1 (newer and verified) or 2 (refused); the CLI installs nothing and never checks on its own. MCP gets nothing. | The engine rules hold as written. A script stays deterministic and offline. An agent cannot start an install. |
| **D789** | **Where the code goes.** The new leaf crate `wipemark-update` (no workspace dependency, no new package in `Cargo.lock`), its entry in `scripts/check-dep-direction.sh`, `apps/wipemark-app/src/update.rs`, the CLI subcommand, the keys under `crates/wipemark-update/keys/`, the catalogue keys of §8.6, the tests of §8.7 each seen red, and the live check of §8.8. | The network on the program's own initiative lives in one place, as "who rewrites" does in `duty.rs`. |

---

## 10. Open questions for the owner

| | question | recommended |
|---|---|---|
| **Q1** | The default of the automatic check: **ask once** (off until answered), on by default and said on screen (lazy-shot), or manual only? | Ask once (§4.2). |
| **Q2** | When does a version become visible to installations: at the person's **Publish** of the draft, or at the tag after notarization, as lazy-shot and MnemoVi do (which would mean dropping the draft step)? | At Publish (D783). |
| **Q3** | The giglabo route: **direct** (MnemoVi: a sha256 and a `sig` per asset), or mirror (lazy-shot: a webhook on the public GitHub release)? | Direct. |
| **Q4** | Build the install now, or ship the check with a verified **Download…** first and the install in the version after? | Both together, but land the check first. It is what makes 0.2.0 users reachable at all. |
| **Q5** | Keys: may the signing key live in a GitHub Environment limited to `wipemark-v*` tags, and will you keep the offline backup key? Raw Ed25519 with OpenSSL, or minisign? | Yes; raw Ed25519 (§3.2). |
| **Q6** | The Worker change (§8.5) is yours, in `heretic-releases-api`. Is the deployed source newer than `~/self/heretic-releases-api` at `35f9673`? It serves `format_preference` and `format_tokens`, which that clone lacks. Do you want a publish token per slug? | Write the change against the deployed source; per-slug tokens later. |
| **Q7** | `.deb`: notify with a verified download (proposed), or a signed apt repository on `releases.giglabo.com` later? | Notify now; the apt repository is stage two. |
| **Q8** | "What's new": the public GitHub release page, or a page on giglabo.com? (0.1.0's download location is still open in `v1-release.md` §2a.) | GitHub's page, built from the version. |
| **Q9** | AppImage: replace in place, so the file name keeps the old version, or write the new name beside and move the desktop entry? | In place. |
| **Q10** | How many versions the service keeps (`keep`): 5? | 5. |
| **Q11** | A beta channel: not now? It needs the Worker to order prereleases and filter `latest_by_platform` by channel (UC-Q3, UC-Q4). | Not now. |
| **Q12** | The reworded promise (§4.3): is the wording right? | — |
