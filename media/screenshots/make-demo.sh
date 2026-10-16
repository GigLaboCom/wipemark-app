#!/usr/bin/env bash
# make-demo.sh — the neutral demo folder the README's screenshots are taken in.
#
# What it is for: the screenshots and clips in media/out/ must show a real
# run of the application without showing anything of the person taking them —
# no home path, no e-mail address, no real file or row. So the run gets a
# folder of its own under /Users/Shared, a scratch data directory inside it,
# and documents written for the purpose (media/out/screenshots/README.md).
#
# What it does:
#   1. writes five short documents into "$DEMO": a blog draft, meeting notes
#      and an e-mail draft carrying U+200B, U+2060, U+00AD, U+00A0 and a
#      Cyrillic letter inside a Latin word; release notes that carry nothing;
#      a seven-paragraph team update, long enough for Compare to scroll;
#   2. copies picture fixtures under friendly names: a Gemini sticker
#      (visible mark, restored), a Grok picture from X (visible mark), a
#      JPEG with a C2PA manifest and one with an XMP provenance reference;
#   3. links "$DEMO/models" to the folder that already holds the GGUF weights
#      (a symlink, so nothing is copied and nothing is written there);
#   4. seeds "$DEMO/.data/wipemark.db" with the settings rows the run needs:
#      setup done, English, the theme asked for, no system-wide shortcut (the
#      person's own instance keeps it), the local engine on duty with
#      Gemma 4 12B from that folder, MCP on.
#   With --theme-only light|dark it rewrites the theme row and nothing else
#   (the running application reads it at its next start; Settings › General
#   › Appearance changes it live).
#
# How to run:   media/screenshots/make-demo.sh --models <dir> [--theme light|dark]
#               DEMO="/Users/Shared/Wipemark Demo" by default; --models is the
#               folder that already holds the catalogue's Gemma 4 12B GGUF
#               (found there by its sha256, D302). Remove it afterwards with
#               media/screenshots/make-demo.sh --remove (the symlink first,
#               with a plain rm, so the weights it points at are never walked).
# Needs:        bash, python3, sqlite3.
set -euo pipefail

here="$(cd -P "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
DEMO="${DEMO:-/Users/Shared/Wipemark Demo}"
models=""
theme="light"
only_theme=0
remove=0
while [ $# -gt 0 ]; do
  case "$1" in
    --theme) theme="$2"; shift 2 ;;
    --theme-only) theme="$2"; only_theme=1; shift 2 ;;
    --models) models="$2"; shift 2 ;;
    --remove) remove=1; shift ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

if [ "$remove" = 1 ]; then
  # The link first, by itself and without -r: removing a symlink never
  # touches what it points at, and the folder below no longer holds one.
  if [ -L "$DEMO/models" ]; then command rm -f "$DEMO/models"; fi
  command rm -rf "$DEMO"
  echo "removed $DEMO"
  exit 0
fi

db="$DEMO/.data/wipemark.db"
seed() {
  sqlite3 "$db" "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);"
  while [ $# -gt 1 ]; do
    sqlite3 "$db" "INSERT OR REPLACE INTO settings(key, value) VALUES ('$1', '$2');"
    shift 2
  done
}

if [ "$only_theme" = 1 ]; then
  seed ui.theme "\"$theme\""
  exit 0
fi

if [ -z "$models" ]; then echo "--models <dir> is required: the folder holding the GGUF weights" >&2; exit 2; fi
mkdir -p "$DEMO/.data"

python3 - "$DEMO" <<'EOF'
import os, sys
d = sys.argv[1]
zw, wj, shy, nb = "​", "⁠", "­", " "
cyr_a, cyr_o, cyr_e = "а", "о", "е"

blog = (
    "# Five things we learned shipping a small desktop app\n\n"
    "Shipping a desktop app is still harder than it looks. The build is the easy part;"
    " the long tail of install" + shy + "ers, signing and first-launch surprises is where" + zw + " the weeks go.\n\n"
    "1. **Start with the installer.** We wrote the pack" + shy + "aging script in week one, not week ten,"
    " and every build since has been one we could hand to a tester.\n"
    "2. **Measure the cold start.** Nobody complains about a slow" + wj + " app; they just stop opening it.\n"
    "3. **Keep settings in one place.** One table, one row per key, and a page for every row.\n"
    "4. **Say what does not work yet.** A greyed-out button that explains itself saves a support" + nb + "ticket.\n"
    "5. **Ship small, ship often.** Our best rele" + cyr_a + "ses were the boring ones.\n"
)
notes = (
    "Product sync" + nb + "— Tuesday, 14 October\n\n"
    "Attendees: Ana, Marco, Priya, Tom\n\n"
    "- Beta feedback: onboarding is too long;" + zw + " cut the welcome tour to two screens.\n"
    "- Priya drafts the release notes by Friday.\n"
    "- Marco: the crash on resume is fixed in build 412 and needs a second pair of eyes" + wj + ".\n"
    "- Tom checks the new icons on a dark menu bar.\n"
    "- Next sync moves to Thursday 10:00, same r" + cyr_o + "om.\n"
)
mail = (
    "Hi Sam,\n\n"
    "Thanks for sending over the proposal so quickly. I read it on the train this morning, and I think"
    " the overall direction is right, but the timeline in section three feels optimistic to me," + zw +
    " especially with the holidays coming up.\n\n"
    "Could we move the first milestone to the end of January and keep the rest as it is? That would give"
    " the design team room to finish the research they started last month, and it would let us test the"
    " onboarding flow with real cust" + shy + "omers before we commit to it.\n\n"
    "I am free on Thursday afternoon if you would like to talk it through.\n\n"
    "Best,\nAlex\n"
)
release = (
    "# Release notes 2.4\n\n"
    "- Search is faster in large folders.\n"
    "- The export dialog remembers the last folder you used.\n"
    "- Fixed a crash when a file was renamed during sync.\n"
)
# Long enough that Compare scrolls, for the clip of it.
team = "\n\n".join([
    "# Quarterly update from the apps team",
    "This quarter was mostly about stability. We closed more bugs than we opened for the first time in a year,"
    " and the crash rate on launch fell by half after we moved the database migration off the main thread." + zw,
    "The new onboarding flow shipped to everyone in August. Fewer people now drop out before the first sync,"
    " although the welcome tour is still longer than it needs to be, and we plan to cut it to two screens.",
    "Search got faster in large folders. The index is now built in the background and kept up to date as files"
    " change, so the first search after a restart no longer stalls the window for several seconds.",
    "We also spent time on accessibility. Every control in the settings window can now be reached with the"
    " keyboard, and the colours in the dark theme pass the contrast checks we set ourselves in the spring.",
    "Not everything went to plan. The export redesign slipped to next quarter because the file picker behaved"
    " differently on each platform, and we would rather ship it late than ship it twice.",
    "Next quarter we want to finish the export work, make sync conflicts easier to under" + shy + "stand, and spend"
    " a week on nothing but the small annoyances people keep writing to us about.",
    "Thanks to everyone who filed a bug, answered a survey or sat through a test session with us."
    " It made a real difference.",
]) + "\n"
for name, text in [("blog-draft.md", blog), ("meeting-notes.txt", notes),
                   ("email-draft.txt", mail), ("release-notes.md", release),
                   ("team-update.md", team)]:
    with open(os.path.join(d, name), "w", encoding="utf-8") as f:
        f.write(text)
EOF

command cp -f "$root/fixtures/image/gemini/torch-1025.png" "$DEMO/sticker-gemini.png"
command cp -f "$root/fixtures/image/grok-x/HUVfYqgWkAAOhiJ.jpeg" "$DEMO/grok-post.jpeg"
command cp -f "$root/fixtures/image/c2pa-jumbf.jpg" "$DEMO/photo-c2pa.jpg"
command cp -f "$root/fixtures/image/xmp-provenance.jpg" "$DEMO/photo-xmp.jpg"

if [ ! -e "$DEMO/models" ]; then ln -s "$models" "$DEMO/models"; fi

seed \
  ui.setup.done 'true' \
  ui.language '"en-US"' \
  ui.theme "\"$theme\"" \
  hotkey.panel '""' \
  engine.serves '"machine"' \
  models.dir "\"$DEMO/models\"" \
  models.rewrite '"gemma-4-12b-it-qat-ud-q4"' \
  mcp.enabled 'true'

ls -la "$DEMO"
sqlite3 "$db" "select key, value from settings order by key"
