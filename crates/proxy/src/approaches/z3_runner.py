"""Execution environment for the z3 approach (run by optillm-rs in a subprocess).

`prepare_execution_globals` and `execute_code_in_process` are copied from
upstream optillm/z3_solver.py (v0.4.0, Apache-2.0) so solver code sees exactly
the same globals, stdout capture and traceback text.

Protocol: read the solver code from stdin, write a JSON result to argv[1]:
    {"kind": "syntax", "message": str(SyntaxError)}
    {"kind": "error", "output": traceback}
    {"kind": "success", "output": captured stdout}
"""
import ast
import contextlib
import io
import json
import math
import sys
import traceback


def prepare_execution_globals():
    execution_globals = {
        'print': print,
        '__builtins__': {
            'True': True,
            'False': False,
            'None': None,
            'abs': abs,
            'float': float,
            'int': int,
            'len': len,
            'max': max,
            'min': min,
            'round': round,
            'sum': sum,
            'complex': complex,
        }
    }

    # Add common math functions
    execution_globals.update({
        'log': math.log,
        'log2': math.log2,
        'sqrt': math.sqrt,
        'exp': math.exp,
        'sin': math.sin,
        'cos': math.cos,
        'tan': math.tan,
        'pi': math.pi,
        'e': math.e,
    })

    # Add complex number support
    execution_globals['I'] = complex(0, 1)
    execution_globals['Complex'] = complex

    return execution_globals


def execute_code_in_process(code: str):
    import z3
    import sympy
    import math
    import itertools
    from fractions import Fraction

    execution_globals = prepare_execution_globals()

    # Add Z3 specific functions
    z3_whitelist = set(dir(z3))
    execution_globals.update({name: getattr(z3, name) for name in z3_whitelist})

    # Add SymPy specific functions
    sympy_whitelist = set(dir(sympy))
    execution_globals.update({name: getattr(sympy, name) for name in sympy_whitelist})

    # Ensure key Z3 and SymPy components are available
    execution_globals.update({
        'z3': z3,
        'sympy': sympy,
        'Solver': z3.Solver,
        'solver': z3.Solver,
        'Optimize': z3.Optimize,
        'sat': z3.sat,
        'unsat': z3.unsat,
        'unknown': z3.unknown,
        'Real': z3.Real,
        'Int': z3.Int,
        'Bool': z3.Bool,
        'And': z3.And,
        'Or': z3.Or,
        'Not': z3.Not,
        'Implies': z3.Implies,
        'If': z3.If,
        'Sum': z3.Sum,
        'ForAll': z3.ForAll,
        'Exists': z3.Exists,
        'model': z3.Model,
        'Symbol': sympy.Symbol,
        'solve': sympy.solve,
        'simplify': sympy.simplify,
        'expand': sympy.expand,
        'factor': sympy.factor,
        'diff': sympy.diff,
        'integrate': sympy.integrate,
        'limit': sympy.limit,
        'series': sympy.series,
    })

    # Add custom functions
    def as_numerical(x):
        if z3.is_expr(x):
            if z3.is_int_value(x) or z3.is_rational_value(x):
                return float(x.as_decimal(20))
            elif z3.is_algebraic_value(x):
                return x.approx(20)
        return float(x)

    execution_globals['as_numerical'] = as_numerical

    def Mod(x, y):
        return x % y

    execution_globals['Mod'] = Mod

    def Rational(numerator, denominator=1):
        return z3.Real(str(Fraction(numerator, denominator)))

    execution_globals['Rational'] = Rational

    output_buffer = io.StringIO()
    with contextlib.redirect_stdout(output_buffer):
        try:
            exec(code, execution_globals, {})
        except Exception:
            return ("error", traceback.format_exc())
    return ("success", output_buffer.getvalue())


def main():
    code = sys.stdin.read()
    try:
        ast.parse(code)
    except SyntaxError as e:
        result = {"kind": "syntax", "message": str(e)}
    else:
        status, output = execute_code_in_process(code)
        result = {"kind": status, "output": output}
    with open(sys.argv[1], "w") as fh:
        json.dump(result, fh)


if __name__ == "__main__":
    main()
