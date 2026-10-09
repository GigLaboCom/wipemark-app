# Models after the restoration (E12-R10) — the host's runbook

R10 answers "do we need a model?" with a measurement made **at a stated
point, by a stated rule** (`docs/plan/E12-R10-model-evaluation.md`). "Not
needed" is a result that closes the question. Two models, two jobs:
**FDnCNN** removes the noise the codec left after the inverse (Gemini first,
then Grok); **LaMa** makes up what is under a hole (Grok only — no Gemini map
has one). Nothing here goes into the product: no Rust, no catalogue model
(D166; R10 §7). A pixel a model made is *reconstructed*, never *restored*
(D310).

The scripts were written on 2026-10-09, ahead of the runs, because the owner
asked for all code work to be finished before the steps are unblocked. They
were written in a container with numpy and Pillow and **without** torch,
onnxruntime, OpenCV or lpips; every `selftest` runs without them (stubs, no
weights, no corpus). Nothing has been run on a model yet.

## The pieces

| script | what | needs |
|---|---|---|
| `evalkit.py` | the crop reader and the restated measures (σ_base, texture, consistency_px, PSNR/SSIM/ΔE2000 in the ROI), shared by all | numpy, Pillow |
| `trigger.py` | §2.1: the FDnCNN trigger over R1's run; §3.1: the LaMa trigger from R11 stage 1's hole shares; writes the decision report | numpy (through `evalkit.py`) |
| `fdncnn_export.py` | §2.2: KAIR's `fdncnn_color.pth` → ONNX (dynamic H/W), the network restated, weights' sha256 recorded | torch, onnx, (onnxruntime) |
| `fdncnn_run.py` | §2.3–§2.5: 18 variants per crop, F1–F7, one closing line; `model.jsonl` for `scripts/bench/report.py` | onnxruntime |
| `lama_run.py` | §3.2–§3.5: Big-LaMa in the hole mask, the mandatory composite, four variants, M1–M7, one closing line | LaMa's own venv, OpenCV, (lpips) |
| `baselines.py` | §3.2: NS, Telea (OpenCV) and the ring mean (numpy), and the one hole mask every method gets | OpenCV for NS/Telea |
| `ab.py` | Q-R7: blinded sheets (`pair`, `remainder`) and their scoring, any number of observers | numpy, Pillow |

Crops come from two places, in one layout (`evalkit.py`'s header):

* the bench — `recon_bench run … --export-crops DIR` (E12-R5), with a
  truth (`gt.png`); this is where F1/F2 and M2 are decided;
* R1's real files — `scripts/regress.py run … --export-crops DIR`, which
  calls `crates/wipemark-picture/examples/export_crops.rs` once per file
  (the CLI's `--json` carries neither the opacity nor the restored raster
  before the encoder). No truth; F3, F5 and the A/B read them.

## The order

1. **R6 and R8 accepted** (both gates, level A and B, and the owner's
   decisions). Until then nothing here runs: R10 is measured on R*, never
   on R0.
2. **The FDnCNN trigger** (§2.1), on R1 with the accepted R*:

   ```sh
   cargo build --release -p wipemark-cli --features wipemark-picture/planar-preview --locked
   WIPEMARK_INTERVAL=<the method R8 kept> python3 scripts/regress.py run --cli target/release/wipemark-cli \
       --baseline golden/baseline/<commit>/ --route lossy --select recon-jpeg-444 --select recon-jpeg-420 \
       --out reports/regress-r10-<date>/
   python3 scripts/model-eval/trigger.py fdncnn --run reports/regress-r10-<date>/ \
       --out docs/plan/reports/E12-R10-fdncnn-decision-<date>.md
   ```

   If the owner has answered Q-R7, add the blind look (20 items):

   ```sh
   python3 scripts/regress.py run … --export-crops crops/r1-<date> --crop-planar true --crop-refine <method>
   python3 scripts/model-eval/ab.py build --question remainder --crops crops/r1-<date> --n 20 --out ab/remainder
   # each observer opens ab/remainder/index.html, saves <name>.csv; nobody opens key.json
   python3 scripts/model-eval/ab.py score --key ab/remainder/key.json --answers owner=owner.csv --out ab/remainder/score.json
   python3 scripts/model-eval/trigger.py fdncnn --run … --ab ab/remainder/score.json --out …
   ```

   "FDnCNN not evaluated" closes it for Gemini: upload the decision report
   and stop.
3. **FDnCNN, only if triggered** (§2.2–§2.5):

   ```sh
   python3 scripts/model-eval/fdncnn_export.py export --weights model_zoo/fdncnn_color.pth \
       --sha256 <below> --kair-commit <below> --out model_zoo/fdncnn_color.onnx
   cargo run --release -p wipemark-picture --example recon_bench -- run --in bench/out/<run> \
       --config <R*> --out bench/out/<run>/results.jsonl --export-crops bench/out/<run>/crops
   python3 scripts/model-eval/fdncnn_run.py run --crops bench/out/<run>/crops/<R*> \
       --model model_zoo/fdncnn_color.onnx --results bench/out/<run>/results.jsonl --out bench/out/<run>/fdncnn
   python3 scripts/bench/report.py gates bench/out/<run>/fdncnn/model.jsonl \
       --candidate <R*>+fdncnn:<best> --baseline <R*>@crop --route lossy --targets 'jpeg*-q9[05],jpeg420-q85'
   python3 scripts/model-eval/ab.py build --question pair --crops bench/out/<run>/crops/<R*> --lossy-only \
       --images bench/out/<run>/fdncnn/images --a recon --b <best> --n 30 --out ab/fdncnn
   python3 scripts/model-eval/ab.py score --key ab/fdncnn/key.json --answers owner=owner.csv --out ab/fdncnn/score.json
   python3 scripts/model-eval/fdncnn_run.py run … --ab ab/fdncnn/score.json   # F6, and the closing line
   ```

   Copy `bench/out/<run>/fdncnn/report.md` into
   `docs/plan/reports/E12-R10-fdncnn-<date>.md`.
4. **The LaMa trigger** (§3.1), after R11 stage 1: write its hole shares as
   JSON or CSV (`profile, source, hole_share` — a fraction of the mark's
   support with `α̂ ≥ opaque_above`) and

   ```sh
   python3 scripts/model-eval/trigger.py lama --holes r11-holes.json \
       --out docs/plan/reports/E12-R10-lama-decision-<date>.md
   ```

   "Grok has no holes: LaMa not evaluated" closes it.
5. **LaMa, only if Grok has holes** (§3.2–§3.5), on Grok's crops with R11's
   profile and the accepted R* — exported with 128 px of context:

   ```sh
   python3 scripts/model-eval/lama_run.py run --crops bench/out/<grok-run>/crops/<R*> \
       --lama-dir ~/src/lama --checkpoint ~/models/big-lama --lpips --out bench/out/<grok-run>/lama
   python3 scripts/model-eval/ab.py build --question pair --crops … --holes-only \
       --images bench/out/<grok-run>/lama/images --a recon --b m-hard --n 30 --out ab/lama-holes   # M6
   python3 scripts/model-eval/ab.py build --question pair --crops … --holes-only \
       --images bench/out/<grok-run>/lama/images --a ns --b m-hard --n 20 --out ab/lama-ns          # M7
   python3 scripts/model-eval/lama_run.py run … --ab-holes ab/lama-holes/score.json --ab-ns ab/lama-ns/score.json
   ```

   `recon_bench run … --export-crops DIR --crop-pad 128` exports LaMa's
   128 px (64 by default); where a crop has less, `lama_run.py` mirror-pads
   the rest and records how much each crop had (`context_px`). R1's crops
   take `regress.py … --crop-pad 128`.

## The venvs

The model venv (FDnCNN, the controls, the A/B, the triggers) — the versions
below are the ones to pin; write the ones actually installed into the
report (every run's JSON records them too):

```sh
python3 -m venv ~/venv/wipemark-r10 && . ~/venv/wipemark-r10/bin/activate
pip install numpy==2.2.6 Pillow==12.3.0 torch==2.5.1 onnx==1.17.0 onnxruntime==1.20.1 \
    opencv-python-headless==4.10.0.84 lpips==0.1.4
```

LaMa's reference code is not on PyPI and its requirements are its own
(older torch, PyTorch Lightning, kornia, albumentations, hydra/OmegaConf);
it gets a venv of its own, with numpy and Pillow from its requirements and
this repository's scripts run from it:

```sh
git clone https://github.com/advimman/lama ~/src/lama && git -C ~/src/lama rev-parse HEAD   # record it below
python3 -m venv ~/venv/lama && . ~/venv/lama/bin/activate
pip install -r ~/src/lama/requirements.txt torch torchvision   # the versions its README names
pip install opencv-python-headless==4.10.0.84 lpips==0.1.4
```

`lama_run.lama_predictor` is the one integration point: it puts the
checkout on `sys.path`, reads `<checkpoint>/config.yaml`, loads
`<checkpoint>/models/best.ckpt` with `saicinpainting.training.trainers.load_checkpoint`
(`predict_only`, visualiser `noop`, `strict=False`, CPU) — as the checkout's
`bin/predict.py` does — and returns LaMa's raw `predicted_image`, so the
composite that is checked is ours.

## The weights — to fill in once, on the host

| what | where it comes from | sha256 |
|---|---|---|
| `fdncnn_color.pth` | https://github.com/cszn/KAIR/releases/download/v1.0/fdncnn_color.pth (KAIR's model zoo) | *to fill in* |
| KAIR, the network the export restates | https://github.com/cszn/KAIR, `models/network_dncnn.py` (`class FDnCNN`) | commit *to fill in* (`--kair-commit`) |
| `big-lama` (folder: `config.yaml`, `models/best.ckpt`) | LaMa's README, "Download pre-trained models" (`big-lama.zip`) | `best.ckpt` *to fill in*; `config.yaml` *to fill in* |
| advimman/lama | https://github.com/advimman/lama | commit *to fill in* |

A weights file whose sha256 differs from the one written here is refused by
`fdncnn_export.py --sha256`, and `lama_run.py` records the checkpoint's in
every report. Weights never go into git.

## Watchword (R10 §6)

Every report is uploaded as a FILE, ttl 0, read back without `expires_at`:

| report | key |
|---|---|
| the FDnCNN decision (§2.1) | `wipemark-recon-r10-fdncnn-decision-<date>` |
| the FDnCNN evaluation (§2.4, if triggered) | `wipemark-recon-r10-fdncnn-report-<date>` |
| the LaMa decision (§3.1) | `wipemark-recon-r10-lama-decision-<date>` |
| the LaMa evaluation (§3.4, if Grok has holes) | `wipemark-recon-r10-lama-report-<date>` |
| these scripts (2026-10-09) | `wipemark-recon-r10-scripts-report-2026-10-09` (the coordinator's upload) |

## Every script's `selftest`

```sh
for s in evalkit trigger fdncnn_export fdncnn_run lama_run baselines ab; do
    python3 scripts/model-eval/$s.py selftest || echo "$s FAILED"
done
python3 scripts/regress.py selftest
python3 scripts/bench/report.py selftest
```

R10 §5's three cases are `where_alpha_is_zero_fdncnn_changes_nothing`
(`fdncnn_run.py`), `outside_the_mask_the_output_is_the_input`
(`lama_run.py`) and `the_trigger_counts_left_marks_by_file_not_by_mark`
(`trigger.py`).
