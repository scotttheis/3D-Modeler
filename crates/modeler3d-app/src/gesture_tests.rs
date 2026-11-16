use super::*;
use crate::transform_gesture::*;
use crate::transforms::world_bounds;
use crate::{CommandId, CommandOutcome, CommandState};
use modeler3d_core::{PrimitiveKind, SceneObject, make_box};
use modeler3d_viewport::Ray;
use serde::Deserialize;

#[derive(Deserialize)]
struct Descriptor {
    #[serde(rename = "type")]
    kind: Option<String>,
    axis: Option<usize>,
    side: Option<f32>,
    sides: Option<[f32; 3]>,
}
#[derive(Deserialize)]
struct Case {
    kind: String,
    mode: String,
    handle: String,
    basis: bool,
    descriptor: Descriptor,
    before: Vec<[[f32; 4]; 4]>,
    after: Vec<[[f32; 4]; 4]>,
    start: [[f32; 3]; 2],
    updates: Vec<[[f32; 3]; 2]>,
}
fn ray(pair: [[f32; 3]; 2]) -> Ray {
    Ray::new(pair[0], pair[1]).expect("valid oracle ray")
}

fn assert_object_eq(actual: &SceneObject, expected: &SceneObject) {
    assert_eq!(actual.object_id(), expected.object_id());
    assert_eq!(actual.name(), expected.name());
    assert_eq!(actual.mesh(), expected.mesh());
    assert_eq!(actual.transform(), expected.transform());
    assert_eq!(actual.color(), expected.color());
    assert_eq!(actual.is_visible(), expected.is_visible());
    assert_eq!(actual.metadata(), expected.metadata());
    assert_eq!(
        format!("{:?}", actual.children()),
        format!("{:?}", expected.children())
    );
}
#[test]
fn all_python_handles_axes_bases_groups_and_hidden_targets_match_122_oracles() {
    let cases: Vec<Case> = serde_json::from_str(include_str!(
        "../../../evidence/rebuild/rb-04-scene-and-precise-transforms/gesture-oracle.json"
    ))
    .expect("oracle");
    assert_eq!(cases.len(), 122);
    for case in cases {
        let mut state = CommandState::default();
        let count = if case.kind == "single" { 1 } else { 2 };
        for i in 0..count {
            let size = if i == 0 {
                [20., 30., 40.]
            } else {
                [10., 20., 30.]
            };
            let position = if i == 0 {
                [7., 11., 13.]
            } else {
                [42., 21., 13.]
            };
            state
                .scene
                .add_object(SceneObject::new(make_box(size).expect("box")))
                .expect("add");
            state
                .scene
                .set_transform(i, Transform::identity().translated(position).expect("pose"))
                .expect("set");
            if i == 1 {
                state.scene.set_visible(i, false).expect("hide");
            }
        }
        state.scene.set_selection(0..count);
        if case.kind == "group" {
            state.scene.group_selected().expect("group");
        }
        for (i, rows) in case.before.iter().enumerate() {
            state
                .scene
                .set_transform(i, Transform::from_rows(*rows).expect("before"))
                .expect("pose");
        }
        state.scene.set_selection(0..case.before.len());
        let indices = (0..case.before.len()).collect::<Vec<_>>();
        let frame = ResizeFrame::from_scene(&state.scene, &indices, case.basis).expect("frame");
        assert_eq!(frame.handles().len(), 26);
        let center = world_bounds(&state.scene, &indices)
            .expect("bounds")
            .center();
        let snapshot = state.capture_transform_selection().expect("capture");
        let axis_index = "xyz".find(case.handle.as_str()).unwrap_or(0);
        let axis = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]][axis_index];
        let mut gesture = match case.mode.as_str() {
            "move" => TransformGesture::move_axis(snapshot, center, axis, ray(case.start))
                .expect("start move"),
            "rotate" => TransformGesture::rotate(snapshot, center, axis, ray(case.start))
                .expect("start rotate"),
            _ => {
                let handle = match case.descriptor.kind.as_deref() {
                    Some("face") => ResizeHandle::Face {
                        axis: case.descriptor.axis.expect("axis"),
                        side: case.descriptor.side.expect("side"),
                    },
                    Some("edge") => ResizeHandle::Edge {
                        axis: case.descriptor.axis.expect("axis"),
                        sides: case.descriptor.sides.expect("sides"),
                    },
                    _ => ResizeHandle::Corner(case.descriptor.sides.expect("sides")),
                };
                TransformGesture::resize(snapshot, frame, handle, ray(case.start))
                    .expect("start resize")
            }
        };
        let revision = state.scene.revision();
        let history = state.scene.history_state_id();
        for update in case.updates {
            gesture.update(ray(update)).expect("preview");
        }
        assert_eq!(revision, state.scene.revision());
        assert_eq!(history, state.scene.history_state_id());
        assert!(!state.is_dirty());
        assert!(matches!(
            state.commit_transform_snapshot(&gesture.snapshot, gesture.delta),
            Ok(CommandOutcome::Applied(_))
        ));
        for (object, expected) in state.scene.objects().iter().zip(&case.after) {
            for (a, b) in object
                .transform()
                .rows()
                .iter()
                .flatten()
                .zip(expected.iter().flatten())
            {
                assert!(
                    (a - b).abs() < 0.003,
                    "{} {} {} basis {} actual {:?} expected {:?}",
                    case.kind,
                    case.mode,
                    case.handle,
                    case.basis,
                    object.transform().rows(),
                    expected
                );
            }
        }
        state.dispatch(CommandId::Undo).expect("undo");
        for (object, before) in state.scene.objects().iter().zip(case.before) {
            assert_eq!(object.transform().rows(), before);
        }
        assert_eq!(state.scene.history_state_id(), history);
        state.dispatch(CommandId::Redo).expect("redo");
    }
}

#[test]
fn cancelled_noop_stale_and_replaced_document_keep_scene_safe() {
    let mut state = CommandState::default();
    state
        .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
        .expect("cube");
    state.dispatch(CommandId::Duplicate).expect("duplicate");
    state.scene.set_selection([0, 1]);
    let snapshot = state.capture_transform_selection().expect("capture");
    let history = state.scene.history_state_id();
    assert!(matches!(
        state.commit_transform_snapshot(&snapshot, Transform::identity()),
        Ok(CommandOutcome::Noop(_))
    ));
    assert_eq!(history, state.scene.history_state_id());
    state.scene.clear_selection();
    let rest = state.scene.objects()[0].position();
    let delta = Transform::identity()
        .translated([5., 0., 0.])
        .expect("delta");
    assert!(matches!(
        state.commit_transform_snapshot(&snapshot, delta),
        Ok(CommandOutcome::Applied(_))
    ));
    assert_eq!(
        state.scene.objects()[0].position(),
        [rest[0] + 5., rest[1], rest[2]]
    );
    assert!(state.commit_transform_snapshot(&snapshot, delta).is_err());
    let before = state.scene.objects()[0].transform().rows();
    assert_eq!(before, state.scene.objects()[0].transform().rows());
    state.scene.set_selection([0]);
    let old = state.capture_transform_selection().expect("snapshot");
    state.dispatch(CommandId::NewScene).expect("new");
    state
        .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
        .expect("cube");
    assert!(state.commit_transform_snapshot(&old, delta).is_err());
    assert_eq!(state.scene.objects()[0].position(), [0., 20., 0.]);
}

#[test]
fn parallel_and_zero_radius_rotation_rays_preserve_preview() {
    let snapshot = TransformSelectionSnapshot {
        revision: 0,
        document_generation: 0,
        selection: Default::default(),
        objects: vec![],
    };
    let start = Ray::new([1., 0., 10.], [0., 0., -1.]).expect("start");
    let mut drag =
        TransformGesture::rotate(snapshot, [0.; 3], [0., 0., 1.], start).expect("rotate");
    drag.update(Ray::new([1., 0., 10.], [1., 0., 0.]).expect("parallel"))
        .expect("safe");
    drag.update(Ray::new([0., 0., 10.], [0., 0., -1.]).expect("center"))
        .expect("safe");
    assert_eq!(drag.delta, Transform::identity());
}

#[test]
fn multi_numeric_matches_32_python_toolbar_axis_unit_and_lock_cases() {
    #[derive(Deserialize)]
    struct NumericCase {
        mode: String,
        before: Vec<[[f32; 4]; 4]>,
        target: [f32; 3],
        after: Vec<[[f32; 4]; 4]>,
    }
    let cases: Vec<NumericCase> = serde_json::from_str(include_str!(
        "../../../evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection-oracle.json"
    ))
    .expect("oracle");
    assert_eq!(cases.len(), 32);
    for case in cases {
        let mut state = CommandState::default();
        for (i, size) in [[20., 30., 40.], [10., 20., 30.]].into_iter().enumerate() {
            state
                .scene
                .add_object(SceneObject::new(make_box(size).expect("box")))
                .expect("add");
            state
                .scene
                .set_transform(i, Transform::from_rows(case.before[i]).expect("pose"))
                .expect("set");
        }
        state.scene.set_visible(1, false).expect("hide");
        state.scene.set_selection([1]);
        state.scene.set_selection([1, 0]);
        let ids = state
            .scene
            .objects()
            .iter()
            .map(|o| o.object_id().to_owned())
            .collect::<Vec<_>>();
        let history = state.scene.history_state_id();
        let outcome = match case.mode.as_str() {
            "move" => state.set_selected_position(case.target),
            "rotate" => state.set_selected_rotation(case.target),
            _ => state.set_selected_size(case.target),
        };
        assert!(
            matches!(outcome, Ok(CommandOutcome::Applied(_))),
            "{outcome:?}"
        );
        for (i, object) in state.scene.objects().iter().enumerate() {
            assert_eq!(object.object_id(), ids[i]);
            assert_eq!(object.is_visible(), i == 0);
            for (actual, expected) in object
                .transform()
                .rows()
                .iter()
                .flatten()
                .zip(case.after[i].iter().flatten())
            {
                assert!(
                    (actual - expected).abs() < 0.003,
                    "{} actual {:?} expected {:?}",
                    case.mode,
                    object.transform().rows(),
                    case.after[i]
                );
            }
        }
        state.dispatch(CommandId::Undo).expect("undo");
        assert_eq!(state.scene.history_state_id(), history);
        for (object, before) in state.scene.objects().iter().zip(&case.before) {
            assert_eq!(object.transform().rows(), *before);
        }
        state.dispatch(CommandId::Redo).expect("redo");
    }
}

#[test]
fn invalid_multi_numeric_keeps_all_targets_history_identity_and_visibility() {
    let mut state = CommandState::default();
    for _ in 0..2 {
        state
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
    }
    state.scene.set_selection([0, 1]);
    state.scene.set_visible(1, false).expect("hide");
    let before = state.scene.snapshot();
    let history = state.scene.history_state_id();
    assert!(state.set_selected_position([f32::NAN, 0., 0.]).is_err());
    assert!(
        state
            .set_selected_rotation([0., f32::INFINITY, 0.])
            .is_err()
    );
    for value in [0., -1., f32::NAN, f32::INFINITY] {
        assert!(state.set_selected_size([value, 20., 20.]).is_err());
    }
    assert_eq!(history, state.scene.history_state_id());
    for (a, b) in state.scene.objects().iter().zip(&before.objects) {
        assert_object_eq(a, b);
    }
}

#[test]
fn grouped_hidden_clipboard_duplicate_delete_and_undo_preserve_metadata_and_fresh_ids() {
    let mut state = CommandState::default();
    for _ in 0..2 {
        state
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
    }
    state.scene.set_visible(1, false).expect("hide child");
    state.scene.set_selection([0, 1]);
    state.dispatch(CommandId::Group).expect("group");
    state.rename_selected("Named group".into()).expect("name");
    let original = state.scene.objects()[0].clone();
    let before = state.scene.history_state_id();
    state.dispatch(CommandId::Copy).expect("copy");
    assert_eq!(before, state.scene.history_state_id());
    state.dispatch(CommandId::Paste).expect("paste");
    let pasted = state.scene.objects()[1].clone();
    assert_ne!(pasted.object_id(), original.object_id());
    assert_eq!(pasted.name(), "Named group (copy)");
    assert_eq!(pasted.mesh(), original.mesh());
    // OF-43 replaced the Python offsets: copies sit 10 mm beside the original.
    assert_eq!(pasted.position(), [50., 0., 0.]);
    for (a, b) in original.children().iter().zip(pasted.children()) {
        assert_ne!(a.object_id(), b.object_id());
        assert_eq!(a.mesh(), b.mesh());
        assert_eq!(a.is_visible(), b.is_visible());
        assert_eq!(a.name(), b.name());
        assert_eq!(a.color(), b.color());
    }
    state.dispatch(CommandId::Undo).expect("undo paste");
    assert_eq!(before, state.scene.history_state_id());
    state.dispatch(CommandId::Redo).expect("redo paste");
    assert_object_eq(&state.scene.objects()[1], &pasted);
    state.dispatch(CommandId::Cut).expect("cut");
    assert_eq!(state.scene.objects().len(), 1);
    assert_object_eq(&state.scene.objects()[0], &original);
    state.dispatch(CommandId::Undo).expect("undo cut");
    assert_object_eq(&state.scene.objects()[1], &pasted);
    state.dispatch(CommandId::Duplicate).expect("duplicate");
    let duplicate = state.scene.objects()[2].clone();
    assert_eq!(duplicate.position(), [100., 0., 0.]);
    assert_ne!(duplicate.object_id(), pasted.object_id());
    state.dispatch(CommandId::Delete).expect("delete");
    assert_eq!(state.scene.objects().len(), 2);
    state.dispatch(CommandId::Undo).expect("undo delete");
    assert_object_eq(&state.scene.objects()[2], &duplicate);
    state.dispatch(CommandId::Ungroup).expect("ungroup");
    assert_eq!(state.scene.objects().len(), 4);
    assert!(!state.scene.objects()[3].is_visible());
    state.dispatch(CommandId::Undo).expect("undo ungroup");
    assert_object_eq(&state.scene.objects()[2], &duplicate);
}

#[test]
fn placement_all_axes_landmarks_gaps_and_anchor_orders_match_42_python_cases() {
    #[derive(Deserialize)]
    struct PlacementCase {
        operation: String,
        axis: String,
        argument: serde_json::Value,
        order: Vec<usize>,
        before: Vec<[[f32; 4]; 4]>,
        after: Vec<[[f32; 4]; 4]>,
    }
    let cases: Vec<PlacementCase> = serde_json::from_str(include_str!(
        "../../../evidence/rebuild/rb-04-scene-and-precise-transforms/placement-oracle.json"
    ))
    .expect("oracle");
    assert_eq!(cases.len(), 42);
    for case in cases {
        let mut state = CommandState::default();
        for (i, size) in [[20., 30., 40.], [10., 20., 30.], [25., 15., 35.]]
            .into_iter()
            .enumerate()
        {
            state
                .scene
                .add_object(SceneObject::new(make_box(size).expect("box")))
                .expect("add");
            state
                .scene
                .set_transform(i, Transform::from_rows(case.before[i]).expect("pose"))
                .expect("set");
        }
        state.scene.set_visible(1, false).expect("hide");
        state.scene.clear_selection();
        state.scene.set_selection(case.order);
        let history = state.scene.history_state_id();
        let axis = match case.axis.as_str() {
            "x" => Axis::X,
            "y" => Axis::Y,
            _ => Axis::Z,
        };
        let command = match case.operation.as_str() {
            "mirror" => CommandId::Mirror,
            "align" => CommandId::Align,
            _ => CommandId::Arrange,
        };
        let landmark = match case.argument.as_str() {
            Some("min") => Alignment::Minimum,
            Some("max") => Alignment::Maximum,
            _ => Alignment::Center,
        };
        state
            .place_selected(
                command,
                axis,
                landmark,
                case.argument.as_f64().unwrap_or(0.) as f32,
            )
            .expect("place");
        for (object, expected) in state.scene.objects().iter().zip(&case.after) {
            for (a, b) in object
                .transform()
                .rows()
                .iter()
                .flatten()
                .zip(expected.iter().flatten())
            {
                assert!(
                    (a - b).abs() < 0.003,
                    "{} {} {:?} actual {:?} expected {:?}",
                    case.operation,
                    case.axis,
                    case.argument,
                    object.transform().rows(),
                    expected
                );
            }
        }
        state.dispatch(CommandId::Undo).expect("undo");
        assert_eq!(state.scene.history_state_id(), history);
        for (object, before) in state.scene.objects().iter().zip(case.before) {
            assert_eq!(object.transform().rows(), before);
        }
        state.dispatch(CommandId::Redo).expect("redo");
    }
}
