import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SOURCE = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(SOURCE))
os.environ["PYTHONPATH"] = os.pathsep.join(
    filter(None, [str(SOURCE), os.environ.get("PYTHONPATH")])
)
from requirements_validator.describe import describe_requirements
from requirements_validator.validate import schema_context

DATA = Path(os.environ["REQUIREMENTS_SCHEMA_DATA"])


def describe(project=None):
    schema, registry = schema_context(DATA / "schemas", project)
    return describe_requirements(schema, registry)


class DescriptionTests(unittest.TestCase):
    def test_strict_schema_inherits_core_and_adds_status(self):
        result = describe()
        self.assertFalse(result["additionalFields"])
        self.assertTrue(result["fields"]["title"]["required"])
        self.assertFalse(result["fields"]["tags"]["required"])
        self.assertEqual(
            result["fields"]["status"]["schema"]["enum"],
            ["draft", "approved", "retired"],
        )

    def test_project_fields_and_local_refs_are_described(self):
        result = describe(DATA / "examples/project.schema.json")
        self.assertTrue(result["fields"]["owner"]["required"])
        self.assertTrue(result["fields"]["criticality"]["required"])
        self.assertEqual(
            result["fields"]["criticality"]["schema"]["enum"], ["low", "high"]
        )
        self.assertFalse(result["additionalFields"])
        self.assertIn("^[0-9]{3}/[A-Z]+$", json.dumps(result))

    def test_open_schema_allows_extras_and_allof_fields_are_collected(self):
        with tempfile.TemporaryDirectory() as directory:
            project = Path(directory) / "project.schema.json"
            project.write_text(
                json.dumps(
                    {
                        "$schema": "https://json-schema.org/draft/2020-12/schema",
                        "$id": "urn:test:form",
                        "allOf": [
                            {"$ref": "urn:requirements-framework:v1:core"},
                            {
                                "properties": {
                                    "requirements": {
                                        "items": {
                                            "properties": {
                                                "priority": {"type": "integer"},
                                                "safety": {"type": "boolean"},
                                                "metadata": {"type": "object"},
                                            },
                                            "required": ["priority"],
                                            "if": {
                                                "properties": {
                                                    "safety": {"const": True}
                                                }
                                            },
                                            "then": {
                                                "properties": {
                                                    "reviewer": {"type": "string"}
                                                },
                                                "required": ["reviewer"],
                                            },
                                        }
                                    }
                                }
                            },
                        ],
                    }
                )
            )
            result = describe(project)
            self.assertTrue(result["additionalFields"])
            self.assertTrue(result["fields"]["priority"]["required"])
            self.assertEqual(result["fields"]["safety"]["schema"]["type"], "boolean")
            self.assertIn("reviewer", result["fields"])
            self.assertFalse(result["fields"]["reviewer"]["required"])

    def test_cli_describe_needs_no_document_and_returns_json(self):
        result = subprocess.run(
            [
                sys.executable,
                "-m",
                "requirements_validator",
                "--schema-dir",
                str(DATA / "schemas"),
                "--describe",
            ],
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("status", json.loads(result.stdout)["fields"])
