"""Loaded into the upstream optillm process by the parity harness (PYTHONPATH).

Upstream draws from the global `random` module (mcts, rstar, mars, ...) and
never seeds it. To compare call sequences we reseed it at the start of every
request; the Rust server does the same with its CPython-compatible MT19937
when started with OPTILLM_RANDOM_SEED.

When a fixture selects optillm-rs's SearXNG search backend
(OPTILLM_SEARCH_BACKEND=searxng), upstream's Selenium GoogleSearcher is given
the same HTTP backend instead of a Chrome window on google.com, so web_search
and deep_research can be compared. The rest of its search() contract is kept:
results as [{"title", "url", "snippet"}], [] on failure, and the post-search
delay (random.randint(4, 32) when unset), scaled by OPTILLM_SEARCH_DELAY_SCALE
exactly as optillm-rs scales it. Nothing else about upstream is changed.
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


if os.environ.get("OPTILLM_SEARCH_BACKEND") == "searxng":
    import json as _json
    import time as _time
    import urllib.parse as _parse
    import urllib.request as _request

    import importlib.machinery as _machinery

    def _init(self, headless=False, timeout=30):
        self.timeout = timeout
        self.headless = headless
        self.driver = object()

    def _search(self, query, num_results=10, delay_seconds=None):
        import random

        base = os.environ["OPTILLM_SEARXNG_URL"].rstrip("/")
        url = f"{base}/search?" + _parse.urlencode({"q": query, "format": "json"})
        try:
            with _request.urlopen(url, timeout=30) as resp:
                data = _json.loads(resp.read().decode())
        except Exception:
            return []
        if delay_seconds is None:
            delay_seconds = random.randint(4, 32)
        if delay_seconds > 0:
            _time.sleep(delay_seconds * float(os.environ.get("OPTILLM_SEARCH_DELAY_SCALE", "1")))
        return [{"title": r.get("title", ""), "url": r.get("url", ""), "snippet": r.get("content", "")}
                for r in data.get("results", [])[:num_results]]

    def _close(self):
        self.driver = None

    # Upstream loads plugin files as fresh modules (spec_from_file_location),
    # and deep_research imports the package module: patch every copy.
    _orig_exec = _machinery.SourceFileLoader.exec_module

    def _exec_module(self, module):
        _orig_exec(self, module)
        searcher = getattr(module, "GoogleSearcher", None)
        if searcher is not None and hasattr(module, "BrowserSessionManager"):
            searcher.__init__ = _init
            searcher.search = _search
            searcher.close = _close

    _machinery.SourceFileLoader.exec_module = _exec_module
