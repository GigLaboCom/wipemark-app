# The product website

A static landing page for Wipemark: plain HTML and CSS, one small script, no
build step, no web fonts, no CDN, no trackers, no cookies. Any static host
can serve it as it is.

| file | what it is |
|---|---|
| `index.html` | the page: hero, intro video, what it removes, how it works and privacy, where you use it, screenshots, download, FAQ |
| `404.html` | the not-found page; its links are absolute, because a host serves it at whatever path was missing |
| `style.css` | the one stylesheet, light and dark by `prefers-color-scheme`, the application's palette |
| `site.js` | shows an empty labelled frame for a screenshot or a video that is not there yet; the page works without it |
| `icons.svg` | the application's own Font Awesome Free glyphs as one sprite |
| `favicon.ico`, `icon-192.png`, `apple-touch-icon.png`, `og-image.png`, `icons.svg` | not in `site/`: made from the application's icon and glyphs by `media/site-images/images.sh` into `media/out/site/`, copied to the site's root by the build |
| `robots.txt`, `sitemap.xml` | carry `__SITE_ORIGIN__`, written in by the build |

The screenshots and the video are not in `site/` and not in git: they are
generated into `media/out/screenshots/` and `media/out/video/` (gitignored;
`media/README.md` says by what, and where they will be kept is still open),
and the page names them as `assets/screenshots/…` and `assets/video/…`.
`scripts/site/build.sh` puts the pages and those files together in
`site/dist/` (gitignored), which is what a host serves. So, until the assets
have a home of their own, build and deploy from a machine that has
`media/out/`; the Pages workflow on GitHub has no `media/out/` and builds the
pages with empty frames.

## Build and preview

```sh
scripts/site/build.sh                      # site/dist/, missing pictures allowed
scripts/site/build.sh --strict             # refuses while a picture or the video is missing
cd site/dist && python3 -m http.server 8000
```

Then open <http://localhost:8000>. For the 404 page to find its stylesheet
in a preview, build with the preview's origin:
`SITE_ORIGIN=http://localhost:8000 scripts/site/build.sh`.

Two settings, both read by the build script and set nowhere else:

| | default | |
|---|---|---|
| `SITE_ORIGIN` | `https://wipemark.example` | where the site is served — scheme, host, and a path for a GitHub Pages project page; no trailing slash. Canonical and Open Graph URLs, `robots.txt`, `sitemap.xml` and the 404 page use it. The default is a placeholder until a domain is chosen. |
| `RELEASES_URL` | `https://github.com/GigLaboCom/wipemark-app/releases/latest` | where every download button points |

The build checks that no placeholder is left, that every local link and
source in the pages names a file, and says which files are over 25 MiB.

## Deploy

Pick one host. Both serve `site/dist/` unchanged.

### GitHub Pages

`.github/workflows/pages.yml` builds and deploys. It runs only by hand.

1. Once: repository **Settings › Pages › Build and deployment › Source:
   GitHub Actions**.
2. **Actions › pages › Run workflow**, with `site_origin` set to where the site
   will be served. Without a custom domain that is
   `https://giglabocom.github.io/wipemark-app`.
3. For a custom domain: add it under **Settings › Pages › Custom domain** and
   point a `CNAME` record at `giglabocom.github.io`. With the Actions
   deployment a `CNAME` file in the site is not used.

`strict` is on by default, so the workflow refuses to publish while a
screenshot or the video is missing.

### Cloudflare Pages

From a machine logged in to Cloudflare (`npx wrangler login`, or a
`CLOUDFLARE_API_TOKEN` in your own shell; no secret goes into this
repository):

```sh
SITE_ORIGIN=https://wipemark.giglabo.com scripts/site/build.sh --strict
npx wrangler pages project create wipemark --production-branch main   # once
npx wrangler pages deploy site/dist --project-name wipemark
```

Then add the domain under **Workers & Pages › wipemark › Custom domains**.
`giglabo.com` is already on Cloudflare DNS, so a subdomain needs no DNS work
outside Cloudflare.

Cloudflare Pages refuses a file over 25 MiB. If the intro video is larger,
put it in an R2 bucket behind a `giglabo.com` subdomain, as GigLabo's other
product serves its media, point the `<source>` and the poster in `index.html` at it,
and add that origin to `media-src` and `img-src` in the page's
Content-Security-Policy. GitHub Pages takes files up to 100 MB.

## Images

```sh
media/site-images/images.sh
```

redraws `icons.svg`, the favicon, the touch icon and `og-image.png` from
`apps/wipemark-app/assets/icon/icon.png` and `assets/icons/`. The Open Graph
picture is `media/site-images/og-image.html`, drawn by a headless Chrome; edit
that file, not the PNG. Every output is written without metadata. It needs
python3, ImageMagick 7 and Chrome or Chromium. The outputs go to
`media/out/site/` and are not committed (`media/README.md`).

## Editing the words

The page follows the repository's rules for anything a person reads:

- Every fact must be true of the version it names. When unsure, leave it out.
- Never "undetectable". Never a promise to beat or fool an AI detector, and
  nothing that suggests passing off generated work as a person's.
- Wipemark removes marks that software put into **your own** files without
  asking. Say that, calmly, without superlatives.
- No epic or decision numbers.
- What it cannot do is said plainly: a rewrite is best-effort, the third
  shelf (*not established*) is never empty, marks inside a picture's pixels
  are neither looked for nor removed.

Where the facts come from, to check them when the product changes:

| on the page | source |
|---|---|
| invisible characters, homoglyphs, Unicode 18.0 | `docs/architecture/layer-a.md`, `crates/wipemark-core/src/lib.rs` (`Options`) |
| picture metadata, generators, what stays | `crates/wipemark-image/src/lib.rs` (`MetadataKind`, `Scope`), `src/signatures.rs` (`Generator`) |
| visible marks | `manifests/marks.v1.json` (profiles `gemini-sparkle-v1`, `-v2`, `grok-x-dark`, `grok-x-light`) |
| re-encoding after a mark is removed | `docs/architecture/visible-marks.md`, README |
| the three shelves | `crates/wipemark-core/src/report.rs` (`not_established`) |
| remote endpoint refused by default, keys | `apps/wipemark-app/src/engine.rs` (`EngineSettings::default`), the catalogue's Engine strings |
| what goes online | `crates/wipemark-models` (Hugging Face downloads) and `crates/wipemark-engine` (endpoints) are the only crates with an HTTP client |
| MCP off by default, port, tools | `apps/wipemark-app/src/config.rs` (`mcp.enabled`), `src/mcp/mod.rs`, `src/mcp/protocol.rs` |
| file types | `apps/wipemark-app/src/clean.rs` (`cleanable`), `crates/wipemark-intake/src/format.rs` |
| where results go, Retention | `docs/architecture/retention.md`, `apps/wipemark-app/src/retention.rs` (`Destination`) |
| models, sizes, memory, licences | `manifests/models.v1.json` (`size_bytes`, `mem.min_ram_mb`, `license`) |
| platforms, install, data folder, uninstall | `docs/release/0.1.0.md`, `scripts/release/macos/bundle.sh` (`LSMinimumSystemVersion`) |
| the panel's shortcut | `apps/wipemark-app/src/hotkey.rs` (`PANEL_DEFAULT`) |
| licence | `LICENSE.md`, `NOTICE` |

## The owner's to-do

1. **The domain.** A suggestion, not a decision: `wipemark.giglabo.com`, beside
   the other GigLabo subdomains on Cloudflare. Until one is picked,
   `SITE_ORIGIN` stays the placeholder and the site should not be published.
2. **The host.** GitHub Pages (the workflow is ready) or Cloudflare Pages
   (the `wrangler` lines above).
3. **The download links.** Every download button points at
   `https://github.com/GigLaboCom/wipemark-app/releases/latest` until 0.1.0 is
   released. Once it is, set `RELEASES_URL` (or the buttons themselves) to the
   final place: the release on GitHub, or the release service, if 0.1.0 is
   published there.
4. **macOS signing.** The page says, as the release notes and the README do,
   that the `.dmg` is not notarized, and shows the first-open steps (the
   `<details>` marked `OWNER` in `index.html`). If 0.1.0 ships signed and
   notarized, delete that block.
5. **Screenshots and the video.** Build with `--strict` once
   `media/out/screenshots/`, `media/out/video/` and `media/out/site/` are
   there, and decide where the assets live so CI can fetch them. If the video
   has speech, add a captions track (`<track kind="captions" …>`).
6. **Third-party notices.** The footer links to `NOTICE` on `main`, which
   exists only once PR #1 is merged. `NOTICE` mentions an epic number in its
   third paragraph, which the site's rule keeps off the site; reword that
   line before the link goes live, or link to the release's `THIRD-PARTY.md`
   instead.
