use modeler3d_core::{PrimitiveKind, Scene, load_project_bytes, save_project_bytes};

#[test]
fn isolated_scene_copy_preserves_an_untouched_source_archive() {
    let mut source = Scene::new();
    source
        .add_primitive(PrimitiveKind::Cube, None, Some(String::from("Cube")))
        .expect("add source cube");
    let source_bytes = save_project_bytes(&source).expect("serialize source project");
    let loaded = load_project_bytes(&source_bytes).expect("load source project");

    let isolated = loaded.isolated_copy();

    assert_eq!(isolated.source_archive_bytes_len(), source_bytes.len());
    assert_eq!(
        save_project_bytes(&isolated).expect("save isolated project"),
        source_bytes
    );
}

#[test]
fn isolated_scene_result_commits_as_one_undoable_history_entry() {
    let mut live = Scene::new();
    let (cube_id, _) = live
        .add_primitive(PrimitiveKind::Cube, None, Some(String::from("Cube")))
        .expect("add live cube");
    live.set_selection([0]);
    let project_id = live.project_id().to_owned();
    let source_revision = live.revision();
    let source_history_id = live.history_state_id();

    let mut isolated = Scene::new();
    isolated
        .restore_snapshot(live.snapshot())
        .expect("restore isolated input");
    let (sphere_id, _) = isolated
        .add_primitive(PrimitiveKind::Sphere, None, Some(String::from("Sphere")))
        .expect("add isolated sphere");
    isolated.clear_selection();
    isolated.set_selection([1, 0]);
    let result = live
        .commit_snapshot_from_operation(&project_id, source_revision, isolated.snapshot())
        .expect("commit isolated result");

    assert!(result.changed);
    assert_eq!(result.revision, source_revision + 1);
    assert_eq!(live.revision(), source_revision + 1);
    assert_eq!(live.history_state_id(), source_history_id + 1);
    assert_eq!(live.objects().len(), 2);
    assert_eq!(live.objects()[0].object_id(), cube_id);
    assert_eq!(live.objects()[1].object_id(), sphere_id);
    assert_eq!(live.ordered_selection(), [1, 0]);

    let undone = live.undo().expect("undo result").expect("undo command");
    assert_eq!(undone.history_state_id, source_history_id);
    assert_eq!(live.objects().len(), 1);
    assert_eq!(live.objects()[0].object_id(), cube_id);
    assert_eq!(live.ordered_selection(), [0]);

    let redone = live.redo().expect("redo result").expect("redo command");
    assert_eq!(redone.history_state_id, source_history_id + 1);
    assert_eq!(live.objects().len(), 2);
    assert_eq!(live.objects()[1].object_id(), sphere_id);
    assert_eq!(live.ordered_selection(), [1, 0]);
}

#[test]
fn isolated_scene_result_rejects_a_stale_revision_without_mutation() {
    let mut live = Scene::new();
    live.add_primitive(PrimitiveKind::Cube, None, None)
        .expect("add live cube");
    let project_id = live.project_id().to_owned();
    let source_revision = live.revision();

    let mut isolated = Scene::new();
    isolated
        .restore_snapshot(live.snapshot())
        .expect("restore isolated input");
    isolated
        .add_primitive(PrimitiveKind::Sphere, None, None)
        .expect("add isolated sphere");
    let stale_result = isolated.snapshot();
    live.add_primitive(PrimitiveKind::Cylinder, None, None)
        .expect("intervening live edit");
    let objects_before = live.objects().len();
    let revision_before = live.revision();
    let history_before = live.history_state_id();

    let result = live.commit_snapshot_from_operation(&project_id, source_revision, stale_result);
    assert!(result.is_err());
    assert_eq!(live.objects().len(), objects_before);
    assert_eq!(live.revision(), revision_before);
    assert_eq!(live.history_state_id(), history_before);
}
