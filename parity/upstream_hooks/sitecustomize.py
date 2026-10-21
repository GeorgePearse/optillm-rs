"""Loaded into the upstream optillm process by the parity harness (PYTHONPATH).

Upstream draws from the global `random` module (mcts, rstar, mars, ...) and
never seeds it. To compare call sequences we reseed it at the start of every
request; the Rust server does the same with its CPython-compatible MT19937
when started with OPTILLM_RANDOM_SEED. Nothing else about upstream is changed.
"""
import os

_seed = os.environ.get("PARITY_SEED")
if _seed is not None:
    import random

    import flask

    _orig_dispatch = flask.Flask.full_dispatch_request

    def _seeded_dispatch(self):
        random.seed(int(_seed))
        return _orig_dispatch(self)

    flask.Flask.full_dispatch_request = _seeded_dispatch
