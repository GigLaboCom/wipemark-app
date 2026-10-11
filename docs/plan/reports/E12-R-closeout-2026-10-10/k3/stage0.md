# E12-R2 — Grok, stage 0 (2026-10-10)

Written by `scripts/corpus/manifest.py grok`: the format, the JPEG tables, sizes and clips from the files' bytes; the mark's place, size, kind, colour and outline by hand, from the sidecar.

| source | mark? | kind | format | subsampling / DQT | sizes / aspect | corner, margin | mark size | shadow / outline |
|---|---|---|---|---|---|---|---|---|
| grok.com | yes | wordmark ×30 | PNG ×30 | — | 1100×1100 (1:1) ×15, 1280×720 (16:9) ×15 | bottom-left 48×40 ×15, bottom-left 24×20 ×15 | 72×24 ×30 | none ×30 |
| grok-in-x | yes | wordmark ×4 | JPEG ×4 | 4:2:0 q90 (IJG) ×4 | 1100×1100 (1:1) ×2, 1280×720 (16:9) ×2 | bottom-left 48×40 ×2, bottom-left 24×20 ×2 | 72×24 ×4 | none ×4 |
| xai-api | no | — | PNG ×12 | — | 1100×1100 (1:1) ×6, 1280×720 (16:9) ×6 | — | — | — |
| grok-imagine-video | not captured | — | — | — | — | — | — | — |

## Every file

| file | source | mark | kind | format | subsampling / quality | size (aspect) | corner, margin | mark size | colour | shadow / outline | background | metadata | date | note |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| api/photo-1100x1100-00.png | xai-api | no | — | PNG | — | 1100×1100 (1:1) | — | — | — | — | content | none | 2026-10-10 | stand-in |
| api/photo-1100x1100-01.png | xai-api | no | — | PNG | — | 1100×1100 (1:1) | — | — | — | — | content | none | 2026-10-10 | stand-in |
| api/photo-1100x1100-02.png | xai-api | no | — | PNG | — | 1100×1100 (1:1) | — | — | — | — | content | none | 2026-10-10 | stand-in |
| api/photo-1100x1100-03.png | xai-api | no | — | PNG | — | 1100×1100 (1:1) | — | — | — | — | content | none | 2026-10-10 | stand-in |
| api/photo-1100x1100-04.png | xai-api | no | — | PNG | — | 1100×1100 (1:1) | — | — | — | — | content | none | 2026-10-10 | stand-in |
| api/photo-1100x1100-05.png | xai-api | no | — | PNG | — | 1100×1100 (1:1) | — | — | — | — | content | none | 2026-10-10 | stand-in |
| api/photo-1280x720-00.png | xai-api | no | — | PNG | — | 1280×720 (16:9) | — | — | — | — | content | none | 2026-10-10 | stand-in |
| api/photo-1280x720-01.png | xai-api | no | — | PNG | — | 1280×720 (16:9) | — | — | — | — | content | none | 2026-10-10 | stand-in |
| api/photo-1280x720-02.png | xai-api | no | — | PNG | — | 1280×720 (16:9) | — | — | — | — | content | none | 2026-10-10 | stand-in |
| api/photo-1280x720-03.png | xai-api | no | — | PNG | — | 1280×720 (16:9) | — | — | — | — | content | none | 2026-10-10 | stand-in |
| api/photo-1280x720-04.png | xai-api | no | — | PNG | — | 1280×720 (16:9) | — | — | — | — | content | none | 2026-10-10 | stand-in |
| api/photo-1280x720-05.png | xai-api | no | — | PNG | — | 1280×720 (16:9) | — | — | — | — | content | none | 2026-10-10 | stand-in |
| web/black-1100x1100-00.png | grok.com | yes | wordmark | PNG | — | 1100×1100 (1:1) | bottom-left 48×40 | 72×24 | white, ~50 % | none | black | none | 2026-10-10 | stand-in |
| web/black-1100x1100-01.png | grok.com | yes | wordmark | PNG | — | 1100×1100 (1:1) | bottom-left 48×40 | 72×24 | white, ~50 % | none | black | none | 2026-10-10 | stand-in |
| web/black-1100x1100-02.png | grok.com | yes | wordmark | PNG | — | 1100×1100 (1:1) | bottom-left 48×40 | 72×24 | white, ~50 % | none | black | none | 2026-10-10 | stand-in |
| web/black-1280x720-00.png | grok.com | yes | wordmark | PNG | — | 1280×720 (16:9) | bottom-left 24×20 | 72×24 | white, ~50 % | none | black | none | 2026-10-10 | stand-in |
| web/black-1280x720-01.png | grok.com | yes | wordmark | PNG | — | 1280×720 (16:9) | bottom-left 24×20 | 72×24 | white, ~50 % | none | black | none | 2026-10-10 | stand-in |
| web/black-1280x720-02.png | grok.com | yes | wordmark | PNG | — | 1280×720 (16:9) | bottom-left 24×20 | 72×24 | white, ~50 % | none | black | none | 2026-10-10 | stand-in |
| web/gray-50-1100x1100-06.png | grok.com | yes | wordmark | PNG | — | 1100×1100 (1:1) | bottom-left 48×40 | 72×24 | white, ~50 % | none | gray-50 | none | 2026-10-10 | stand-in |
| web/gray-50-1100x1100-07.png | grok.com | yes | wordmark | PNG | — | 1100×1100 (1:1) | bottom-left 48×40 | 72×24 | white, ~50 % | none | gray-50 | none | 2026-10-10 | stand-in |
| web/gray-50-1100x1100-08.png | grok.com | yes | wordmark | PNG | — | 1100×1100 (1:1) | bottom-left 48×40 | 72×24 | white, ~50 % | none | gray-50 | none | 2026-10-10 | stand-in |
| web/gray-50-1280x720-06.png | grok.com | yes | wordmark | PNG | — | 1280×720 (16:9) | bottom-left 24×20 | 72×24 | white, ~50 % | none | gray-50 | none | 2026-10-10 | stand-in |
| web/gray-50-1280x720-07.png | grok.com | yes | wordmark | PNG | — | 1280×720 (16:9) | bottom-left 24×20 | 72×24 | white, ~50 % | none | gray-50 | none | 2026-10-10 | stand-in |
| web/gray-50-1280x720-08.png | grok.com | yes | wordmark | PNG | — | 1280×720 (16:9) | bottom-left 24×20 | 72×24 | white, ~50 % | none | gray-50 | none | 2026-10-10 | stand-in |
| web/photo-1100x1100-09.png | grok.com | yes | wordmark | PNG | — | 1100×1100 (1:1) | bottom-left 48×40 | 72×24 | white, ~50 % | none | content | none | 2026-10-10 | stand-in |
| web/photo-1100x1100-10.png | grok.com | yes | wordmark | PNG | — | 1100×1100 (1:1) | bottom-left 48×40 | 72×24 | white, ~50 % | none | content | none | 2026-10-10 | stand-in |
| web/photo-1100x1100-11.png | grok.com | yes | wordmark | PNG | — | 1100×1100 (1:1) | bottom-left 48×40 | 72×24 | white, ~50 % | none | content | none | 2026-10-10 | stand-in |
| web/photo-1100x1100-12.png | grok.com | yes | wordmark | PNG | — | 1100×1100 (1:1) | bottom-left 48×40 | 72×24 | white, ~50 % | none | content | none | 2026-10-10 | stand-in |
| web/photo-1100x1100-13.png | grok.com | yes | wordmark | PNG | — | 1100×1100 (1:1) | bottom-left 48×40 | 72×24 | white, ~50 % | none | content | none | 2026-10-10 | stand-in |
| web/photo-1100x1100-14.png | grok.com | yes | wordmark | PNG | — | 1100×1100 (1:1) | bottom-left 48×40 | 72×24 | white, ~50 % | none | content | none | 2026-10-10 | stand-in |
| web/photo-1280x720-09.png | grok.com | yes | wordmark | PNG | — | 1280×720 (16:9) | bottom-left 24×20 | 72×24 | white, ~50 % | none | content | none | 2026-10-10 | stand-in |
| web/photo-1280x720-10.png | grok.com | yes | wordmark | PNG | — | 1280×720 (16:9) | bottom-left 24×20 | 72×24 | white, ~50 % | none | content | none | 2026-10-10 | stand-in |
| web/photo-1280x720-11.png | grok.com | yes | wordmark | PNG | — | 1280×720 (16:9) | bottom-left 24×20 | 72×24 | white, ~50 % | none | content | none | 2026-10-10 | stand-in |
| web/photo-1280x720-12.png | grok.com | yes | wordmark | PNG | — | 1280×720 (16:9) | bottom-left 24×20 | 72×24 | white, ~50 % | none | content | none | 2026-10-10 | stand-in |
| web/photo-1280x720-13.png | grok.com | yes | wordmark | PNG | — | 1280×720 (16:9) | bottom-left 24×20 | 72×24 | white, ~50 % | none | content | none | 2026-10-10 | stand-in |
| web/photo-1280x720-14.png | grok.com | yes | wordmark | PNG | — | 1280×720 (16:9) | bottom-left 24×20 | 72×24 | white, ~50 % | none | content | none | 2026-10-10 | stand-in |
| web/white-1100x1100-03.png | grok.com | yes | wordmark | PNG | — | 1100×1100 (1:1) | bottom-left 48×40 | 72×24 | white, ~50 % | none | white | none | 2026-10-10 | stand-in |
| web/white-1100x1100-04.png | grok.com | yes | wordmark | PNG | — | 1100×1100 (1:1) | bottom-left 48×40 | 72×24 | white, ~50 % | none | white | none | 2026-10-10 | stand-in |
| web/white-1100x1100-05.png | grok.com | yes | wordmark | PNG | — | 1100×1100 (1:1) | bottom-left 48×40 | 72×24 | white, ~50 % | none | white | none | 2026-10-10 | stand-in |
| web/white-1280x720-03.png | grok.com | yes | wordmark | PNG | — | 1280×720 (16:9) | bottom-left 24×20 | 72×24 | white, ~50 % | none | white | none | 2026-10-10 | stand-in |
| web/white-1280x720-04.png | grok.com | yes | wordmark | PNG | — | 1280×720 (16:9) | bottom-left 24×20 | 72×24 | white, ~50 % | none | white | none | 2026-10-10 | stand-in |
| web/white-1280x720-05.png | grok.com | yes | wordmark | PNG | — | 1280×720 (16:9) | bottom-left 24×20 | 72×24 | white, ~50 % | none | white | none | 2026-10-10 | stand-in |
| x/photo-1100x1100-00.jpg | grok-in-x | yes | wordmark | JPEG | 4:2:0 q90 (IJG) | 1100×1100 (1:1) | bottom-left 48×40 | 72×24 | white, ~50 % | none | content | none | 2026-10-10 | stand-in |
| x/photo-1100x1100-01.jpg | grok-in-x | yes | wordmark | JPEG | 4:2:0 q90 (IJG) | 1100×1100 (1:1) | bottom-left 48×40 | 72×24 | white, ~50 % | none | content | none | 2026-10-10 | stand-in |
| x/photo-1280x720-00.jpg | grok-in-x | yes | wordmark | JPEG | 4:2:0 q90 (IJG) | 1280×720 (16:9) | bottom-left 24×20 | 72×24 | white, ~50 % | none | content | none | 2026-10-10 | stand-in |
| x/photo-1280x720-01.jpg | grok-in-x | yes | wordmark | JPEG | 4:2:0 q90 (IJG) | 1280×720 (16:9) | bottom-left 24×20 | 72×24 | white, ~50 % | none | content | none | 2026-10-10 | stand-in |

Files with no sidecar entry (by-hand fields "to fill"): 0

**Gate (§4.3)**: not met — grok-imagine-video: not captured.
