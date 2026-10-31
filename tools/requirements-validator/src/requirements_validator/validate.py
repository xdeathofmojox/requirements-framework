"""Validate requirement documents with an explicit, offline schema registry."""

import argparse
import json
import sys
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource
from referencing.exceptions import Unresolvable


def schema_context(schema_directory, project_schema=None, schema_version=None):
    if project_schema is not None and schema_version is not None:
        raise ValueError("select either a project schema or a schema version")
    paths = sorted(Path(schema_directory).rglob("*.schema.json"))
    if project_schema:
        project_path = Path(project_schema)
        if project_path.resolve() not in {path.resolve() for path in paths}:
            paths.append(project_path)
    registry = Registry()
    schemas = {}
    for path in paths:
        schema = json.loads(path.read_text())
        Draft202012Validator.check_schema(schema)
        uri = schema["$id"]
        if uri in schemas:
            raise ValueError(f"duplicate schema ID: {uri}")
        schemas[uri] = schema
        registry = registry.with_resource(uri, Resource.from_contents(schema))
    if project_schema:
        selected = json.loads(Path(project_schema).read_text())
    else:
        version = schema_version if schema_version is not None else "v1"
        uri = f"urn:requirements-framework:{version}:requirements"
        if uri not in schemas:
            raise ValueError(f"unsupported schema version: {version}")
        selected = schemas[uri]
    return selected, registry


def validator(schema_directory, project_schema=None, schema_version=None):
    selected, registry = schema_context(
        schema_directory, project_schema, schema_version
    )
    return Draft202012Validator(selected, registry=registry)


def validate_document(check, path):
    failed = False
    try:
        document = json.loads(path.read_text())
        for error in check.iter_errors(document):
            pointer = "/" + "/".join(
                str(part).replace("~", "~0").replace("/", "~1")
                for part in error.absolute_path
            )
            print(f"{path}:{pointer}: {error.message}", file=sys.stderr)
            failed = True
    except (OSError, ValueError, Unresolvable) as error:
        print(f"{path}: {error}", file=sys.stderr)
        failed = True
    print(f"{'FAIL' if failed else 'PASS'}: {path}", flush=True)
    return not failed


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--schema-dir", type=Path, required=True)
    selection = parser.add_mutually_exclusive_group()
    selection.add_argument(
        "--schema", type=Path, help="Project schema with an absolute $id"
    )
    selection.add_argument(
        "--schema-version", help="Bundled schema version, e.g. v1 (default: v1)"
    )
    parser.add_argument(
        "--describe",
        action="store_true",
        help="Print requirement form metadata as JSON",
    )
    parser.add_argument("documents", type=Path, nargs="*")
    args = parser.parse_args()
    if not args.describe and not args.documents:
        parser.error("provide at least one document, or use --describe")
    try:
        if args.describe:
            from .describe import describe_requirements

            selected, registry = schema_context(
                args.schema_dir, args.schema, args.schema_version
            )
            print(json.dumps(describe_requirements(selected, registry)))
            return 0
        check = validator(args.schema_dir, args.schema, args.schema_version)
        failed = False
        for path in args.documents:
            if not validate_document(check, path):
                failed = True
        return int(failed)
    except (OSError, ValueError, KeyError, Unresolvable) as error:
        print(f"FAIL: validation setup: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
