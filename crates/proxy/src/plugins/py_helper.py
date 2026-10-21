"""CPython helpers for optillm-rs plugins that execute code (run in a subprocess).

`executecode_execute` and `coc_execute` are upstream's `execute_code`
functions from optillm/plugins/executecode_plugin.py and coc_plugin.py
(v0.4.0, Apache-2.0), unchanged, so notebook outputs and error strings match.
They need `nbformat`, `nbconvert` and an `ipykernel` python3 kernel.
`privacy_anonymize` is the Presidio part of privacy_plugin.py (needs
presidio-analyzer / -anonymizer and spaCy's en_core_web_lg, which it downloads
on first use exactly like upstream).

Protocol: stdin = JSON {"op": ..., ...}; argv[1] = path for the JSON result.
"""
import ast
import json
import os
import sys
import tempfile


def executecode_execute(code: str) -> str:
    """Execute Python code in a Jupyter notebook environment."""
    import nbformat
    from nbconvert.preprocessors import ExecutePreprocessor

    notebook = nbformat.v4.new_notebook()
    notebook['cells'] = [nbformat.v4.new_code_cell(code)]

    # Convert notebook to JSON string
    notebook_json = nbformat.writes(notebook)

    # Convert JSON string to bytes
    notebook_bytes = notebook_json.encode('utf-8')

    with tempfile.NamedTemporaryFile(mode='wb', suffix='.ipynb', delete=False) as tmp:
        tmp.write(notebook_bytes)
        tmp.flush()
        tmp_name = tmp.name

    try:
        with open(tmp_name, 'r', encoding='utf-8') as f:
            nb = nbformat.read(f, as_version=4)
        ep = ExecutePreprocessor(timeout=30, kernel_name='python3')
        ep.preprocess(nb, {'metadata': {'path': './'}})

        # Extract the output
        output = ""
        for cell in nb.cells:
            if cell.cell_type == 'code' and cell.outputs:
                for output_item in cell.outputs:
                    if output_item.output_type == 'stream':
                        output += output_item.text
                    elif output_item.output_type == 'execute_result':
                        output += str(output_item.data.get('text/plain', ''))

        return output.strip()
    finally:
        os.unlink(tmp_name)


def sanitize_code(code: str) -> str:
    """Prepare code for safe execution by removing problematic visualization code."""
    lines = code.split('\n')
    safe_lines = []
    for line in lines:
        if any(x in line.lower() for x in ['matplotlib', 'plt.', '.plot(', '.show(', 'figure', 'subplot']):
            safe_lines.append(f"# {line}  # Removed for safety")
        else:
            safe_lines.append(line)

    return '\n'.join(safe_lines)


def coc_execute(code: str):
    """Attempt to execute the code using Jupyter notebook kernel and return result or error."""
    try:
        import nbformat
        from nbconvert.preprocessors import ExecutePreprocessor

        sanitized_code = sanitize_code(code)
        notebook = nbformat.v4.new_notebook()
        enhanced_code = f"""
{sanitized_code}

# Capture the answer variable for output
if 'answer' in locals():
    print(f"ANSWER_RESULT: {{answer}}")
else:
    print("ANSWER_RESULT: No answer variable found")
"""

        notebook['cells'] = [nbformat.v4.new_code_cell(enhanced_code)]
        notebook_json = nbformat.writes(notebook)
        notebook_bytes = notebook_json.encode('utf-8')

        with tempfile.NamedTemporaryFile(mode='wb', suffix='.ipynb', delete=False) as tmp:
            tmp.write(notebook_bytes)
            tmp.flush()
            tmp_name = tmp.name

        try:
            with open(tmp_name, 'r', encoding='utf-8') as f:
                nb = nbformat.read(f, as_version=4)

            ep = ExecutePreprocessor(timeout=30, kernel_name='python3')
            ep.preprocess(nb, {'metadata': {'path': './'}})

            output = ""
            error_output = ""

            for cell in nb.cells:
                if cell.cell_type == 'code' and cell.outputs:
                    for output_item in cell.outputs:
                        if output_item.output_type == 'stream':
                            if output_item.name == 'stdout':
                                output += output_item.text
                            elif output_item.name == 'stderr':
                                error_output += output_item.text
                        elif output_item.output_type == 'execute_result':
                            output += str(output_item.data.get('text/plain', ''))
                        elif output_item.output_type == 'error':
                            error_output += f"{output_item.ename}: {output_item.evalue}"

            if error_output:
                return None, error_output

            output = output.strip()

            if "ANSWER_RESULT:" in output:
                answer_line = [line for line in output.split('\n') if 'ANSWER_RESULT:' in line][-1]
                answer_str = answer_line.split('ANSWER_RESULT:', 1)[1].strip()

                if answer_str == "No answer variable found":
                    error = "Code executed but did not produce an answer variable"
                    return None, error

                try:
                    answer = ast.literal_eval(answer_str)
                except (ValueError, SyntaxError):
                    answer = answer_str

                return answer, None
            else:
                if output:
                    return output, None
                else:
                    error = "Code executed but produced no output"
                    return None, error

        finally:
            try:
                os.unlink(tmp_name)
            except:  # noqa: E722
                pass

    except Exception as e:
        error = f"Notebook execution failed: {str(e)}"
        return None, error


def privacy_anonymize(text: str):
    """Anonymisation half of upstream privacy_plugin.run (Presidio + spaCy en_core_web_lg)."""
    import spacy
    from presidio_analyzer import AnalyzerEngine
    from presidio_anonymizer import AnonymizerEngine, OperatorConfig
    from presidio_anonymizer.operators import Operator, OperatorType

    class InstanceCounterAnonymizer(Operator):
        REPLACING_FORMAT = "<{entity_type}_{index}>"

        def operate(self, text: str, params=None) -> str:
            entity_type = params["entity_type"]
            entity_mapping = params["entity_mapping"]
            entity_mapping_for_type = entity_mapping.get(entity_type)
            if not entity_mapping_for_type:
                new_text = self.REPLACING_FORMAT.format(entity_type=entity_type, index=0)
                entity_mapping[entity_type] = {}
            else:
                if text in entity_mapping_for_type:
                    return entity_mapping_for_type[text]
                previous_index = self._get_last_index(entity_mapping_for_type)
                new_text = self.REPLACING_FORMAT.format(entity_type=entity_type, index=previous_index + 1)
            entity_mapping[entity_type][text] = new_text
            return new_text

        @staticmethod
        def _get_last_index(entity_mapping_for_type) -> int:
            def get_index(value: str) -> int:
                return int(value.split("_")[-1][:-1])
            indices = [get_index(v) for v in entity_mapping_for_type.values()]
            return max(indices)

        def validate(self, params=None) -> None:
            if "entity_mapping" not in params:
                raise ValueError("An input Dict called `entity_mapping` is required.")
            if "entity_type" not in params:
                raise ValueError("An entity_type param is required.")

        def operator_name(self) -> str:
            return "entity_counter"

        def operator_type(self) -> OperatorType:
            return OperatorType.Anonymize

    model_name = "en_core_web_lg"
    if not spacy.util.is_package(model_name):
        spacy.cli.download(model_name)
    analyzer = AnalyzerEngine()
    analyzer.analyze(text="warm up", language="en")
    analyzer_results = analyzer.analyze(text=text, language="en")
    anonymizer_engine = AnonymizerEngine()
    anonymizer_engine.add_anonymizer(InstanceCounterAnonymizer)
    entity_mapping = dict()
    anonymized_result = anonymizer_engine.anonymize(
        text,
        analyzer_results,
        {"DEFAULT": OperatorConfig("entity_counter", {"entity_mapping": entity_mapping})},
    )
    return anonymized_result.text, entity_mapping


def literal_str(text: str) -> str:
    """str(ast.literal_eval(text)) with upstream's bare-except fallback to the text."""
    try:
        answer = ast.literal_eval(text)
    except:  # noqa: E722
        answer = text
    return str(answer)


def main():
    req = json.loads(sys.stdin.read())
    op = req["op"]
    if op == "executecode":
        try:
            result = {"ok": executecode_execute(req["code"])}
        except Exception as e:
            result = {"error": str(e)}
    elif op == "coc":
        answer, error = coc_execute(req["code"])
        result = {"answer": None if error is not None else str(answer), "error": error}
    elif op == "privacy":
        try:
            text, mapping = privacy_anonymize(req["text"])
            result = {"text": text, "mapping": mapping}
        except Exception as e:
            result = {"error": str(e)}
    elif op == "literal_str":
        result = {"ok": literal_str(req["text"])}
    else:
        result = {"error": f"unknown op {op}"}
    with open(sys.argv[1], "w") as fh:
        json.dump(result, fh)


if __name__ == "__main__":
    main()
