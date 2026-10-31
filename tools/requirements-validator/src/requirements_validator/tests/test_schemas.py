import copy
import os
import json
import unittest
import sys
import shutil
import subprocess
import tempfile
from pathlib import Path

SOURCE = Path(__file__).resolve().parents[2]
if "REQUIREMENTS_SCHEMA_DATA" not in os.environ:
    raise RuntimeError("Run tests in nix develop or through nix flake check")
SCHEMA_DATA = Path(os.environ["REQUIREMENTS_SCHEMA_DATA"])
SCHEMAS = SCHEMA_DATA / "schemas"
EXAMPLES = SCHEMA_DATA / "examples"
sys.path.insert(0, str(SOURCE))
os.environ["PYTHONPATH"] = os.pathsep.join(
    filter(None, [str(SOURCE), os.environ.get("PYTHONPATH")])
)

from requirements_validator.validate import validator


class SchemaTests(unittest.TestCase):
    def setUp(self):
        self.check = validator(SCHEMAS)
        self.document = json.loads((EXAMPLES / "requirements.json").read_text())

    def test_core_example(self):
        self.check.validate(self.document)

    def test_rejects_invalid_core_fields(self):
        for field, value in [
            ("id", "bad id"),
            ("statement", "  "),
            ("title", "  "),
            ("status", "unknown"),
            ("parents", ["bad id"]),
            ("typo", True),
        ]:
            with self.subTest(field=field):
                document = copy.deepcopy(self.document)
                document["requirements"][0][field] = value
                self.assertFalse(self.check.is_valid(document))

    def test_required_fields_and_version(self):
        for field in ["id", "title", "statement"]:
            document = copy.deepcopy(self.document)
            del document["requirements"][0][field]
            self.assertFalse(self.check.is_valid(document))
        self.document["schemaVersion"] = "2.0"
        self.assertFalse(self.check.is_valid(self.document))

    def test_unknown_document_fields_rejected(self):
        self.document["typo"] = True
        self.assertFalse(self.check.is_valid(self.document))

    def test_project_extension_and_core_constraints(self):
        check = validator(SCHEMAS, EXAMPLES / "project.schema.json")
        document = json.loads((EXAMPLES / "project-requirements.json").read_text())
        check.validate(document)
        self.assertFalse(self.check.is_valid(document))
        for field, value in [
            ("statement", ""),
            ("criticality", "unknown"),
            ("id", "HEAT-001"),
            ("parents", ["HEAT-001"]),
            ("status", "draft"),
            ("typo", 1),
        ]:
            invalid = copy.deepcopy(document)
            invalid["requirements"][0][field] = value
            self.assertFalse(check.is_valid(invalid))
        del document["requirements"][0]["owner"]
        self.assertFalse(check.is_valid(document))

    def test_core_leaves_id_policy_and_status_to_consumers(self):
        core = self.check.evolve(schema={"$ref": "urn:requirements-framework:v1:core"})
        self.document["requirements"][0].update(
            id="001/HEAT", parents=["002/SYSTEM"], status="project-specific"
        )
        core.validate(self.document)
        for field, value in [
            ("id", " "),
            ("parents", [" "]),
            ("title", ""),
            ("tags", [1]),
        ]:
            invalid = copy.deepcopy(self.document)
            invalid["requirements"][0][field] = value
            self.assertFalse(core.is_valid(invalid))

    def test_parent_ids_follow_each_concrete_schema(self):
        self.document["requirements"][0]["parents"] = ["SYSTEM-001"]
        self.check.validate(self.document)
        project = validator(SCHEMAS, EXAMPLES / "project.schema.json")
        document = json.loads((EXAMPLES / "project-requirements.json").read_text())
        document["requirements"][0]["parents"] = ["002/SYSTEM"]
        project.validate(document)

    def test_removed_fields_are_rejected_by_default_schema(self):
        for field in ["rationale", "verification"]:
            with self.subTest(field=field):
                invalid = copy.deepcopy(self.document)
                invalid["requirements"][0][field] = "legacy data"
                self.assertFalse(self.check.is_valid(invalid))

    def test_empty_collection_rejected(self):
        self.document["requirements"] = []
        self.assertFalse(self.check.is_valid(self.document))


class VersionSelectionTests(unittest.TestCase):
    def test_all_bundled_versions_have_a_compatible_example(self):
        for directory in sorted((SCHEMAS).glob("v[0-9]*")):
            version = directory.name
            example = (
                EXAMPLES / "requirements.json"
                if version == "v1"
                else EXAMPLES / version / "requirements.json"
            )
            with self.subTest(version=version):
                document = json.loads(example.read_text())
                validator(SCHEMAS, schema_version=version).validate(document)

    def test_multiple_versions_coexist_without_changing_v1(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            shutil.copytree(SCHEMAS / "v1", directory / "v1")
            # A synthetic future contract exercises selection without shipping v2.
            future = {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "$id": "urn:requirements-framework:v2:requirements",
                "type": "object",
                "required": ["schemaVersion"],
                "properties": {"schemaVersion": {"const": "2.0"}},
            }
            (directory / "v2").mkdir()
            (directory / "v2/requirements.schema.json").write_text(json.dumps(future))
            old = json.loads((EXAMPLES / "requirements.json").read_text())
            validator(directory).validate(old)
            validator(directory, schema_version="v1").validate(old)
            future_check = validator(directory, schema_version="v2")
            future_check.validate({"schemaVersion": "2.0"})
            self.assertFalse(future_check.is_valid(old))
            self.assertFalse(validator(directory).is_valid({"schemaVersion": "2.0"}))

    def test_unknown_version_has_a_clear_error(self):
        with self.assertRaisesRegex(ValueError, "unsupported schema version: v999"):
            validator(SCHEMAS, schema_version="v999")

    def test_project_schema_and_version_are_mutually_exclusive(self):
        with self.assertRaisesRegex(
            ValueError, "either a project schema or a schema version"
        ):
            validator(SCHEMAS, EXAMPLES / "project.schema.json", "v1")

    def test_cli_version_selection(self):
        command = [
            sys.executable,
            "-m",
            "requirements_validator",
            "--schema-dir",
            str(SCHEMAS),
        ]
        document = str(EXAMPLES / "requirements.json")
        success = subprocess.run(
            command + ["--schema-version", "v1", document],
            capture_output=True,
            text=True,
        )
        self.assertEqual(success.returncode, 0, success.stderr)
        self.assertIn(f"PASS: {document}", success.stdout)
        failure = subprocess.run(
            command + ["--schema-version", "v999", document],
            capture_output=True,
            text=True,
        )
        self.assertEqual(failure.returncode, 1)
        self.assertIn("unsupported schema version: v999", failure.stderr)
        self.assertIn("FAIL: validation setup:", failure.stderr)


class ValidationOutputTests(unittest.TestCase):
    def test_reports_each_file_and_continues_after_failures(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            invalid = directory / "invalid.json"
            invalid.write_text('{"schemaVersion": "1.0", "requirements": []}')
            malformed = directory / "malformed.json"
            malformed.write_text("{")
            missing = directory / "missing.json"
            valid = EXAMPLES / "requirements.json"
            result = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "requirements_validator",
                    "--schema-dir",
                    str(SCHEMAS),
                    str(invalid),
                    str(malformed),
                    str(missing),
                    str(valid),
                ],
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 1)
            self.assertEqual(
                result.stdout.splitlines(),
                [
                    f"FAIL: {invalid}",
                    f"FAIL: {malformed}",
                    f"FAIL: {missing}",
                    f"PASS: {valid}",
                ],
            )
            self.assertIn(f"{invalid}:/requirements:", result.stderr)
            self.assertIn(str(malformed), result.stderr)
            self.assertIn(str(missing), result.stderr)
