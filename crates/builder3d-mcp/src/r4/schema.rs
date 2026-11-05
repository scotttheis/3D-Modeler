//! Strict, bounded JSON schemas and the matching small validator used by R4.
//!
//! The validator intentionally implements only the JSON-Schema vocabulary the
//! catalog emits.  Keeping validation beside schema construction prevents a
//! tool from publishing one contract while accepting a wider runtime shape.

use serde_json::{Map, Value, json};

type JsonObject = Map<String, Value>;

const MAX_STRING: usize = 16 * 1024;
const MAX_ARRAY: usize = 1000;

fn object(properties: Map<String, Value>, required: &[&str]) -> JsonObject {
    let mut schema = Map::new();
    schema.insert("type".to_owned(), Value::String("object".to_owned()));
    schema.insert("properties".to_owned(), Value::Object(properties));
    schema.insert("additionalProperties".to_owned(), Value::Bool(false));
    if !required.is_empty() {
        schema.insert(
            "required".to_owned(),
            Value::Array(
                required
                    .iter()
                    .map(|name| Value::String((*name).to_owned()))
                    .collect(),
            ),
        );
    }
    schema
}

fn property(kind: &str) -> Value {
    json!({"type": kind})
}

fn string(minimum: usize, maximum: usize) -> Value {
    json!({"type":"string","minLength":minimum,"maxLength":maximum})
}

fn enum_string(values: &[&str]) -> Value {
    json!({"type":"string","enum":values})
}

fn number(minimum: Option<f64>, maximum: Option<f64>) -> Value {
    let mut value = Map::new();
    value.insert("type".to_owned(), Value::String("number".to_owned()));
    if let Some(minimum) = minimum {
        value.insert("minimum".to_owned(), json!(minimum));
    }
    if let Some(maximum) = maximum {
        value.insert("maximum".to_owned(), json!(maximum));
    }
    value.insert("finite".to_owned(), Value::Bool(true));
    Value::Object(value)
}

fn integer(minimum: Option<i64>, maximum: Option<i64>) -> Value {
    let mut value = Map::new();
    value.insert("type".to_owned(), Value::String("integer".to_owned()));
    if let Some(minimum) = minimum {
        value.insert("minimum".to_owned(), json!(minimum));
    }
    if let Some(maximum) = maximum {
        value.insert("maximum".to_owned(), json!(maximum));
    }
    Value::Object(value)
}

fn array(items: Value, minimum: usize, maximum: usize) -> Value {
    json!({"type":"array","items":items,"minItems":minimum,"maxItems":maximum})
}

fn vec3() -> Value {
    array(number(None, None), 3, 3)
}

fn positive_vec3() -> Value {
    array(number(Some(0.000_001), None), 3, 3)
}

fn color() -> Value {
    array(number(Some(0.0), Some(1.0)), 3, 4)
}

fn optional(schema: Value, default: Option<Value>) -> Value {
    let mut value = Map::new();
    value.insert(
        "anyOf".to_owned(),
        Value::Array(vec![schema, json!({"type":"null"})]),
    );
    if let Some(default) = default {
        value.insert("default".to_owned(), default);
    }
    Value::Object(value)
}

fn common_revision(properties: &mut Map<String, Value>, include_detail: bool) {
    properties.insert(
        "expected_scene_revision".to_owned(),
        optional(integer(Some(0), None), None),
    );
    properties.insert(
        "expected_project_id".to_owned(),
        optional(string(1, 128), None),
    );
    properties.insert("idempotency_key".to_owned(), optional(string(1, 128), None));
    if include_detail {
        properties.insert(
            "return_detail".to_owned(),
            optional(
                enum_string(&["none", "changes", "summary"]),
                Some(json!("summary")),
            ),
        );
    }
}

fn common_project(properties: &mut Map<String, Value>) {
    properties.insert(
        "expected_project_id".to_owned(),
        optional(string(1, 128), None),
    );
    properties.insert("idempotency_key".to_owned(), optional(string(1, 128), None));
}

fn target_properties(properties: &mut Map<String, Value>) {
    properties.insert(
        "object_id".to_owned(),
        optional(string(0, 128), Some(json!(""))),
    );
    properties.insert("index".to_owned(), optional(integer(Some(0), None), None));
}

fn indices_property() -> Value {
    optional(array(integer(Some(0), Some(250_000)), 0, 256), None)
}

fn object_ids_property() -> Value {
    optional(array(string(1, 128), 0, 256), None)
}

fn centers_property() -> Value {
    array(vec3(), 1, 256)
}

fn transform_selection_schema() -> JsonObject {
    let mut properties = Map::new();
    properties.insert("translation_mm".to_owned(), optional(vec3(), None));
    properties.insert("rotation_delta_deg".to_owned(), optional(vec3(), None));
    properties.insert("scale_factor".to_owned(), optional(positive_vec3(), None));
    properties.insert("pivot_mm".to_owned(), optional(vec3(), None));
    properties.insert("object_ids".to_owned(), object_ids_property());
    properties.insert("indices".to_owned(), indices_property());
    common_revision(&mut properties, true);
    object(properties, &[])
}

fn apply_actions_schema() -> JsonObject {
    let mut translate = Map::new();
    translate.insert("type".to_owned(), Value::String("object".to_owned()));
    translate.insert(
        "properties".to_owned(),
        json!({
            "action": {"const":"translate"},
            "offset_mm": vec3(),
            "object_id": string(0,128),
            "index": integer(Some(0),None)
        }),
    );
    translate.insert("required".to_owned(), json!(["action", "offset_mm"]));
    translate.insert("additionalProperties".to_owned(), Value::Bool(false));

    let mut set_color = Map::new();
    set_color.insert("type".to_owned(), Value::String("object".to_owned()));
    set_color.insert(
        "properties".to_owned(),
        json!({
            "action": {"const":"set_color"},
            "color": color(),
            "object_id": string(0,128),
            "index": integer(Some(0),None)
        }),
    );
    set_color.insert("required".to_owned(), json!(["action", "color"]));
    set_color.insert("additionalProperties".to_owned(), Value::Bool(false));

    let item = json!({
        "oneOf": [Value::Object(translate), Value::Object(set_color)],
        "discriminator": {"propertyName":"action"}
    });
    let mut properties = Map::new();
    properties.insert("actions".to_owned(), array(item, 1, 100));
    common_revision(&mut properties, true);
    object(properties, &["actions"])
}

/// Returns the strict input schema for one catalog entry.
#[must_use]
pub fn input_schema(name: &str) -> JsonObject {
    let mut properties = Map::new();
    let required: &[&str] = match name {
        "get_status" | "view_state" | "discover_slicers" => &[],
        "undo" | "redo" | "delete_selected" | "copy_selected" | "cut_selected" | "paste"
        | "duplicate_selected" | "group_selected" | "ungroup_selected" => {
            common_revision(&mut properties, false);
            &[]
        }
        "operation_status" | "cancel_operation" => {
            properties.insert("operation_id".to_owned(), string(1, 128));
            &["operation_id"]
        }
        "scene_summary" => {
            properties.insert(
                "fields".to_owned(),
                optional(array(string(1, 128), 0, 64), None),
            );
            &[]
        }
        "print_check" => {
            properties.insert(
                "bed_size_mm".to_owned(),
                optional(number(Some(0.000_001), Some(10_000.0)), Some(json!(200.0))),
            );
            &[]
        }
        "scene_changes" => {
            properties.insert("since_revision".to_owned(), integer(Some(0), None));
            properties.insert(
                "fields".to_owned(),
                optional(array(string(1, 128), 0, 64), None),
            );
            &["since_revision"]
        }
        "apply_actions" => return apply_actions_schema(),
        "add_primitive" => {
            properties.insert(
                "primitive".to_owned(),
                enum_string(&[
                    "cube",
                    "cylinder",
                    "cone",
                    "sphere",
                    "pyramid",
                    "torus",
                    "hexagon",
                    "wedge",
                    "tetrahedron",
                ]),
            );
            properties.insert("name".to_owned(), optional(string(0, 256), Some(json!(""))));
            properties.insert("dimensions_mm".to_owned(), optional(positive_vec3(), None));
            common_revision(&mut properties, false);
            &["primitive"]
        }
        "set_torus_parameters" => {
            target_properties(&mut properties);
            properties.insert("object_id".to_owned(), optional(string(1, 128), None));
            properties.insert(
                "mode".to_owned(),
                optional(enum_string(&["solid", "hole"]), None),
            );
            properties.insert(
                "radius_mm".to_owned(),
                optional(number(Some(0.5), Some(1000.0)), None),
            );
            properties.insert(
                "tube_mm".to_owned(),
                optional(number(Some(0.1), Some(500.0)), None),
            );
            properties.insert(
                "sides".to_owned(),
                optional(integer(Some(3), Some(256)), None),
            );
            properties.insert(
                "steps".to_owned(),
                optional(integer(Some(3), Some(256)), None),
            );
            common_revision(&mut properties, true);
            &[]
        }
        "elastic_grab" => {
            target_properties(&mut properties);
            properties.insert("center_mm".to_owned(), vec3());
            properties.insert("offset_mm".to_owned(), vec3());
            properties.insert(
                "radius_mm".to_owned(),
                optional(number(Some(0.1), Some(1000.0)), Some(json!(10.0))),
            );
            properties.insert(
                "strength".to_owned(),
                optional(number(Some(0.0), Some(1.0)), Some(json!(0.75))),
            );
            properties.insert(
                "falloff".to_owned(),
                optional(
                    enum_string(&["smooth", "linear", "constant"]),
                    Some(json!("smooth")),
                ),
            );
            properties.insert(
                "symmetry".to_owned(),
                optional(
                    enum_string(&["none", "x", "y", "z", "xy", "xz", "yz", "xyz"]),
                    Some(json!("none")),
                ),
            );
            common_revision(&mut properties, true);
            &["center_mm", "offset_mm"]
        }
        "crease_polish" => {
            target_properties(&mut properties);
            properties.insert("centers_mm".to_owned(), centers_property());
            properties.insert(
                "radius_mm".to_owned(),
                optional(number(Some(0.1), Some(1000.0)), Some(json!(5.0))),
            );
            properties.insert(
                "strength".to_owned(),
                optional(number(Some(0.0), Some(1.0)), Some(json!(0.5))),
            );
            properties.insert(
                "direction".to_owned(),
                optional(enum_string(&["inward", "outward"]), Some(json!("inward"))),
            );
            properties.insert(
                "falloff".to_owned(),
                optional(
                    enum_string(&["smooth", "linear", "constant"]),
                    Some(json!("smooth")),
                ),
            );
            common_revision(&mut properties, true);
            &["centers_mm"]
        }
        "smooth_brush" => {
            target_properties(&mut properties);
            properties.insert("centers_mm".to_owned(), centers_property());
            properties.insert(
                "radius_mm".to_owned(),
                optional(number(Some(0.1), Some(1000.0)), Some(json!(8.0))),
            );
            properties.insert(
                "strength".to_owned(),
                optional(number(Some(0.0), Some(1.0)), Some(json!(0.5))),
            );
            properties.insert(
                "shrinkage".to_owned(),
                optional(number(Some(0.0), Some(1.0)), Some(json!(0.25))),
            );
            properties.insert(
                "falloff".to_owned(),
                optional(
                    enum_string(&["smooth", "linear", "constant"]),
                    Some(json!("smooth")),
                ),
            );
            common_revision(&mut properties, true);
            &["centers_mm"]
        }
        "blob" => {
            target_properties(&mut properties);
            properties.insert("centers_mm".to_owned(), centers_property());
            properties.insert(
                "radius_mm".to_owned(),
                optional(number(Some(0.1), Some(1000.0)), Some(json!(8.0))),
            );
            properties.insert(
                "strength".to_owned(),
                optional(number(Some(0.0), Some(1.0)), Some(json!(0.5))),
            );
            properties.insert(
                "direction".to_owned(),
                optional(enum_string(&["outward", "inward"]), Some(json!("outward"))),
            );
            properties.insert(
                "falloff".to_owned(),
                optional(
                    enum_string(&["smooth", "linear", "constant"]),
                    Some(json!("smooth")),
                ),
            );
            common_revision(&mut properties, true);
            &["centers_mm"]
        }
        "normalize_face_density" => {
            target_properties(&mut properties);
            properties.insert(
                "face_indices".to_owned(),
                optional(array(integer(Some(0), Some(250_000)), 1, 250_000), None),
            );
            properties.insert("centers_mm".to_owned(), optional(centers_property(), None));
            properties.insert(
                "radius_mm".to_owned(),
                optional(number(Some(0.1), Some(1000.0)), Some(json!(8.0))),
            );
            properties.insert(
                "target_edge_length_mm".to_owned(),
                optional(number(Some(0.01), Some(1000.0)), None),
            );
            properties.insert(
                "max_iterations".to_owned(),
                optional(integer(Some(1), Some(4)), Some(json!(4))),
            );
            common_revision(&mut properties, true);
            &[]
        }
        "select" => {
            properties.insert("indices".to_owned(), indices_property());
            properties.insert("object_ids".to_owned(), object_ids_property());
            common_revision(&mut properties, false);
            &[]
        }
        "set_snapping" => {
            properties.insert("enabled".to_owned(), property("boolean"));
            &["enabled"]
        }
        "translate" => {
            properties.insert("offset_mm".to_owned(), vec3());
            target_properties(&mut properties);
            common_revision(&mut properties, true);
            &["offset_mm"]
        }
        "set_transform" => {
            target_properties(&mut properties);
            properties.insert("position_mm".to_owned(), optional(vec3(), None));
            properties.insert("rotation_deg".to_owned(), optional(vec3(), None));
            properties.insert("size_mm".to_owned(), optional(positive_vec3(), None));
            common_revision(&mut properties, true);
            &[]
        }
        "transform_selection" => return transform_selection_schema(),
        "snap_translate" => {
            properties.insert("axis".to_owned(), enum_string(&["x", "y", "z"]));
            properties.insert("proposed_delta_mm".to_owned(), number(None, None));
            properties.insert("indices".to_owned(), indices_property());
            properties.insert("object_ids".to_owned(), object_ids_property());
            properties.insert(
                "tolerance_mm".to_owned(),
                optional(number(Some(0.0), Some(100.0)), Some(json!(4.0))),
            );
            properties.insert(
                "tolerance_mode".to_owned(),
                optional(
                    enum_string(&["fixed_mm", "view_scaled", "floor_fixed"]),
                    Some(json!("fixed_mm")),
                ),
            );
            properties.insert(
                "targets".to_owned(),
                optional(
                    enum_string(&["objects_and_floor", "objects", "floor"]),
                    Some(json!("objects_and_floor")),
                ),
            );
            common_revision(&mut properties, true);
            &["axis", "proposed_delta_mm"]
        }
        "settle" | "mirror" => {
            target_properties(&mut properties);
            if name == "mirror" {
                properties.insert("axis".to_owned(), enum_string(&["x", "y", "z"]));
            }
            common_revision(&mut properties, true);
            if name == "mirror" { &["axis"] } else { &[] }
        }
        "align" => {
            properties.insert("axis".to_owned(), enum_string(&["x", "y", "z"]));
            properties.insert(
                "mode".to_owned(),
                optional(
                    enum_string(&["min", "center", "max"]),
                    Some(json!("center")),
                ),
            );
            properties.insert("indices".to_owned(), indices_property());
            properties.insert("object_ids".to_owned(), object_ids_property());
            common_revision(&mut properties, true);
            &["axis"]
        }
        "arrange" => {
            properties.insert("axis".to_owned(), enum_string(&["x", "y", "z"]));
            properties.insert(
                "gap_mm".to_owned(),
                optional(number(Some(0.0), None), Some(json!(0.0))),
            );
            properties.insert("indices".to_owned(), indices_property());
            properties.insert("object_ids".to_owned(), object_ids_property());
            common_revision(&mut properties, true);
            &["axis"]
        }
        "set_color" => {
            properties.insert("color".to_owned(), color());
            target_properties(&mut properties);
            common_revision(&mut properties, true);
            &["color"]
        }
        "sample_image_color" => {
            properties.insert("path".to_owned(), string(1, 4096));
            common_revision(&mut properties, false);
            &["path"]
        }
        "mesh_health" => {
            properties.insert(
                "index".to_owned(),
                optional(integer(Some(-1), None), Some(json!(-1))),
            );
            properties.insert(
                "object_id".to_owned(),
                optional(string(0, 128), Some(json!(""))),
            );
            &[]
        }
        "repair" => {
            target_properties(&mut properties);
            properties.insert(
                "profile".to_owned(),
                optional(
                    enum_string(&["simple", "extended"]),
                    Some(json!("extended")),
                ),
            );
            common_revision(&mut properties, true);
            &[]
        }
        "simplify" => {
            target_properties(&mut properties);
            properties.insert(
                "face_count".to_owned(),
                optional(integer(Some(1), Some(10_000_000)), None),
            );
            common_revision(&mut properties, true);
            &[]
        }
        "subdivide" => {
            target_properties(&mut properties);
            common_revision(&mut properties, true);
            &[]
        }
        "smooth" => {
            target_properties(&mut properties);
            properties.insert(
                "iterations".to_owned(),
                optional(integer(Some(1), Some(10)), Some(json!(1))),
            );
            common_revision(&mut properties, true);
            &[]
        }
        "import_file" => {
            properties.insert("path".to_owned(), string(1, 4096));
            properties.insert(
                "unit".to_owned(),
                optional(
                    enum_string(&["um", "mm", "cm", "in", "ft", "m"]),
                    Some(json!("mm")),
                ),
            );
            common_revision(&mut properties, false);
            &["path"]
        }
        "export_file" => {
            properties.insert("path".to_owned(), string(1, 4096));
            properties.insert(
                "overwrite".to_owned(),
                optional(property("boolean"), Some(json!(false))),
            );
            common_revision(&mut properties, false);
            &["path"]
        }
        "new_project" => {
            common_revision(&mut properties, false);
            &[]
        }
        "open_project" => {
            properties.insert("path".to_owned(), string(1, 4096));
            common_revision(&mut properties, false);
            &["path"]
        }
        "save_project" | "save_project_as" => {
            properties.insert("path".to_owned(), optional(string(1, 4096), None));
            properties.insert(
                "overwrite".to_owned(),
                optional(property("boolean"), Some(json!(false))),
            );
            common_project(&mut properties);
            if name == "save_project_as" {
                &["path"]
            } else {
                &[]
            }
        }
        "set_view" => {
            properties.insert(
                "yaw_deg".to_owned(),
                optional(number(Some(-360_000.0), Some(360_000.0)), None),
            );
            properties.insert(
                "pitch_deg".to_owned(),
                optional(number(Some(-89.0), Some(89.0)), None),
            );
            properties.insert(
                "distance_mm".to_owned(),
                optional(number(Some(0.5), Some(5000.0)), None),
            );
            properties.insert("center_mm".to_owned(), optional(vec3(), None));
            properties.insert("show_grid".to_owned(), optional(property("boolean"), None));
            properties.insert("wireframe".to_owned(), optional(property("boolean"), None));
            properties.insert("xray".to_owned(), optional(property("boolean"), None));
            common_project(&mut properties);
            &[]
        }
        "frame_view" => {
            properties.insert(
                "target".to_owned(),
                optional(enum_string(&["all", "selected"]), Some(json!("all"))),
            );
            common_project(&mut properties);
            &[]
        }
        "orbit_view" => {
            properties.insert(
                "yaw_delta_deg".to_owned(),
                number(Some(-360_000.0), Some(360_000.0)),
            );
            properties.insert(
                "pitch_delta_deg".to_owned(),
                number(Some(-360_000.0), Some(360_000.0)),
            );
            common_project(&mut properties);
            &["yaw_delta_deg", "pitch_delta_deg"]
        }
        "pan_view" => {
            properties.insert("offset_mm".to_owned(), vec3());
            common_project(&mut properties);
            &["offset_mm"]
        }
        "zoom_view" => {
            properties.insert("factor".to_owned(), number(Some(0.01), Some(100.0)));
            common_project(&mut properties);
            &["factor"]
        }
        "view_preset" => {
            properties.insert(
                "preset".to_owned(),
                enum_string(&["reset", "top", "front", "side"]),
            );
            common_project(&mut properties);
            &["preset"]
        }
        "boolean_merge" | "boolean_subtract" | "boolean_intersect" => {
            properties.insert("target_object_id".to_owned(), string(1, 128));
            properties.insert("tool_object_id".to_owned(), string(1, 128));
            common_revision(&mut properties, true);
            &["target_object_id", "tool_object_id"]
        }
        "emboss_text" => {
            properties.insert("object_id".to_owned(), string(1, 128));
            properties.insert("text".to_owned(), string(1, 256));
            properties.insert(
                "font_size".to_owned(),
                optional(integer(Some(1), Some(200)), Some(json!(10))),
            );
            properties.insert(
                "depth_mm".to_owned(),
                optional(number(Some(0.000_001), Some(50.0)), Some(json!(1.0))),
            );
            properties.insert(
                "raised".to_owned(),
                optional(property("boolean"), Some(json!(true))),
            );
            properties.insert(
                "font_name".to_owned(),
                optional(
                    enum_string(&["default", "sans", "serif", "mono"]),
                    Some(json!("default")),
                ),
            );
            properties.insert(
                "offset_x_mm".to_owned(),
                optional(number(Some(-10_000.0), Some(10_000.0)), Some(json!(0.0))),
            );
            properties.insert(
                "offset_z_mm".to_owned(),
                optional(number(Some(-10_000.0), Some(10_000.0)), Some(json!(0.0))),
            );
            common_revision(&mut properties, true);
            &["object_id", "text"]
        }
        "hollow" => {
            properties.insert("object_id".to_owned(), string(1, 128));
            properties.insert(
                "wall_thickness_mm".to_owned(),
                optional(number(Some(0.000_001), Some(50.0)), Some(json!(1.0))),
            );
            common_revision(&mut properties, true);
            &["object_id"]
        }
        "launch_slicer" => {
            properties.insert("slicer_key".to_owned(), string(1, 128));
            common_revision(&mut properties, false);
            &["slicer_key"]
        }
        _ => &[],
    };
    object(properties, required)
}

/// Returns a strict top-level output schema for one catalog entry.
#[must_use]
pub fn output_schema(name: &str) -> JsonObject {
    let mut properties = Map::new();
    let _ = name;
    // The wire contract uses several deliberately small output models.  The
    // adapter keeps a strict top-level allow-list while sharing this superset
    // for the dynamic dispatcher; every value still has a typed semantic
    // shape in the Rust result constructors.
    const OUTPUT_KEYS: &[&str] = &[
        "action",
        "app_state",
        "approved_roots",
        "affected_object_ids",
        "after",
        "before",
        "before_extents_mm",
        "before_faces",
        "before_vertices",
        "body_count",
        "bed_size_mm",
        "boundary_edge_loops",
        "boundary_edges",
        "boundary_preserved",
        "bytes",
        "busy",
        "candidate_changed",
        "candidate_safe",
        "can_apply",
        "changed",
        "changes",
        "characteristic_edge_length_after_mm",
        "characteristic_edge_length_before_mm",
        "children",
        "color",
        "collapsed_edges",
        "committed",
        "distance_mm",
        "dirty",
        "error",
        "errors",
        "estimated_peak_bytes",
        "face_count_after",
        "face_count_before",
        "fits",
        "history_state_id",
        "health",
        "issues",
        "index",
        "iterations",
        "maximum_displacement_mm",
        "message",
        "object_id",
        "objects",
        "operation",
        "operation_id",
        "path",
        "printable",
        "profile",
        "project_id",
        "progress",
        "regularized_cap_patches",
        "report",
        "resource_budget_units",
        "resource_work_units",
        "result",
        "result_after",
        "result_volume_mm3",
        "scene_revision",
        "selection",
        "selection_boundary_preserved",
        "self_intersection_status",
        "self_intersections",
        "snap_translation",
        "state",
        "subdivided_faces",
        "target_edge_length_mm",
        "target_volume_mm3",
        "tiny_shell_tolerance_mm",
        "tool_before",
        "tool_volume_mm3",
        "topology_passes",
        "torus",
        "triangles",
        "vertices",
        "view",
        "center_mm",
        "yaw_deg",
        "pitch_deg",
        "show_grid",
        "wireframe",
        "xray",
        "wall_thickness_mm",
        "warnings",
        "object_snapping_enabled",
        "applied_delta_mm",
        "proposed_delta_mm",
        "tolerance_mm",
        "snapped",
        "axis",
        "mode",
        "gap_mm",
        "results",
        "font_size",
        "depth_mm",
        "text",
        "slicer_key",
        "unit",
        "slicers",
        "slicer",
        "paths",
        "objects_exported",
        "format",
        "process_id",
    ];
    for key in OUTPUT_KEYS {
        properties.insert((*key).to_owned(), json!({}));
    }
    object(properties, &[])
}

/// Checks the catalog's JSON arguments against the exact emitted schema.
pub fn validate_arguments(name: &str, args: &Value) -> Result<(), String> {
    let schema = Value::Object(input_schema(name));
    validate_value(&schema, args, "$")
}

fn validate_value(schema: &Value, value: &Value, path: &str) -> Result<(), String> {
    if let Some(any_of) = schema.get("anyOf").and_then(Value::as_array) {
        if any_of
            .iter()
            .any(|candidate| validate_value(candidate, value, path).is_ok())
        {
            return Ok(());
        }
        return Err(format!("{path} does not match any allowed schema"));
    }
    if let Some(one_of) = schema.get("oneOf").and_then(Value::as_array) {
        let matches = one_of
            .iter()
            .filter(|candidate| validate_value(candidate, value, path).is_ok())
            .count();
        if matches == 1 {
            return Ok(());
        }
        return Err(format!("{path} must match exactly one schema"));
    }
    if let Some(constant) = schema.get("const")
        && value != constant
    {
        return Err(format!("{path} must equal {constant}"));
    }
    if let Some(enumeration) = schema.get("enum").and_then(Value::as_array)
        && !enumeration.iter().any(|candidate| candidate == value)
    {
        return Err(format!("{path} has a value outside the enum"));
    }
    if let Some(kind) = schema.get("type").and_then(Value::as_str) {
        let matches = match kind {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "number" => value.as_f64().is_some(),
            "integer" => {
                value.as_i64().is_some()
                    || value
                        .as_u64()
                        .is_some_and(|number| i64::try_from(number).is_ok())
            }
            "boolean" => value.is_boolean(),
            "null" => value.is_null(),
            _ => true,
        };
        if !matches {
            return Err(format!("{path} must be {kind}"));
        }
    }
    if let Some(object) = value.as_object() {
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for name in required.iter().filter_map(Value::as_str) {
                if !object.contains_key(name) {
                    return Err(format!("{path}.{name} is required"));
                }
            }
        }
        if schema.get("additionalProperties").and_then(Value::as_bool) == Some(false)
            && let Some(properties) = schema.get("properties").and_then(Value::as_object)
            && let Some(unknown) = object.keys().find(|key| !properties.contains_key(*key))
        {
            return Err(format!("{path}.{unknown} is not allowed"));
        }
        if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
            for (name, value) in object {
                if let Some(property_schema) = properties.get(name) {
                    validate_value(property_schema, value, &format!("{path}.{name}"))?;
                }
            }
        }
    }
    if let Some(array) = value.as_array() {
        if let Some(minimum) = schema.get("minItems").and_then(Value::as_u64)
            && array.len() < minimum as usize
        {
            return Err(format!("{path} has fewer than {minimum} items"));
        }
        if let Some(maximum) = schema.get("maxItems").and_then(Value::as_u64)
            && array.len() > maximum as usize
        {
            return Err(format!("{path} has more than {maximum} items"));
        }
        if let Some(item_schema) = schema.get("items") {
            for (index, item) in array.iter().enumerate() {
                validate_value(item_schema, item, &format!("{path}[{index}]"))?;
            }
        }
    }
    if let Some(string) = value.as_str() {
        let length = string.chars().count();
        if let Some(minimum) = schema.get("minLength").and_then(Value::as_u64)
            && length < minimum as usize
        {
            return Err(format!("{path} is shorter than {minimum}"));
        }
        if let Some(maximum) = schema.get("maxLength").and_then(Value::as_u64)
            && length > maximum as usize
        {
            return Err(format!("{path} is longer than {maximum}"));
        }
        if length > MAX_STRING {
            return Err(format!("{path} exceeds the bounded string limit"));
        }
    }
    if let Some(number) = value.as_f64() {
        if let Some(minimum) = schema.get("minimum").and_then(Value::as_f64)
            && number < minimum
        {
            return Err(format!("{path} is below {minimum}"));
        }
        if let Some(maximum) = schema.get("maximum").and_then(Value::as_f64)
            && number > maximum
        {
            return Err(format!("{path} is above {maximum}"));
        }
        if !number.is_finite() {
            return Err(format!("{path} must be finite"));
        }
    }
    if let Some(array) = value.as_array()
        && array.len() > MAX_ARRAY
        && schema.get("maxItems").is_none()
    {
        return Err(format!("{path} exceeds the bounded array limit"));
    }
    Ok(())
}

/// Reads a string, preserving a typed validation error for dispatch.
pub fn required_string(args: &Map<String, Value>, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("{key} must be a string"))
}

/// Reads an optional string with a default.
pub fn string_or(args: &Map<String, Value>, key: &str, default: &str) -> Result<String, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(default.to_owned()),
        Some(Value::String(value)) => Ok(value.clone()),
        Some(_) => Err(format!("{key} must be a string")),
    }
}

/// Reads an optional finite number with a default.
pub fn number_or(args: &Map<String, Value>, key: &str, default: f32) -> Result<f32, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(value) => value
            .as_f64()
            .filter(|number| number.is_finite())
            .map(|number| number as f32)
            .filter(|number| number.is_finite())
            .ok_or_else(|| format!("{key} must be a finite number")),
    }
}

/// Reads an optional bounded integer with a default.
pub fn integer_or(args: &Map<String, Value>, key: &str, default: i64) -> Result<i64, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(value) => value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
            .ok_or_else(|| format!("{key} must be an integer")),
    }
}

/// Reads a required or optional three-component vector.
pub fn vec3_or(
    args: &Map<String, Value>,
    key: &str,
    default: [f32; 3],
) -> Result<[f32; 3], String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Array(values)) if values.len() == 3 => {
            let mut output = [0.0; 3];
            for (index, value) in values.iter().enumerate() {
                output[index] = value
                    .as_f64()
                    .filter(|number| number.is_finite())
                    .map(|number| number as f32)
                    .filter(|number| number.is_finite())
                    .ok_or_else(|| format!("{key}[{index}] must be finite"))?;
            }
            Ok(output)
        }
        Some(_) => Err(format!("{key} must contain exactly three numbers")),
    }
}

/// Reads a bounded list of top-level indices.
pub fn indices(args: &Map<String, Value>) -> Result<Option<Vec<usize>>, String> {
    match args.get("indices") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                value
                    .as_u64()
                    .and_then(|number| usize::try_from(number).ok())
                    .ok_or_else(|| "indices must contain non-negative integers".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        Some(_) => Err("indices must be an array".to_owned()),
    }
}

/// Reads a bounded list of object IDs.
pub fn object_ids(args: &Map<String, Value>) -> Result<Option<Vec<String>>, String> {
    match args.get("object_ids") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(ToOwned::to_owned)
                    .ok_or_else(|| "object_ids must contain strings".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        Some(_) => Err("object_ids must be an array".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_reference_schema_fields_are_strict_and_bounded() {
        let translate = input_schema("translate");
        assert_eq!(translate["additionalProperties"], Value::Bool(false));
        assert_eq!(translate["properties"]["offset_mm"]["minItems"], json!(3));
        assert!(translate["properties"]["expected_project_id"]["anyOf"].is_array());

        let mirror = input_schema("mirror");
        assert_eq!(mirror["properties"]["axis"]["enum"], json!(["x", "y", "z"]));
        assert!(validate_arguments("mirror", &json!({"axis":"x"})).is_ok());
        assert!(validate_arguments("mirror", &json!({"axis":"bad"})).is_err());
        assert!(validate_arguments("mirror", &json!({"axis":"x","extra":true})).is_err());

        let batch = input_schema("apply_actions");
        assert_eq!(
            batch["properties"]["actions"]["items"]["discriminator"]["propertyName"],
            json!("action")
        );
        assert_eq!(
            input_schema("print_check")["properties"]["bed_size_mm"]["anyOf"][0]["type"],
            json!("number")
        );
        assert_eq!(
            input_schema("translate")["properties"]["return_detail"]["anyOf"][0]["enum"],
            json!(["none", "changes", "summary"])
        );
        assert!(
            input_schema("set_view")["properties"]
                .get("expected_scene_revision")
                .is_none()
        );
        assert_eq!(
            input_schema("snap_translate")["properties"]["tolerance_mode"]["anyOf"][0]["enum"],
            json!(["fixed_mm", "view_scaled", "floor_fixed"])
        );
        assert_eq!(
            input_schema("repair")["properties"]["profile"]["anyOf"][0]["enum"],
            json!(["simple", "extended"])
        );
        assert!(output_schema("get_status")["additionalProperties"] == Value::Bool(false));
        assert!(
            output_schema("discover_slicers")["properties"]
                .get("slicers")
                .is_some()
        );
    }
}
