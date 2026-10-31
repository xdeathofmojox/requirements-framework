use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use requirements_viewer_rust::{Document, Store, Validation, router};
use serde_json::{Value, json};
use std::path::Path;
use tower::ServiceExt;

fn requirement(id: &str, parents: &[&str]) -> Value {
    json!({"id":id,"title":"A requirement","statement":"The system shall behave.","parents":parents})
}
fn validation() -> Validation {
    Validation {
        executable: std::env::var_os("REQUIREMENTS_VALIDATOR")
            .expect("Run tests with nix develop or nix build")
            .into(),
        schema: None,
        schema_version: Some("v1".into()),
    }
}
fn write_document(path: &Path, items: Vec<Value>) {
    std::fs::write(
        path,
        serde_json::to_vec_pretty(
            &json!({"schemaVersion":"1.0","title":"Test","requirements":items}),
        )
        .unwrap(),
    )
    .unwrap();
}
fn store(path: &Path, editable: bool) -> Store {
    Store::new(vec![Document::load(path).unwrap()], validation(), editable).unwrap()
}

#[test]
fn additions_validate_preserve_data_and_reject_stale_or_external_changes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("requirements.json");
    write_document(&path, vec![requirement("ROOT", &[])]);
    let mut session = store(&path, true);
    let result = session.add(0, 0, requirement("CHILD", &["ROOT"])).unwrap();
    assert_eq!(result["nodes"][1]["layer"], 1);
    assert_eq!(result["revision"], 1);
    let saved = std::fs::read(&path).unwrap();
    let doc: Value = serde_json::from_slice(&saved).unwrap();
    assert_eq!(doc["title"], "Test");
    assert_eq!(doc["requirements"].as_array().unwrap().len(), 2);
    assert!(session.add(0, 0, requirement("OTHER", &[])).is_err());
    assert!(session.add(0, 1, requirement("CHILD", &[])).is_err());
    let mut invalid = requirement("INVALID", &[]);
    invalid["statement"] = json!("");
    assert!(session.add(0, 1, invalid).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    std::fs::write(&path, b"external edit").unwrap();
    assert!(session.add(0, 1, requirement("OTHER", &[])).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"external edit");
}

#[test]
fn graph_handles_multiple_parents_cycles_duplicates_and_missing_links() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("requirements.json");
    write_document(
        &path,
        vec![
            requirement("ROOT", &[]),
            requirement("MID", &["ROOT"]),
            requirement("LEAF", &["ROOT", "MID"]),
            requirement("A", &["B"]),
            requirement("B", &["A"]),
            requirement("ORPHAN", &["MISSING"]),
            requirement("DUP", &[]),
            requirement("DUP", &[]),
            requirement("AMBIGUOUS", &["DUP"]),
        ],
    );
    let graph = store(&path, false).snapshot().unwrap();
    assert_eq!(graph["nodes"][2]["layer"], 2);
    for index in [3, 4, 5, 8] {
        assert!(graph["nodes"][index]["layer"].is_null());
    }
    assert!(graph["issues"].as_array().unwrap().len() >= 4);
}

#[test]
fn read_only_and_unknown_targets_do_not_write() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("requirements.json");
    write_document(&path, vec![requirement("ROOT", &[])]);
    let original = std::fs::read(&path).unwrap();
    assert!(
        store(&path, false)
            .add(0, 0, requirement("NEW", &[]))
            .is_err()
    );
    assert!(
        store(&path, true)
            .add(10, 0, requirement("NEW", &[]))
            .is_err()
    );
    assert_eq!(std::fs::read(&path).unwrap(), original);
}

#[tokio::test]
async fn api_requires_host_origin_and_session_token_and_saves_valid_additions() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("requirements.json");
    write_document(&path, vec![requirement("ROOT", &[])]);
    let app = router(store(&path, true), "127.0.0.1:8765".into());
    let bad_host = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/data")
                .header("host", "attacker.test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_host.status(), StatusCode::FORBIDDEN);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/data")
                .header("host", "127.0.0.1:8765")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let data: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let payload = json!({"document":0,"revision":0,"requirement":requirement("CHILD", &["ROOT"])});
    for (origin, token, status) in [
        (
            "http://evil.test",
            data["token"].as_str().unwrap(),
            StatusCode::FORBIDDEN,
        ),
        ("http://127.0.0.1:8765", "wrong", StatusCode::FORBIDDEN),
        (
            "http://127.0.0.1:8765",
            data["token"].as_str().unwrap(),
            StatusCode::OK,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/requirements")
                    .header("host", "127.0.0.1:8765")
                    .header("origin", origin)
                    .header("x-viewer-token", token)
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
    let saved: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(saved["requirements"][1]["id"], "CHILD");
}

#[test]
fn custom_schema_fields_are_required_preserved_and_displayed() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("requirements.json");
    let schema = directory.path().join("project.schema.json");
    let project = json!({
        "$schema":"https://json-schema.org/draft/2020-12/schema", "$id":"urn:test:viewer-project",
        "$ref":"urn:requirements-framework:v1:core",
        "properties":{"requirements":{"items":{
            "$ref":"urn:requirements-framework:v1:core#/$defs/requirement",
            "properties":{"owner":{"type":"object","required":["team"]}},
            "required":["owner"],"unevaluatedProperties":false
        }}},"unevaluatedProperties":false
    });
    std::fs::write(&schema, project.to_string()).unwrap();
    let mut root = requirement("001/ROOT", &[]);
    root["owner"] = json!({"team":"controls"});
    write_document(&path, vec![root]);
    let mut selected = validation();
    selected.schema = Some(schema);
    selected.schema_version = None;
    let mut session = Store::new(vec![Document::load(&path).unwrap()], selected, true).unwrap();
    let form = session.snapshot().unwrap()["formSchema"].clone();
    assert_eq!(form["fields"]["owner"]["required"], true);
    assert_eq!(form["additionalFields"], false);
    let original = std::fs::read(&path).unwrap();
    assert!(
        session
            .add(0, 0, requirement("002/CHILD", &["001/ROOT"]))
            .is_err()
    );
    assert_eq!(std::fs::read(&path).unwrap(), original);
    let mut child = requirement("002/CHILD", &["001/ROOT"]);
    child["owner"] = json!({"team":"safety","reviewers":["one","two"]});
    let graph = session.add(0, 0, child).unwrap();
    assert_eq!(
        graph["nodes"][1]["data"]["owner"]["reviewers"],
        json!(["one", "two"])
    );
    let saved: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved["requirements"][1]["owner"]["team"], "safety");
}

#[test]
fn folders_discover_documents_deduplicate_inputs_and_reload_atomically() {
    let directory = tempfile::tempdir().unwrap();
    let nested = directory.path().join("nested");
    std::fs::create_dir(&nested).unwrap();
    let root = directory.path().join("system.json");
    let child = nested.join("software.json");
    write_document(&root, vec![requirement("ROOT", &[])]);
    write_document(&child, vec![requirement("CHILD", &["ROOT"])]);
    std::fs::write(directory.path().join("config.json"), r#"{"setting":true}"#).unwrap();
    std::fs::write(
        directory.path().join("project.schema.json"),
        "not a document",
    )
    .unwrap();
    std::fs::create_dir(directory.path().join(".git")).unwrap();
    std::fs::write(directory.path().join(".git/ignore.json"), "invalid").unwrap();
    let mut session = Store::open(
        vec![directory.path().into(), child.clone()],
        validation(),
        true,
    )
    .unwrap();
    let initial = session.snapshot().unwrap();
    assert_eq!(initial["documents"].as_array().unwrap().len(), 2);
    let target = initial["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|doc| doc["path"].as_str().unwrap().ends_with("software.json"))
        .unwrap()["index"]
        .as_u64()
        .unwrap() as usize;
    session
        .add(target, 0, requirement("ADDED", &["CHILD"]))
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(&child).unwrap()).unwrap()["requirements"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    // The explicitly supplied child file remains required, so remove a folder-only file.
    std::fs::remove_file(&root).unwrap();
    let added = nested.join("new.json");
    write_document(&added, vec![requirement("NEW", &[])]);
    let before_reload = session.snapshot().unwrap();
    assert_eq!(before_reload["nodes"].as_array().unwrap().len(), 3);
    let reloaded = session.reload().unwrap();
    assert_eq!(reloaded["revision"], 2);
    assert!(
        reloaded["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["data"]["id"] == "NEW")
    );
    assert!(
        !reloaded["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["data"]["id"] == "ROOT")
    );
    write_document(&added, vec![requirement("CHANGED", &[])]);
    assert_eq!(session.snapshot().unwrap(), reloaded);
    let changed = session.reload().unwrap();
    assert_eq!(changed["revision"], 3);
    std::fs::write(&added, "{").unwrap();
    assert!(session.reload().is_err());
    assert_eq!(session.snapshot().unwrap(), changed);
    write_document(&added, vec![json!({"id":"INVALID"})]);
    assert!(session.reload().is_err());
    assert_eq!(session.snapshot().unwrap(), changed);
    write_document(&added, vec![requirement("RECOVERED", &[])]);
    let recovered = session.reload().unwrap();
    assert_eq!(recovered["revision"], 4);
    assert!(session.add(0, 3, requirement("STALE", &[])).is_err());
    assert_eq!(session.reload().unwrap()["revision"], 4);
    let target = recovered["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|doc| doc["path"].as_str().unwrap().ends_with("new.json"))
        .unwrap()["index"]
        .as_u64()
        .unwrap() as usize;
    session
        .add(target, 4, requirement("NEWLY-DISCOVERED-EDIT", &[]))
        .unwrap();
    let saved: Value = serde_json::from_slice(&std::fs::read(&added).unwrap()).unwrap();
    assert_eq!(saved["requirements"][1]["id"], "NEWLY-DISCOVERED-EDIT");
}

#[test]
fn empty_folders_pick_up_new_documents_only_on_reload() {
    let directory = tempfile::tempdir().unwrap();
    let mut session = Store::open(vec![directory.path().into()], validation(), false).unwrap();
    assert_eq!(session.snapshot().unwrap()["nodes"], json!([]));
    write_document(
        &directory.path().join("new.json"),
        vec![requirement("NEW", &[])],
    );
    assert_eq!(session.snapshot().unwrap()["nodes"], json!([]));
    assert_eq!(session.reload().unwrap()["nodes"][0]["data"]["id"], "NEW");
}

#[cfg(unix)]
#[test]
fn discovery_does_not_follow_nested_symlinks() {
    let directory = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    write_document(
        &outside.path().join("external.json"),
        vec![requirement("EXTERNAL", &[])],
    );
    std::os::unix::fs::symlink(outside.path(), directory.path().join("linked-folder")).unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("external.json"),
        directory.path().join("linked.json"),
    )
    .unwrap();
    let session = Store::open(vec![directory.path().into()], validation(), true).unwrap();
    assert_eq!(session.snapshot().unwrap()["documents"], json!([]));
}

#[tokio::test]
async fn get_data_reuses_snapshot_until_an_authorized_reload() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("requirements.json");
    write_document(&path, vec![requirement("ROOT", &[])]);
    let app = router(
        Store::open(vec![directory.path().into()], validation(), true).unwrap(),
        "127.0.0.1:8765".into(),
    );
    let fetch = || {
        Request::builder()
            .uri("/data")
            .header("host", "127.0.0.1:8765")
            .body(Body::empty())
            .unwrap()
    };
    let response = app.clone().oneshot(fetch()).await.unwrap();
    let original: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    write_document(&path, vec![requirement("UPDATED", &[])]);
    let response = app.clone().oneshot(fetch()).await.unwrap();
    let cached: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(cached, original);
    for (token, status) in [
        ("invalid", StatusCode::FORBIDDEN),
        (original["token"].as_str().unwrap(), StatusCode::OK),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/reload")
                    .header("host", "127.0.0.1:8765")
                    .header("origin", "http://127.0.0.1:8765")
                    .header("x-viewer-token", token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        if status == StatusCode::OK {
            let refreshed: Value =
                serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                    .unwrap();
            assert_eq!(refreshed["nodes"][0]["data"]["id"], "UPDATED");
            assert_eq!(refreshed["revision"], 1);
        }
    }
}

#[test]
fn empty_folder_can_create_first_document_and_append_to_it() {
    let directory = tempfile::tempdir().unwrap();
    let mut session = Store::open(vec![directory.path().into()], validation(), true).unwrap();
    assert_eq!(
        session.snapshot().unwrap()["folders"][0]["path"],
        directory.path().to_str().unwrap()
    );
    let snapshot = session
        .create(
            0,
            "first.json",
            0,
            json!({"schemaVersion":"1.0","title":"First document"}),
            requirement("ROOT", &[]),
        )
        .unwrap();
    assert_eq!(snapshot["nodes"][0]["data"]["id"], "ROOT");
    assert_eq!(snapshot["revision"], 1);
    let path = directory.path().join("first.json");
    let saved: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved["title"], "First document");
    session.add(0, 1, requirement("CHILD", &["ROOT"])).unwrap();
    assert_eq!(
        session.reload().unwrap()["nodes"].as_array().unwrap().len(),
        2
    );
}

#[test]
fn new_documents_validate_and_cannot_escape_or_overwrite() {
    let directory = tempfile::tempdir().unwrap();
    let mut session = Store::open(vec![directory.path().into()], validation(), true).unwrap();
    for filename in [
        "../outside.json",
        "/tmp/outside.json",
        "nested/../outside.json",
        "nested/../../outside.json",
        "nested/.hidden/file.json",
        "nested/.hidden.json",
        "nested//file.json",
        "nested/./file.json",
        "C:/outside.json",
        "nested\\file.json",
        ".hidden.json",
        "file.schema.json",
        "bad.txt",
        "",
    ] {
        assert!(
            session
                .create(
                    0,
                    filename,
                    0,
                    json!({"schemaVersion":"1.0"}),
                    requirement("ROOT", &[])
                )
                .is_err()
        );
    }
    assert!(
        session
            .create(
                0,
                "invalid.json",
                0,
                json!({"schemaVersion":"1.0"}),
                json!({"id":"INVALID"})
            )
            .is_err()
    );
    assert!(!directory.path().join("invalid.json").exists());
    assert!(
        session
            .create(0, "invalid.json", 0, json!([]), requirement("ROOT", &[]))
            .is_err()
    );
    let existing = directory.path().join("existing.json");
    std::fs::write(&existing, b"keep me").unwrap();
    assert!(
        session
            .create(
                0,
                "existing.json",
                0,
                json!({"schemaVersion":"1.0"}),
                requirement("ROOT", &[])
            )
            .is_err()
    );
    assert_eq!(std::fs::read(&existing).unwrap(), b"keep me");
    assert!(
        session
            .create(
                10,
                "unknown.json",
                0,
                json!({"schemaVersion":"1.0"}),
                requirement("ROOT", &[])
            )
            .is_err()
    );
    let readonly_directory = tempfile::tempdir().unwrap();
    let mut readonly =
        Store::open(vec![readonly_directory.path().into()], validation(), false).unwrap();
    assert!(
        readonly
            .create(
                0,
                "readonly.json",
                0,
                json!({"schemaVersion":"1.0"}),
                requirement("ROOT", &[])
            )
            .is_err()
    );
}

#[test]
fn new_documents_create_nested_folders_and_reload() {
    let directory = tempfile::tempdir().unwrap();
    let mut session = Store::open(vec![directory.path().into()], validation(), true).unwrap();
    assert!(
        session
            .create(
                0,
                "invalid/nested/file.json",
                0,
                json!({"schemaVersion":"1.0"}),
                json!({"id":"INVALID"})
            )
            .is_err()
    );
    assert!(!directory.path().join("invalid").exists());
    let snapshot = session
        .create(
            0,
            "system/controls/requirements.json",
            0,
            json!({"schemaVersion":"1.0"}),
            requirement("ROOT", &[]),
        )
        .unwrap();
    assert_eq!(snapshot["revision"], 1);
    let path = directory.path().join("system/controls/requirements.json");
    assert!(path.is_file());
    session
        .create(
            0,
            "system/controls/other.json",
            1,
            json!({"schemaVersion":"1.0"}),
            requirement("CHILD", &["ROOT"]),
        )
        .unwrap();
    assert_eq!(
        session.reload().unwrap()["nodes"].as_array().unwrap().len(),
        2
    );
    let original = std::fs::read(&path).unwrap();
    assert!(
        session
            .create(
                0,
                "system/controls/requirements.json",
                2,
                json!({"schemaVersion":"1.0"}),
                requirement("ANOTHER", &[])
            )
            .is_err()
    );
    assert_eq!(std::fs::read(path).unwrap(), original);
}

#[cfg(unix)]
#[test]
fn new_documents_reject_symlink_folders_and_targets() {
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let mut session = Store::open(vec![directory.path().into()], validation(), true).unwrap();
    std::fs::create_dir(directory.path().join("nested")).unwrap();
    symlink(outside.path(), directory.path().join("nested/link")).unwrap();
    symlink(
        outside.path().join("missing"),
        directory.path().join("dangling"),
    )
    .unwrap();
    symlink(
        outside.path().join("missing.json"),
        directory.path().join("target.json"),
    )
    .unwrap();
    std::fs::write(directory.path().join("ordinary-file"), b"keep me").unwrap();
    for filename in [
        "nested/link/new/deep.json",
        "dangling/file.json",
        "target.json",
        "ordinary-file/file.json",
    ] {
        assert!(
            session
                .create(
                    0,
                    filename,
                    0,
                    json!({"schemaVersion":"1.0"}),
                    requirement("ROOT", &[])
                )
                .is_err(),
            "accepted {filename}"
        );
    }
    assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
    assert_eq!(session.snapshot().unwrap()["revision"], 0);
}

#[tokio::test]
async fn api_creates_first_document_in_an_opened_folder() {
    let directory = tempfile::tempdir().unwrap();
    let app = router(
        Store::open(vec![directory.path().into()], validation(), true).unwrap(),
        "127.0.0.1:8765".into(),
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/data")
                .header("host", "127.0.0.1:8765")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let data: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let payload = json!({"folder":0,"filename":"first.json","revision":0,"document":{"schemaVersion":"1.0"},"requirement":requirement("ROOT", &[])});
    for (token, status) in [
        ("invalid", StatusCode::FORBIDDEN),
        (data["token"].as_str().unwrap(), StatusCode::OK),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/documents")
                    .header("host", "127.0.0.1:8765")
                    .header("origin", "http://127.0.0.1:8765")
                    .header("x-viewer-token", token)
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
    assert!(directory.path().join("first.json").is_file());
}

#[test]
fn edits_replace_only_selected_record_and_reject_invalid_or_stale_changes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("requirements.json");
    write_document(
        &path,
        vec![requirement("ROOT", &[]), requirement("CHILD", &["ROOT"])],
    );
    let mut session = store(&path, true);
    let mut changed = requirement("CHILD", &[]);
    changed["title"] = json!("Edited title");
    changed["tags"] = json!(["reviewed"]);
    let result = session.update("1", 0, changed.clone()).unwrap();
    assert_eq!(result["nodes"][1]["data"], changed);
    assert_eq!(result["nodes"][1]["layer"], 0);
    let saved = std::fs::read(&path).unwrap();
    let document: Value = serde_json::from_slice(&saved).unwrap();
    assert_eq!(document["requirements"][0], requirement("ROOT", &[]));
    assert_eq!(document["title"], "Test");
    assert_eq!(document["requirements"].as_array().unwrap().len(), 2);
    assert!(session.update("1", 0, changed.clone()).is_err());
    assert!(session.update("unknown", 1, changed.clone()).is_err());
    assert!(session.update("99", 1, changed.clone()).is_err());
    let mut invalid = changed.clone();
    invalid["statement"] = json!("");
    assert!(session.update("1", 1, invalid).is_err());
    let mut renamed = changed.clone();
    renamed["id"] = json!("RENAMED");
    assert!(session.update("1", 1, renamed).is_err());
    assert!(store(&path, false).update("1", 0, changed.clone()).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    std::fs::write(&path, b"external edit").unwrap();
    assert!(session.update("1", 1, changed).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"external edit");
}

#[tokio::test]
async fn api_edits_an_existing_record_with_session_guards() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("requirements.json");
    write_document(&path, vec![requirement("ROOT", &[])]);
    let app = router(store(&path, true), "127.0.0.1:8765".into());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/data")
                .header("host", "127.0.0.1:8765")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let data: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let mut updated = requirement("ROOT", &[]);
    updated["statement"] = json!("The system shall support edits.");
    let payload = json!({"key":"0","revision":0,"requirement":updated});
    for (token, status) in [
        ("invalid", StatusCode::FORBIDDEN),
        (data["token"].as_str().unwrap(), StatusCode::OK),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/requirements")
                    .header("host", "127.0.0.1:8765")
                    .header("origin", "http://127.0.0.1:8765")
                    .header("x-viewer-token", token)
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
    let saved: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        saved["requirements"][0]["statement"],
        "The system shall support edits."
    );
}

#[test]
fn deletion_requires_confirmation_and_preserves_validation_and_conflicts() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("requirements.json");
    write_document(
        &path,
        vec![requirement("ROOT", &[]), requirement("CHILD", &["ROOT"])],
    );
    let mut session = store(&path, true);
    let original = std::fs::read(&path).unwrap();
    assert!(session.delete("1", 0, "WRONG").is_err());
    assert!(session.delete("1", 1, "CHILD").is_err());
    assert!(session.delete("99", 0, "CHILD").is_err());
    assert!(session.delete("invalid", 0, "CHILD").is_err());
    assert!(store(&path, false).delete("1", 0, "CHILD").is_err());
    assert!(
        session
            .delete("0", 0, "ROOT")
            .unwrap_err()
            .to_string()
            .contains("CHILD")
    );
    assert_eq!(std::fs::read(&path).unwrap(), original);
    let graph = session.delete("1", 0, "CHILD").unwrap();
    assert_eq!(graph["revision"], 1);
    assert_eq!(graph["nodes"].as_array().unwrap().len(), 1);
    let saved = std::fs::read(&path).unwrap();
    let document: Value = serde_json::from_slice(&saved).unwrap();
    assert_eq!(document["title"], "Test");
    assert_eq!(document["requirements"], json!([requirement("ROOT", &[])]));
    std::fs::write(&path, b"external edit").unwrap();
    assert!(session.delete("0", 1, "ROOT").is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"external edit");
    std::fs::write(&path, &saved).unwrap();
    let empty = session.delete("0", 1, "ROOT").unwrap();
    assert!(!path.exists());
    assert_eq!(empty["documents"], json!([]));
    assert_eq!(empty["nodes"], json!([]));
    assert_eq!(empty["revision"], 2);
    assert_eq!(session.reload().unwrap()["nodes"], json!([]));
}

#[test]
fn deletion_checks_parent_references_across_files() {
    let directory = tempfile::tempdir().unwrap();
    let parent = directory.path().join("parent.json");
    let child = directory.path().join("child.json");
    write_document(
        &parent,
        vec![requirement("ROOT", &[]), requirement("OTHER", &[])],
    );
    write_document(&child, vec![requirement("CHILD", &["ROOT"])]);
    let mut session = Store::open(vec![directory.path().into()], validation(), true).unwrap();
    let snapshot = session.snapshot().unwrap();
    let key = snapshot["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["data"]["id"] == "ROOT")
        .unwrap()["key"]
        .as_str()
        .unwrap();
    assert!(
        session
            .delete(key, 0, "ROOT")
            .unwrap_err()
            .to_string()
            .contains("CHILD")
    );
}

#[tokio::test]
async fn api_deletes_only_after_matching_confirmation_and_session_guards() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("requirements.json");
    write_document(
        &path,
        vec![requirement("ROOT", &[]), requirement("CHILD", &["ROOT"])],
    );
    let app = router(store(&path, true), "127.0.0.1:8765".into());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/data")
                .header("host", "127.0.0.1:8765")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let data: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let token = data["token"].as_str().unwrap();
    let original = std::fs::read(&path).unwrap();
    for (provided_token, confirmation, status) in [
        ("wrong", "CHILD", StatusCode::FORBIDDEN),
        (token, "ROOT", StatusCode::UNPROCESSABLE_ENTITY),
        (token, "CHILD", StatusCode::OK),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/requirements")
                    .header("host", "127.0.0.1:8765")
                    .header("origin", "http://127.0.0.1:8765")
                    .header("x-viewer-token", provided_token)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"key":"1","revision":0,"confirm_id":confirmation}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        if status != StatusCode::OK {
            assert_eq!(std::fs::read(&path).unwrap(), original);
        }
    }
    let saved: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved["requirements"], json!([requirement("ROOT", &[])]));
}

#[test]
fn deleting_last_record_in_folder_removes_file_and_allows_starting_another() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("requirements.json");
    write_document(&path, vec![requirement("ROOT", &[])]);
    let mut session = Store::open(
        vec![directory.path().into(), path.clone()],
        validation(),
        true,
    )
    .unwrap();
    let empty = session.delete("0", 0, "ROOT").unwrap();
    assert!(!path.exists());
    assert_eq!(empty["documents"], json!([]));
    assert_eq!(empty["folders"].as_array().unwrap().len(), 1);
    assert_eq!(session.reload().unwrap()["nodes"], json!([]));
    let created = session
        .create(
            0,
            "new.json",
            1,
            json!({"schemaVersion":"1.0"}),
            requirement("NEW", &[]),
        )
        .unwrap();
    assert_eq!(created["nodes"][0]["data"]["id"], "NEW");
    assert!(directory.path().join("new.json").exists());
}

#[test]
fn explicit_reload_refreshes_schema_form_metadata_without_document_changes() {
    let directory = tempfile::tempdir().unwrap();
    let schema = directory.path().join("project.schema.json");
    let mut definition = json!({
        "$schema":"https://json-schema.org/draft/2020-12/schema", "$id":"urn:test:reload-form",
        "$ref":"urn:requirements-framework:v1:core",
        "properties":{"requirements":{"items":{"properties":{"owner":{"type":"string"}}}}}
    });
    std::fs::write(&schema, definition.to_string()).unwrap();
    let mut policy = validation();
    policy.schema = Some(schema.clone());
    policy.schema_version = None;
    let mut session = Store::open(vec![directory.path().into()], policy, true).unwrap();
    assert!(
        session.snapshot().unwrap()["formSchema"]["fields"]
            .get("owner")
            .is_some()
    );
    definition["properties"]["requirements"]["items"]["properties"]["criticality"] =
        json!({"enum":["low","high"]});
    std::fs::write(&schema, definition.to_string()).unwrap();
    assert!(
        session.snapshot().unwrap()["formSchema"]["fields"]
            .get("criticality")
            .is_none()
    );
    let refreshed = session.reload().unwrap();
    assert_eq!(
        refreshed["formSchema"]["fields"]["criticality"]["schema"]["enum"],
        json!(["low", "high"])
    );
    assert_eq!(refreshed["revision"], 1);
}
