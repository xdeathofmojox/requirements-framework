use crate::{Document, Error};
use serde_json::{Value, json};
use std::collections::{BTreeMap, VecDeque};

fn requirements(document: &Value) -> Result<&Vec<Value>, Error> {
    document
        .get("requirements")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Invalid("Document must have a requirements array".into()))
}

/// Build a layered graph while preserving arbitrary project-specific fields.
pub fn build_graph(documents: &[Document]) -> Result<Value, Error> {
    let mut nodes = Vec::new();
    let mut by_id: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for document in documents {
        for requirement in requirements(&document.value)? {
            let id = requirement
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| Error::Invalid("Requirement ID must be a string".into()))?;
            let index = nodes.len();
            by_id.entry(id.into()).or_default().push(index);
            nodes.push(json!({"key":index.to_string(), "source":document.path,
                "documentTitle":document.value.get("title").cloned().unwrap_or_else(|| json!(document.path)),
                "data":requirement, "layer":null}));
        }
    }
    let mut issues = Vec::new();
    for (id, matches) in &by_id {
        if matches.len() > 1 {
            issues.push(format!(
                "Duplicate ID {id}: {} records; references are ambiguous.",
                matches.len()
            ));
        }
    }
    let mut children = vec![Vec::new(); nodes.len()];
    let mut remaining = vec![0; nodes.len()];
    let mut edges = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        let parents = node["data"]
            .get("parents")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        remaining[index] = parents.len();
        for parent in parents {
            let parent = parent
                .as_str()
                .ok_or_else(|| Error::Invalid("Parent IDs must be strings".into()))?;
            match by_id.get(parent).map(Vec::as_slice) {
                Some([ancestor]) => {
                    children[*ancestor].push(index);
                    edges.push(json!({"parent":ancestor.to_string(), "child":index.to_string()}));
                }
                matches => issues.push(format!(
                    "{}: {} parent {parent}.",
                    node["data"]["id"].as_str().unwrap_or("?"),
                    if matches.is_none() {
                        "missing"
                    } else {
                        "ambiguous"
                    }
                )),
            }
        }
    }
    let mut depths = vec![0; nodes.len()];
    let mut queue: VecDeque<_> = (0..nodes.len())
        .filter(|index| remaining[*index] == 0)
        .collect();
    while let Some(index) = queue.pop_front() {
        nodes[index]["layer"] = json!(depths[index]);
        for child in &children[index] {
            depths[*child] = depths[*child].max(depths[index] + 1);
            remaining[*child] -= 1;
            if remaining[*child] == 0 {
                queue.push_back(*child);
            }
        }
    }
    let unresolved: Vec<_> = nodes
        .iter()
        .filter(|node| node["layer"].is_null())
        .map(|node| node["data"]["id"].as_str().unwrap_or("?"))
        .collect();
    if !unresolved.is_empty() {
        issues.push(format!(
            "Unresolved layers (cycle, missing/ambiguous parent, or descendant): {}",
            unresolved.join(", ")
        ));
    }
    Ok(json!({"nodes":nodes, "edges":edges, "issues":issues}))
}
