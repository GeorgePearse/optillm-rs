"""Dump the field tree of openai-python's ChatCompletion pydantic models.

`model_dump()` of a response emits every declared field (None when absent)
plus any extra keys, recursively. The Rust client replays that normalisation
from the JSON this script writes, so `none` passthrough responses are
key-for-key identical to upstream's.

Usage (from the upstream venv):
    python parity/tools/dump_openai_schema.py crates/proxy/src/openai_schema.json
"""
import inspect
import json
import sys
import types
import typing

import pydantic
from openai.resources.chat.completions import Completions
from openai.types import Model
from openai.types.chat import ChatCompletion


def unwrap(tp):
    origin = typing.get_origin(tp)
    args = typing.get_args(tp)
    if origin in (typing.Union, types.UnionType):
        non_none = [a for a in args if a is not type(None)]
        if len(non_none) == 1:
            return unwrap(non_none[0])
        return ("union", [unwrap(a) for a in non_none])
    if origin is typing.Annotated:
        return unwrap(args[0])
    if origin in (list, typing.List):
        return ("list", unwrap(args[0]))
    return tp


def describe(tp):
    tp = unwrap(tp)
    if isinstance(tp, tuple):
        kind, inner = tp
        if kind == "list":
            return {"list": describe(inner)}
        variants = {}
        for v in inner:
            v = unwrap(v)
            if isinstance(v, tuple) and v[0] == "union":
                for sub in v[1]:
                    variants.update(describe(("union", [sub])).get("union", {}))
                continue
            if isinstance(v, type) and issubclass(v, pydantic.BaseModel):
                ann = v.model_fields.get("type")
                lit = typing.get_args(ann.annotation) if ann else ()
                key = lit[0] if lit else v.__name__
                variants[key] = describe(v)
        return {"union": variants} if variants else {}
    if isinstance(tp, type) and issubclass(tp, pydantic.BaseModel):
        return {
            "model": tp.__name__,
            "fields": [[f, describe(info.annotation)] for f, info in tp.model_fields.items()],
        }
    return {}


schema = {
    "ChatCompletion": describe(ChatCompletion),
    "Model": describe(Model),
    # Keyword arguments `Completions.create` accepts; anything else is a TypeError upstream.
    "create_params": [p for p in inspect.signature(Completions.create).parameters if p != "self"],
}
with open(sys.argv[1], "w") as fh:
    json.dump(schema, fh, indent=1)
    fh.write("\n")
