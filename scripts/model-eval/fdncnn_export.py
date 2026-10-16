#!/usr/bin/env python3
"""FDnCNN (colour) from KAIR's weights to ONNX, with the facts that pin it.

What it is for
--------------
Step E12-R10 of the E12-R series (`docs/plan/E12-R10-model-evaluation.md`
§2.2), filed 2026-10-08 by the coordinator from the owner's spec
`wipemark-recon-spec-2026-10-08` (`07-model-evaluation.md` §2; the owner's
S10). Written 2026-10-09, ahead of the run, at the owner's request that all
code work be finished first. It runs only if `trigger.py fdncnn` says
"FDnCNN to be run".

FDnCNN is Kai Zhang's flexible DnCNN (Zhang, Zuo, Zhang, "FFDNet", 2018,
and the KAIR toolbox, https://github.com/cszn/KAIR): 20 convolutions 3 × 3
with 64 channels and ReLU between them, no batch normalisation, 4 channels
in (`[R, G, B, σ_map/255]`) and 3 out — the denoised picture itself, not a
residual. Its receptive field is 41 × 41 and it has 668 803 parameters
(2.7 MB in FP32). KAIR is **not** vendored: the network below is restated
from `models/network_dncnn.py` (`class FDnCNN`, built with
`models/basicblock.py`'s `conv` and `sequential`, which flattens nested
blocks into one `nn.Sequential`), so the state dict's keys are
`model.0.weight`, `model.0.bias`, `model.2.…`, …, `model.38.…` — the
convolutions at the even indices, a ReLU after each but the last. The
weights load **strictly**: a key KAIR named otherwise is a refusal here,
never a silent half-load.

What it does
------------
1. Hashes the weights file (sha256) and, if `--sha256` is given, refuses
   on a mismatch.
2. Builds the network (torch, imported here and nowhere at module level),
   loads the weights strictly, puts it in eval mode, checks its parameter
   count against 668 803.
3. Exports it to ONNX with a dynamic batch, height and width (`opset`
   17 by default), input `x` of shape `(N, 4, H, W)`, output `y` of shape
   `(N, 3, H, W)`.
4. If onnxruntime imports, runs the ONNX model and the torch model on one
   random 4 × 64 × 80 input and refuses if they differ by more than 1e-4.
5. Writes `<out>.json` beside the model: the weights' and the model's
   sha256, the weights' URL, KAIR's commit as given (`--kair-commit`; the
   checkout the network was read against), torch's, onnx's and
   onnxruntime's versions, the opset, the parameter count, the date.
   `fdncnn_run.py` copies that JSON into every output.

How to run it
-------------
    python3 scripts/model-eval/fdncnn_export.py export --weights model_zoo/fdncnn_color.pth \
        --out model_zoo/fdncnn_color.onnx [--sha256 <expected>] [--kair-commit <sha>] [--opset 17]
    python3 scripts/model-eval/fdncnn_export.py selftest

The weights: `fdncnn_color.pth` from KAIR's model zoo,
https://github.com/cszn/KAIR/releases/download/v1.0/fdncnn_color.pth — its
sha256 goes into `scripts/model-eval/README.md` once the host has it.

What it needs
-------------
Python 3.10+, numpy, `torch` (CPU is enough), `onnx`; `onnxruntime` for the
check in step 4 (skipped and said when absent). The versions to pin are in
`scripts/model-eval/README.md`. `selftest` needs none of them.

What its output means
---------------------
The ONNX model and its JSON. Exit 0 when written and (if onnxruntime was
there) checked, 2 on a refusal: a sha256 that does not match, weights that
do not load strictly, a parameter count that is not FDnCNN's, an ONNX
model that does not agree with torch.
"""

import argparse
import json
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import evalkit as ek  # noqa: E402

WEIGHTS_URL = "https://github.com/cszn/KAIR/releases/download/v1.0/fdncnn_color.pth"
IN_NC, OUT_NC, NC, NB = 4, 3, 64, 20


def layout(in_nc=IN_NC, out_nc=OUT_NC, nc=NC, nb=NB):
    """The network as data: (state-dict index, in, out, ReLU after) per convolution, as KAIR flattens it."""
    convs = [(in_nc, nc, True)] + [(nc, nc, True)] * (nb - 2) + [(nc, out_nc, False)]
    out, index = [], 0
    for cin, cout, relu in convs:
        out.append((index, cin, cout, relu))
        index += 2 if relu else 1
    return out


def parameter_count(in_nc=IN_NC, out_nc=OUT_NC, nc=NC, nb=NB):
    return sum(cin * cout * 9 + cout for _i, cin, cout, _r in layout(in_nc, out_nc, nc, nb))


def receptive_field(nb=NB):
    return 1 + 2 * nb


def build():
    """KAIR's `FDnCNN(in_nc=4, out_nc=3, nc=64, nb=20, act_mode='R')`, restated."""
    import torch.nn as nn

    class FDnCNN(nn.Module):
        def __init__(self):
            super().__init__()
            layers = []
            for _index, cin, cout, relu in layout():
                layers.append(nn.Conv2d(cin, cout, kernel_size=3, stride=1, padding=1, bias=True))
                if relu:
                    layers.append(nn.ReLU(inplace=True))
            self.model = nn.Sequential(*layers)

        def forward(self, x):
            return self.model(x)

    return FDnCNN()


def load(weights):
    import torch

    net = build()
    state = torch.load(weights, map_location="cpu", weights_only=True)
    if isinstance(state, dict) and "params" in state and isinstance(state["params"], dict):
        state = state["params"]
    try:
        net.load_state_dict(state, strict=True)
    except RuntimeError as e:
        raise ek.Refusal(f"{weights}: does not load strictly into FDnCNN as restated from KAIR: {e}") from e
    net.eval()
    for p in net.parameters():
        p.requires_grad_(False)
    n = sum(p.numel() for p in net.parameters())
    if n != parameter_count():
        raise ek.Refusal(f"{weights}: {n} parameters, FDnCNN colour has {parameter_count()}")
    return net


def export(args):
    import numpy as np
    import torch

    got = ek.sha256_file(args.weights)
    if args.sha256 and got != args.sha256.lower():
        raise ek.Refusal(f"{args.weights}: sha256 {got}, expected {args.sha256}; refused")
    net = load(args.weights)
    dummy = torch.zeros(1, IN_NC, 64, 64)
    os.makedirs(os.path.dirname(os.path.abspath(args.out)), exist_ok=True)
    torch.onnx.export(net, dummy, args.out, input_names=["x"], output_names=["y"], opset_version=args.opset,
                      dynamic_axes={"x": {0: "n", 2: "h", 3: "w"}, "y": {0: "n", 2: "h", 3: "w"}})
    facts = {"model": "fdncnn_color", "weights": os.path.basename(args.weights), "weights_sha256": got,
             "weights_url": WEIGHTS_URL, "kair_commit": args.kair_commit, "onnx": os.path.basename(args.out),
             "onnx_sha256": ek.sha256_file(args.out), "opset": args.opset, "parameters": parameter_count(),
             "receptive_field": receptive_field(), "torch": torch.__version__, "date": time.strftime("%Y-%m-%d")}
    try:
        import onnx

        facts["onnx_version"] = onnx.__version__
    except ImportError:
        facts["onnx_version"] = None
    try:
        import onnxruntime as ort
    except ImportError:
        facts["onnxruntime"] = None
        facts["checked_against_torch"] = "not checked: onnxruntime is not installed"
    else:
        rng = np.random.default_rng(0)
        x = rng.random((1, IN_NC, 64, 80), dtype=np.float32)
        x[:, 3] = 10.0 / 255.0
        with torch.no_grad():
            want = net(torch.from_numpy(x)).numpy()
        sess = ort.InferenceSession(args.out, providers=["CPUExecutionProvider"])
        have = sess.run(["y"], {"x": x})[0]
        diff = float(np.abs(have - want).max())
        if diff > 1e-4:
            raise ek.Refusal(f"{args.out}: ONNX and torch differ by {diff} on a 64 × 80 input; refused")
        facts["onnxruntime"] = ort.__version__
        facts["checked_against_torch"] = {"input": [1, IN_NC, 64, 80], "max_abs_diff": diff}
    ek.write_json(args.out + ".json", facts)
    print(json.dumps(facts, indent=1))
    return 0


# ───────────────────────────────────────────────────────── selftest

def _t_the_restated_network_is_kairs_shape():
    convs = layout()
    assert len(convs) == 20, len(convs)
    assert [c[0] for c in convs] == list(range(0, 40, 2)), [c[0] for c in convs]
    assert convs[0][1:] == (4, 64, True) and convs[-1][1:] == (64, 3, False)
    assert parameter_count() == 668803, parameter_count()
    assert receptive_field() == 41
    assert abs(parameter_count() * 4 / 1e6 - 2.675) < 0.01


def _t_the_restated_network_loads_its_own_state_dict_when_torch_is_there():
    try:
        import torch
    except ImportError:
        print("        (torch is not installed: the module is not built here)")
        return
    net = build()
    keys = list(net.state_dict())
    assert keys[0] == "model.0.weight" and keys[-1] == "model.38.bias", (keys[0], keys[-1])
    assert sum(p.numel() for p in net.parameters()) == parameter_count()
    y = net(torch.zeros(1, 4, 16, 24))
    assert tuple(y.shape) == (1, 3, 16, 24)


def selftest():
    return ek.run_cases([
        ("the_restated_network_is_kairs_shape", _t_the_restated_network_is_kairs_shape),
        ("the_restated_network_loads_its_own_state_dict_when_torch_is_there",
         _t_the_restated_network_loads_its_own_state_dict_when_torch_is_there),
    ])


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = p.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("export")
    s.add_argument("--weights", required=True)
    s.add_argument("--out", required=True)
    s.add_argument("--sha256")
    s.add_argument("--kair-commit")
    s.add_argument("--opset", type=int, default=17)
    sub.add_parser("selftest")
    args = p.parse_args(argv)
    try:
        return selftest() if args.cmd == "selftest" else export(args)
    except ek.Refusal as e:
        print(f"fdncnn_export.py: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
