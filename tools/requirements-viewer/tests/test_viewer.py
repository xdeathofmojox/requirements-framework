import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

PACKAGE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PACKAGE))
from graph import build_graph


def requirement(identifier, parents=None, **fields):
    return {
        "id": identifier,
        "title": identifier,
        "statement": "Required behavior.",
        "parents": parents or [],
        **fields,
    }


class GraphTests(unittest.TestCase):
    def test_layers_use_deepest_parent_across_files(self):
        graph = build_graph(
            [
                (
                    "system.json",
                    {
                        "requirements": [
                            requirement("ROOT"),
                            requirement("MID", ["ROOT"]),
                        ]
                    },
                ),
                (
                    "software.json",
                    {
                        "requirements": [
                            requirement(
                                "LEAF", ["ROOT", "MID"], custom={"owner": ["team"]}
                            )
                        ]
                    },
                ),
            ]
        )
        self.assertEqual([node["layer"] for node in graph["nodes"]], [0, 1, 2])
        self.assertEqual(len(graph["edges"]), 3)
        self.assertEqual(graph["issues"], [])
        self.assertEqual(graph["nodes"][2]["data"]["custom"], {"owner": ["team"]})
        self.assertEqual(graph["nodes"][2]["source"], "software.json")

    def test_cycles_and_missing_parents_do_not_get_false_layers(self):
        graph = build_graph(
            [
                (
                    "test",
                    {
                        "requirements": [
                            requirement("A", ["B"]),
                            requirement("B", ["A"]),
                            requirement("C", ["MISSING"]),
                            requirement("D", ["C"]),
                        ]
                    },
                )
            ]
        )
        self.assertTrue(all(node["layer"] is None for node in graph["nodes"]))
        self.assertTrue(
            any("missing parent MISSING" in issue for issue in graph["issues"])
        )
        self.assertTrue(any("cycle" in issue for issue in graph["issues"]))

    def test_duplicate_ids_are_preserved_and_links_are_ambiguous(self):
        graph = build_graph(
            [
                (
                    "test",
                    {
                        "requirements": [
                            requirement("A"),
                            requirement("A"),
                            requirement("CHILD", ["A"]),
                        ]
                    },
                )
            ]
        )
        self.assertEqual(len(graph["nodes"]), 3)
        self.assertEqual(graph["edges"], [])
        self.assertIsNone(graph["nodes"][2]["layer"])
        self.assertTrue(any("Duplicate ID A" in issue for issue in graph["issues"]))

    def test_invalid_document_fails_before_starting_server(self):
        with tempfile.TemporaryDirectory() as temporary:
            document = Path(temporary) / "invalid.json"
            document.write_text(
                json.dumps({"schemaVersion": "1.0", "requirements": []})
            )
            result = subprocess.run(
                [
                    sys.executable,
                    str(PACKAGE / "viewer.py"),
                    "--no-browser",
                    "--validator",
                    os.environ["REQUIREMENTS_VALIDATOR"],
                    str(document),
                ],
                capture_output=True,
                text=True,
                timeout=10,
            )
            self.assertEqual(result.returncode, 1)
            self.assertIn("FAIL:", result.stderr)
            self.assertNotIn("http://", result.stdout)
