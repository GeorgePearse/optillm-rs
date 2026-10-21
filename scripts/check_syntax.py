#!/usr/bin/env python3
"""Syntax-check YAML or TOML files: `check_syntax.py yaml|toml FILE...`."""
import sys


def check_yaml(paths):
    import yaml

    class Loader(yaml.SafeLoader):
        """SafeLoader that accepts tags such as mkdocs' `!!python/name:`."""

    Loader.add_multi_constructor("", lambda loader, suffix, node: None)
    for p in paths:
        with open(p) as fh:
            list(yaml.load_all(fh, Loader=Loader))


def check_toml(paths):
    try:
        import tomllib
    except ModuleNotFoundError:  # Python < 3.11
        import tomli as tomllib
    for p in paths:
        with open(p, "rb") as fh:
            tomllib.load(fh)


kind, files = sys.argv[1], sys.argv[2:]
{"yaml": check_yaml, "toml": check_toml}[kind](files)
