"""Build a trace graph while preserving every requirement's original fields."""

from collections import defaultdict, deque


def build_graph(documents):
    nodes = []
    by_id = defaultdict(list)
    for source, document in documents:
        for requirement in document["requirements"]:
            node = {
                "key": str(len(nodes)),
                "source": source,
                "documentTitle": document.get("title", source),
                "data": requirement,
                "layer": None,
            }
            nodes.append(node)
            by_id[requirement["id"]].append(node)

    issues = []
    children = defaultdict(list)
    remaining = {}
    edges = []
    for identifier, matches in by_id.items():
        if len(matches) > 1:
            issues.append(
                f"Duplicate ID {identifier}: {len(matches)} records; references to it are ambiguous."
            )
    for node in nodes:
        parents = node["data"].get("parents", [])
        remaining[node["key"]] = len(parents)
        for parent_id in parents:
            matches = by_id.get(parent_id, [])
            if len(matches) != 1:
                reason = "missing" if not matches else "ambiguous"
                issues.append(f"{node['data']['id']}: {reason} parent {parent_id}.")
                continue
            parent = matches[0]
            children[parent["key"]].append(node)
            edges.append({"parent": parent["key"], "child": node["key"]})

    queue = deque(node for node in nodes if remaining[node["key"]] == 0)
    depths = defaultdict(int)
    while queue:
        node = queue.popleft()
        node["layer"] = depths[node["key"]]
        for child in children[node["key"]]:
            key = child["key"]
            depths[key] = max(depths[key], node["layer"] + 1)
            remaining[key] -= 1
            if remaining[key] == 0:
                queue.append(child)
    unresolved = [node["data"]["id"] for node in nodes if node["layer"] is None]
    if unresolved:
        issues.append(
            "Unresolved layers (cycle, missing/ambiguous parent, or a descendant of one): "
            + ", ".join(unresolved)
        )
    return {"nodes": nodes, "edges": edges, "issues": issues}
