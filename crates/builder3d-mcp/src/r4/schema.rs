//! Draft 2020-12 schemas captured from the currently designated Python MCP
//! reference, plus the validator used for both inputs and structured outputs.
//!
//! The reference MCP SDK accepts unknown top-level input fields and drops them
//! while validating its generated argument model. Nested Pydantic models are
//! closed. `normalize_arguments` mirrors that behavior after validating the
//! complete schema, including nested references and constraints.

use jsonschema::{Validator, draft202012};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

type JsonObject = Map<String, Value>;
type ValidatorCache = HashMap<String, Arc<Validator>>;

static CONTRACT: OnceLock<Value> = OnceLock::new();
static VALIDATORS: OnceLock<Mutex<ValidatorCache>> = OnceLock::new();

fn contract() -> &'static Value {
    CONTRACT.get_or_init(|| {
        serde_json::from_str(include_str!("schema_contract.json"))
            .expect("embedded MCP schema contract must be valid JSON")
    })
}

fn tool_contract(name: &str) -> Result<&'static Value, String> {
    contract()["tools"]
        .get(name)
        .ok_or_else(|| format!("no reference MCP schema is registered for {name}"))
}

fn input_schema_value(name: &str) -> Result<&'static Value, String> {
    tool_contract(name)?
        .get("input_schema")
        .ok_or_else(|| format!("the reference input schema for {name} is missing"))
}

fn output_schema_value(name: &str) -> Result<&'static Value, String> {
    let schema_id = tool_contract(name)?
        .get("output_schema_id")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("the reference output-schema id for {name} is missing"))?;
    contract()["output_schemas"]
        .get(schema_id)
        .ok_or_else(|| format!("the reference output schema {schema_id} is missing"))
}

fn schema_object(name: &str, schema: Result<&Value, String>) -> JsonObject {
    schema
        .unwrap_or_else(|message| panic!("{message}"))
        .as_object()
        .unwrap_or_else(|| panic!("the MCP schema for {name} must be an object"))
        .clone()
}

/// Returns the current reference input schema for one catalog entry.
#[must_use]
pub fn input_schema(name: &str) -> JsonObject {
    schema_object(name, input_schema_value(name))
}

/// Returns the current reference output schema for one catalog entry.
#[must_use]
pub fn output_schema(name: &str) -> JsonObject {
    schema_object(name, output_schema_value(name))
}

fn compiled_validator(key: String, schema: &Value) -> Result<Arc<Validator>, String> {
    let cache = VALIDATORS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = cache
        .lock()
        .map_err(|_| "the JSON Schema validator cache is poisoned".to_owned())?;
    if let Some(validator) = cache.get(&key) {
        return Ok(Arc::clone(validator));
    }

    let validator = draft202012::options()
        .build(schema)
        .map_err(|error| format!("invalid Draft 2020-12 schema {key}: {error}"))?;
    let validator = Arc::new(validator);
    cache.insert(key, Arc::clone(&validator));
    Ok(validator)
}

fn validate(schema_kind: &str, name: &str, schema: &Value, value: &Value) -> Result<(), String> {
    let validator = compiled_validator(format!("{schema_kind}:{name}"), schema)?;
    validator
        .validate(value)
        .map_err(|error| format!("{name} {schema_kind} schema rejected the value: {error}"))
}

/// Validates arguments against the full published Draft 2020-12 input schema.
pub fn validate_arguments(name: &str, args: &Value) -> Result<(), String> {
    validate("input", name, input_schema_value(name)?, args)
}

/// Validates and removes unknown top-level input fields, matching the current
/// Python SDK's argument-model behavior. Nested schemas are validated in full.
pub fn normalize_arguments(name: &str, args: &Value) -> Result<Value, String> {
    validate_arguments(name, args)?;
    let object = args
        .as_object()
        .ok_or_else(|| "tool arguments must be a JSON object".to_owned())?;
    let properties = input_schema_value(name)?["properties"]
        .as_object()
        .ok_or_else(|| format!("the input schema for {name} has no properties object"))?;
    let mut normalized = Map::new();
    for key in properties.keys() {
        if let Some(value) = object.get(key) {
            normalized.insert(key.clone(), value.clone());
        }
    }
    Ok(Value::Object(normalized))
}

/// Validates a successful structured result against the complete tool output
/// schema, including every nested `$ref`, union, required field, and bound.
pub fn validate_result(name: &str, result: &Value) -> Result<(), String> {
    validate("output", name, output_schema_value(name)?, result)
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
    use crate::r4::catalog::{Profile, TOOL_CATALOG};
    use serde_json::json;

    fn example_from_schema(schema: &Value, root: &Value) -> Value {
        if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
            let pointer = reference
                .strip_prefix('#')
                .expect("embedded schema references are local");
            return example_from_schema(
                root.pointer(pointer)
                    .unwrap_or_else(|| panic!("schema reference {reference} resolves")),
                root,
            );
        }
        if let Some(default) = schema.get("default") {
            return default.clone();
        }
        if let Some(value) = schema.get("const") {
            return value.clone();
        }
        if let Some(values) = schema.get("enum").and_then(Value::as_array) {
            return values.first().cloned().unwrap_or(Value::Null);
        }
        if let Some(branches) = schema
            .get("anyOf")
            .or_else(|| schema.get("oneOf"))
            .and_then(Value::as_array)
        {
            return branches
                .first()
                .map(|branch| example_from_schema(branch, root))
                .unwrap_or(Value::Null);
        }
        match schema.get("type").and_then(Value::as_str) {
            Some("object") => {
                let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
                    return json!({});
                };
                let required = schema
                    .get("required")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>();
                let mut object = Map::new();
                for name in required {
                    let property = properties
                        .get(name)
                        .unwrap_or_else(|| panic!("required property {name} is declared"));
                    object.insert(name.to_owned(), example_from_schema(property, root));
                }
                Value::Object(object)
            }
            Some("array") => {
                let minimum = schema.get("minItems").and_then(Value::as_u64).unwrap_or(0) as usize;
                let item_schema = schema.get("items").unwrap_or(&Value::Null);
                Value::Array(
                    (0..minimum)
                        .map(|_| example_from_schema(item_schema, root))
                        .collect(),
                )
            }
            Some("string") => {
                let minimum = schema.get("minLength").and_then(Value::as_u64).unwrap_or(0) as usize;
                Value::String("a".repeat(minimum))
            }
            Some("integer") | Some("number") => {
                let mut number = schema.get("minimum").and_then(Value::as_f64).unwrap_or(0.0);
                if let Some(exclusive) = schema.get("exclusiveMinimum").and_then(Value::as_f64)
                    && number <= exclusive
                {
                    number = exclusive + 1.0;
                }
                if let Some(maximum) = schema.get("maximum").and_then(Value::as_f64) {
                    number = number.min(maximum);
                }
                if schema.get("type").and_then(Value::as_str) == Some("integer") {
                    number = number.ceil();
                }
                json!(number)
            }
            Some("boolean") => Value::Bool(false),
            Some("null") => Value::Null,
            _ => Value::Null,
        }
    }

    #[test]
    fn captured_contract_compiles_for_every_tool_in_every_profile() {
        assert_eq!(
            contract()["tools"].as_object().map(JsonObject::len),
            Some(60)
        );
        assert_eq!(
            contract()["output_schemas"]
                .as_object()
                .map(JsonObject::len),
            Some(18)
        );

        for (profile, expected_count) in [
            (Profile::Observe, 7),
            (Profile::Model, 52),
            (Profile::Files, 59),
            (Profile::Full, 60),
        ] {
            let visible = TOOL_CATALOG
                .iter()
                .filter(|spec| spec.visible_in(profile))
                .collect::<Vec<_>>();
            assert_eq!(visible.len(), expected_count, "{} profile", profile);
            for spec in visible {
                let input = input_schema_value(spec.name).expect("reference input schema");
                let output = output_schema_value(spec.name).expect("reference output schema");
                compiled_validator(format!("input:{}", spec.name), input)
                    .unwrap_or_else(|error| panic!("{} input schema: {error}", spec.name));
                let output_id =
                    tool_contract(spec.name).expect("tool contract")["output_schema_id"]
                        .as_str()
                        .expect("output schema id");
                compiled_validator(format!("output:{}", spec.name), output)
                    .unwrap_or_else(|error| panic!("{} output schema: {error}", spec.name));
                assert!(
                    contract()["output_schemas"].get(output_id).is_some(),
                    "{} output schema id is present",
                    spec.name
                );
            }
        }
    }

    #[test]
    fn return_detail_is_limited_to_the_five_reference_tools() {
        let expected: &[&str] = &[
            "apply_actions",
            "set_color",
            "set_transform",
            "transform_selection",
            "translate",
        ];
        for (profile, expected_count) in [
            (Profile::Observe, 0),
            (Profile::Model, 5),
            (Profile::Files, 5),
            (Profile::Full, 5),
        ] {
            let mut names = TOOL_CATALOG
                .iter()
                .filter(|spec| spec.visible_in(profile))
                .filter(|spec| {
                    input_schema_value(spec.name)
                        .expect("reference input schema")
                        .pointer("/properties/return_detail")
                        .is_some()
                })
                .map(|spec| spec.name)
                .collect::<Vec<_>>();
            names.sort_unstable();
            assert_eq!(names.len(), expected_count, "{} profile", profile);
            assert_eq!(names, if expected_count == 0 { &[] } else { expected });
        }
    }

    #[test]
    fn recursive_input_and_output_examples_validate_for_every_profile() {
        for profile in [
            Profile::Observe,
            Profile::Model,
            Profile::Files,
            Profile::Full,
        ] {
            for spec in TOOL_CATALOG.iter().filter(|spec| spec.visible_in(profile)) {
                let input_schema = input_schema_value(spec.name).expect("reference input schema");
                let input = example_from_schema(input_schema, input_schema);
                validate_arguments(spec.name, &input).unwrap_or_else(|error| {
                    panic!(
                        "{} profile {} input example: {error}; {input}",
                        spec.name, profile
                    )
                });

                let output_schema =
                    output_schema_value(spec.name).expect("reference output schema");
                let output = example_from_schema(output_schema, output_schema);
                validate_result(spec.name, &output).unwrap_or_else(|error| {
                    panic!(
                        "{} profile {} output example: {error}; {output}",
                        spec.name, profile
                    )
                });
            }
        }
    }

    #[test]
    fn top_level_unknown_fields_are_dropped_and_nested_models_remain_closed() {
        assert_eq!(
            normalize_arguments("mirror", &json!({"axis":"x","future":true}))
                .expect("unknown top-level fields match the reference SDK"),
            json!({"axis":"x"})
        );
        assert_eq!(
            normalize_arguments("get_status", &json!({"future":true}))
                .expect("zero-argument model drops extras"),
            json!({})
        );
        assert_eq!(
            normalize_arguments(
                "apply_actions",
                &json!({
                    "actions":[{"action":"translate","args":{"offset_mm":[1,2,3]}}],
                    "future":true
                })
            )
            .expect("top-level extension is dropped"),
            json!({"actions":[{"action":"translate","args":{"offset_mm":[1,2,3]}}]})
        );
        assert!(normalize_arguments(
            "apply_actions",
            &json!({"actions":[{"action":"translate","args":{"offset_mm":[1,2,3],"future":true}}]})
        )
        .is_err());
    }

    #[test]
    fn reference_requiredness_nullability_and_bounds_are_enforced_recursively() {
        assert!(validate_arguments("orbit_view", &json!({})).is_ok());
        assert!(validate_arguments("mirror", &json!({})).is_err());
        assert!(validate_arguments("mirror", &json!({"axis":"sideways"})).is_err());
        assert!(validate_arguments("set_transform", &json!({"size_mm":[0,1,2]})).is_err());
        assert!(
            validate_arguments(
                "apply_actions",
                &json!({"actions":[{"action":"translate","args":{"offset_mm":[1,2]}}]})
            )
            .is_err()
        );
    }

    #[test]
    fn typed_output_schemas_reject_missing_nested_fields_and_extra_properties() {
        assert!(
            validate_result(
                "get_status",
                &json!({"app_state":"ready","unexpected":true})
            )
            .is_err()
        );
        assert!(
            validate_result(
                "view_state",
                &json!({"yaw_deg":0,"pitch_deg":0,"distance_mm":10,"center_mm":[0,0]})
            )
            .is_err()
        );
        assert!(
            validate_result(
                "view_state",
                &json!({"yaw_deg":0,"pitch_deg":0,"distance_mm":10,"center_mm":[0,0,0]})
            )
            .is_ok()
        );

        let nested_scene = json!({
            "objects":[{
                "object_id":"root",
                "index":0,
                "children":[{
                    "object_id":"child",
                    "children":[{"object_id":"grandchild"}]
                }],
                "mesh_health":{"valid":true}
            }]
        });
        assert!(validate_result("scene_summary", &nested_scene).is_ok());
        let nested_extra = json!({
            "objects":[{
                "object_id":"root",
                "index":0,
                "children":[{"object_id":"child","unexpected":true}]
            }]
        });
        assert!(validate_result("scene_summary", &nested_extra).is_err());
        assert!(
            validate_result(
                "translate",
                &json!({"result":{"object_id":"cube","index":0,"mesh_health":{"valid":true}}})
            )
            .is_ok()
        );
    }

    #[test]
    fn captured_release_success_results_match_reference_output_schemas_when_supplied() {
        let Some(path) = std::env::var_os("B3D_RB09_SCHEMA_RESULT_LOG") else {
            eprintln!(
                "set B3D_RB09_SCHEMA_RESULT_LOG to validate a captured Release MCP result log"
            );
            return;
        };
        let log = std::fs::read_to_string(path).expect("read captured Release MCP results");
        let mut validated = 0;
        for (line_number, line) in log
            .lines()
            .enumerate()
            .filter(|(_, line)| !line.trim().is_empty())
        {
            let event: Value = serde_json::from_str(line).unwrap_or_else(|error| {
                panic!("result log line {} is JSON: {error}", line_number + 1)
            });
            if event.get("event").and_then(Value::as_str) != Some("tool_result")
                || event.get("is_error").and_then(Value::as_bool) != Some(false)
            {
                continue;
            }
            let Some(name) = event.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(result) = event.get("structured_content") else {
                continue;
            };
            validate_result(name, result).unwrap_or_else(|error| {
                panic!(
                    "captured Release result {name} at line {}: {error}",
                    line_number + 1
                )
            });
            validated += 1;
        }
        assert!(
            validated > 0,
            "the supplied Release result log has successful structured outputs"
        );
    }
}
