# Schema validation

Check JSON requirement files against a bundled schema or your project's schema.
The Nix package includes Python and the bundled schemas.

Run these examples from the framework repository root:

```sh
nix develop
requirements-validate schemas/examples/requirements.json
```

Or run directly without entering the development shell:

```sh
nix run .#requirements-validator -- schemas/examples/requirements.json
```

Validate several files in one command:

```sh
requirements-validate requirements/system.json requirements/software.json
```

Replace those paths with your own files. V1 is the default schema. To select it
explicitly:

```sh
requirements-validate --schema-version v1 schemas/examples/requirements.json
```

To use a project schema instead:

```sh
requirements-validate --schema schemas/examples/project.schema.json schemas/examples/project-requirements.json
```

Use either `--schema` or `--schema-version`. The document's `$schema` field does
not select the validation schema. Schemas resolve locally; validation does not
download them.

Each file prints a result:

```text
PASS: schemas/examples/requirements.json
FAIL: requirements/invalid.json
```

Failures also include error messages identifying the file and field. The command
checks every file, including when one cannot be read or contains invalid JSON.
It exits with code 0 when all files pass, or 1 when validation fails. This makes
it usable in scripts and checks:

```sh
requirements-validate schemas/examples/requirements.json && echo "Requirements are valid"
```

Validation checks document structure and field constraints. Parent link resolution
and test coverage are outside its scope.

For downstream Nix setup and schema extension guidance, see the
[framework README](../../README.md).

To run the tests from the framework repository root:

```sh
nix develop --command python -m unittest discover -s tools/schema-validation/tests -v
```

The development shell supplies `REQUIREMENTS_SCHEMA_DATA`, pointing to the schemas
and examples installed by the Nix schema package. Tests use that package rather
than sibling directories in the checkout. `nix flake check` runs the same tests
with only the validator sources and its packaged dependencies available.

The viewer can ask the validator for form metadata (without a document):

```sh
requirements-validate --describe
requirements-validate --schema schemas/examples/project.schema.json --describe
```

This returns declared requirement fields and whether additional properties are
allowed. It resolves local references and `allOf` composition; conditional fields
are listed as optional controls, with their applicability checked on save.
