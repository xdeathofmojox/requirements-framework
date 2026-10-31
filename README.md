# Requirements framework

JSON requirements, composable JSON Schema Draft 2020-12 schemas, and Nix tooling.
The schema layer is the first step toward requirements traceability and MC/DC
analysis. No Rust build is needed; the original template crates remain dormant.

## Write and validate requirements

See the [validator quick start](tools/schema-validation/README.md) for copyable commands.

See [the core example](schemas/examples/requirements.json). Each document has
`schemaVersion: "1.0"` and a nonempty `requirements` array. Each requirement needs
a nonblank `id`, `title`, and `statement`. Optional core fields are tags and
parent requirement IDs. The strict default adds an optional status (`draft`,
`approved`, or `retired`) and constrains IDs, including parent IDs, to start with
a letter and contain only letters, digits, underscores, periods, colons, and
hyphens. IDs are case sensitive. Tags and parent lists must contain unique values.

```sh
nix develop
requirements-validate schemas/examples/requirements.json
requirements-validate --schema-version v1 schemas/examples/requirements.json
requirements-validate --schema schemas/examples/project.schema.json schemas/examples/project-requirements.json
python -m unittest discover -s tools/schema-validation/tests -v
nix flake check
```

The validator returns 0 on success and 1 on invalid documents, prints failures
with document paths and JSON pointers, and resolves schemas offline. The
`$schema` field in a document is an editor hint; validation always uses the
framework's strict schema or the explicit `--schema` argument.

## Extend the schema

- [Core](schemas/v1/core.schema.json): open document and requirement definitions.
- [Strict default](schemas/v1/requirements.schema.json): closes both objects to catch typos.
- [Project schema](schemas/examples/project.schema.json): adds required owner and criticality fields.

Compose project schemas with `urn:requirements-framework:v1:core` and its
`#/$defs/requirement` definition. Apply `unevaluatedProperties: false` only at the
final composition boundary. Referencing the strict default would prevent adding
fields. Composition adds constraints; it cannot override or relax the core.
Core defines no ID format beyond a nonblank string and no status vocabulary.
Concrete schemas define ID patterns for both `id` and `parents`, and can add their
own status enumerations. The project example uses IDs such as `001/HEAT` and
statuses `proposed`, `accepted`, and `superseded`, independently of the default.
Parents are optional so root requirements can omit them or use an empty list.

Rationale and verification metadata are not part of the provided schemas.
Verification evidence and test traceability will be modeled externally, allowing
tests to reference requirement IDs without embedding test links in requirements.

Schema IDs are versioned URNs, not hosted endpoints. Register the files explicitly
in editor or third-party validators. The CLI registers all bundled schemas and
the selected project schema by `$id`; external schemas are never fetched.
Project helper schemas can be registered with `--schema-dir` pointing to a directory
containing the core files and any helpers named `*.schema.json`.

The schema validates structure only. It does not enforce ID uniqueness across
records, resolve parent/test links, detect cycles, or prove MC/DC coverage.
Those semantic checks and evidence models belong to subsequent tooling. Breaking
schema changes will use a new schema version and IDs.

Package definitions live beside their sources: `schemas/default.nix` and
`tools/schema-validation/default.nix`. The overlay composes them with `callPackage`.

## Schema and release versions

`version.nix` contains the framework bundle release version, independent of schema
contracts. A bundle can include multiple supported major schema versions. The
schema package automatically includes every `schemas/vN/` directory, preserving
paths and versioned IDs. V1 remains the default even when newer versions are added.

Select a bundled contract with `--schema-version v1`, or supply a project schema
with `--schema`; these options are mutually exclusive. Project schemas select
contracts through their versioned `$ref` IDs. Documents never select a schema
implicitly through `$schema` or `schemaVersion`. Unsupported versions fail explicitly.

To introduce a breaking contract, add `schemas/v2/` with new v2 IDs and references,
and an example at `schemas/examples/v2/requirements.json`. Preserve the v1 contract and
example. Tests validate an example for every bundled version and exercise version
selection with a synthetic future contract. No v2 contract is shipped yet.

## Consume with Nix

Add this repository as a flake input (named `requirements-framework` below).
Packages are `requirements-validator` and `requirements-schemas`; the default
package is the validator. Schemas install under
`share/requirements-framework/schemas/v1`.
Examples install under `share/requirements-framework/examples`. Validator tests
consume these packaged files through `REQUIREMENTS_SCHEMA_DATA`, which Nix sets
in the development shell and test check.

Use `inputs.requirements-framework.overlays.default` in a nixpkgs import to expose
`pkgs.requirements-validator` and `pkgs.requirements-schemas`. The overlay also
works directly via `import ./overlay.nix` and uses the consumer's nixpkgs.

`nix/modules` is discovered by `import-tree`. The requirements module there
configures this repository and exports `flakeModules.default` for downstream
projects; consumers import that output explicitly.

For flake-parts projects:

```nix
{
  imports = [ inputs.requirements-framework.flakeModules.default ];
  perSystem = { ... }: {
    requirements.documents = [ ./requirements.json ];
    # Optional: choose a bundled version (defaults to v1).
    # requirements.schemaVersion = "v1";
    # Or compose a domain schema; mutually exclusive with schemaVersion.
    requirements.schema = ./requirements.schema.json;
  };
}
```

This adds `checks.<system>.requirements` for `nix flake check`. Without documents,
no check is added. Commit schema and document files before evaluating a Git-based
flake, since Nix excludes untracked files. No overlay is required for the module.
