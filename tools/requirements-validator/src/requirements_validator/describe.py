"""Collect form annotations; the JSON Schema validator remains authoritative."""

from referencing import Resource
from referencing.jsonschema import DRAFT202012


def parts(schema, resolver, seen=frozenset()):
    if not isinstance(schema, dict):
        return
    resolver = resolver.in_subresource(
        Resource.from_contents(schema, default_specification=DRAFT202012)
    )
    reference = schema.get("$ref")
    if reference:
        resolved = resolver.lookup(reference)
        marker = id(resolved.contents)
        if marker not in seen:
            yield from parts(resolved.contents, resolved.resolver, seen | {marker})
    yield schema, resolver
    for child in schema.get("allOf", []):
        yield from parts(child, resolver, seen)


def descend(contexts, key):
    result = []
    for schema, resolver in contexts:
        child = schema.get(key)
        if isinstance(child, dict):
            result.extend(parts(child, resolver))
    return result


def property_contexts(contexts, name):
    result = []
    for schema, resolver in contexts:
        child = schema.get("properties", {}).get(name)
        if isinstance(child, dict):
            result.extend(parts(child, resolver))
    return result


def field_schema(schema, resolver, depth=0):
    # Resolve field references for controls, without recursively expanding object trees.
    if depth > 8:
        return {}
    fragments = []
    for part, context in parts(schema, resolver):
        fragment = {
            key: value
            for key, value in part.items()
            if key not in ("$ref", "$defs", "allOf")
        }
        if isinstance(fragment.get("items"), dict):
            fragment["items"] = field_schema(fragment["items"], context, depth + 1)
        if fragment:
            fragments.append(fragment)
    if len(fragments) == 1:
        return fragments[0]
    return {"allOf": fragments}


def describe_requirements(schema, registry):
    resolver = registry.resolver_with_root(Resource.from_contents(schema))
    root = list(parts(schema, resolver))
    requirements = property_contexts(root, "requirements")
    contexts = descend(requirements, "items")
    fields = {}
    required = set()
    closed = False
    patterned = False
    for part, context in contexts:
        required.update(part.get("required", []))
        closed |= (
            part.get("unevaluatedProperties") is False
            or part.get("additionalProperties") is False
        )
        patterned |= bool(part.get("patternProperties"))
        for name, definition in part.get("properties", {}).items():
            fields.setdefault(name, []).append(
                field_schema(definition, context)
                if isinstance(definition, dict)
                else definition
            )

    # Conditional fields are shown as optional controls. Their applicability
    # and branch-specific requirements are checked on save, not guessed here.
    def branches(part, context, depth=0):
        nonlocal patterned
        if depth > 8:
            return
        for keyword in ("oneOf", "anyOf", "then", "else", "dependentSchemas"):
            children = part.get(keyword, [])
            if isinstance(children, dict):
                children = (
                    list(children.values())
                    if keyword == "dependentSchemas"
                    else [children]
                )
            for child in children:
                for branch, branch_context in parts(child, context):
                    patterned |= bool(branch.get("patternProperties"))
                    for name, definition in branch.get("properties", {}).items():
                        if name not in fields:
                            fields[name] = [
                                field_schema(definition, branch_context)
                                if isinstance(definition, dict)
                                else definition
                            ]
                    branches(branch, branch_context, depth + 1)

    for part, context in contexts:
        branches(part, context)
    return {
        "fields": {
            name: {
                "schema": definitions[0]
                if len(definitions) == 1
                else {"allOf": definitions},
                "required": name in required,
            }
            for name, definitions in fields.items()
        },
        "additionalFields": not closed or patterned,
    }
