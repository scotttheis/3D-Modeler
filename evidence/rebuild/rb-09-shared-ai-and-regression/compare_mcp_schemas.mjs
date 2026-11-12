import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const profiles = ["observe", "model", "files", "full"];
const expectedCounts = { observe: 7, model: 52, files: 59, full: 60 };
const here = dirname(fileURLToPath(import.meta.url));
const referenceDirectory = resolve(process.argv[2] ?? join(here, "reference-current-20261107-084248"));
const rustDirectory = resolve(process.argv[3] ?? join(here, "currpair-20261107-085147", "catalogs"));
const outputPath = process.argv[4] ? resolve(process.argv[4]) : join(rustDirectory, "schema-property-diff-current.json");

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.keys(value).sort().map((key) => [key, canonical(value[key])]),
    );
  }
  return value;
}

function stableJson(value) {
  return JSON.stringify(canonical(value));
}

function sha256(value) {
  return createHash("sha256").update(stableJson(value)).digest("hex");
}

function pointer(path, key) {
  const encoded = String(key).replaceAll("~", "~0").replaceAll("/", "~1");
  return `${path}/${encoded}`;
}

function diff(left, right, path = "", differences = []) {
  if (stableJson(left) === stableJson(right)) return differences;
  const leftIsObject = left !== null && typeof left === "object" && !Array.isArray(left);
  const rightIsObject = right !== null && typeof right === "object" && !Array.isArray(right);
  if (leftIsObject && rightIsObject) {
    const keys = [...new Set([...Object.keys(left), ...Object.keys(right)])].sort();
    for (const key of keys) {
      if (!Object.hasOwn(left, key)) {
        differences.push({ pointer: pointer(path, key), reference: "<missing>", rust: right[key] });
      } else if (!Object.hasOwn(right, key)) {
        differences.push({ pointer: pointer(path, key), reference: left[key], rust: "<missing>" });
      } else {
        diff(left[key], right[key], pointer(path, key), differences);
      }
    }
    return differences;
  }
  if (Array.isArray(left) && Array.isArray(right)) {
    const length = Math.max(left.length, right.length);
    for (let index = 0; index < length; index += 1) {
      if (index >= left.length) {
        differences.push({ pointer: pointer(path, index), reference: "<missing>", rust: right[index] });
      } else if (index >= right.length) {
        differences.push({ pointer: pointer(path, index), reference: left[index], rust: "<missing>" });
      } else {
        diff(left[index], right[index], pointer(path, index), differences);
      }
    }
    return differences;
  }
  differences.push({ pointer: path || "/", reference: left, rust: right });
  return differences;
}

function returnDetailTools(tools) {
  return tools
    .filter((tool) => Object.hasOwn(tool.inputSchema?.properties ?? {}, "return_detail"))
    .map((tool) => tool.name);
}

function toolFields(tool) {
  return {
    name: tool.name,
    inputSchema: tool.inputSchema,
    outputSchema: tool.outputSchema,
    annotations: tool.annotations,
  };
}

const profileResults = {};
const differences = [];
const metadataDifferences = [];
let allProfilesExact = true;
for (const profile of profiles) {
  const referencePath = join(referenceDirectory, `python-${profile}.json`);
  const rustPath = join(rustDirectory, `rust-${profile}-live.json`);
  const reference = readJson(referencePath);
  const rust = readJson(rustPath);
  const referenceTools = reference.tools;
  const rustTools = rust.tools;
  const referenceNames = referenceTools.map((tool) => tool.name);
  const rustNames = rustTools.map((tool) => tool.name);
  const referenceByName = new Map(referenceTools.map((tool) => [tool.name, tool]));
  const rustByName = new Map(rustTools.map((tool) => [tool.name, tool]));
  const toolNames = [...new Set([...referenceNames, ...rustNames])].sort();
  let inputMatches = 0;
  let outputMatches = 0;
  let annotationMatches = 0;
  let metadataMatches = 0;
  let fullRecordMatches = 0;
  let customMetadataDiffCount = 0;
  let diffCount = 0;

  for (const name of toolNames) {
    const pythonTool = referenceByName.get(name);
    const rustTool = rustByName.get(name);
    if (!pythonTool || !rustTool) {
      const item = {
        profile,
        tool: name,
        pointer: "/name",
        reference: pythonTool ? "present" : "<missing>",
        rust: rustTool ? "present" : "<missing>",
      };
      differences.push(item);
      diffCount += 1;
      continue;
    }
    if (stableJson(pythonTool.inputSchema) === stableJson(rustTool.inputSchema)) inputMatches += 1;
    if (stableJson(pythonTool.outputSchema) === stableJson(rustTool.outputSchema)) outputMatches += 1;
    if (stableJson(pythonTool.annotations) === stableJson(rustTool.annotations)) annotationMatches += 1;
    if (stableJson(pythonTool._meta) === stableJson(rustTool._meta)) metadataMatches += 1;
    const recordDiffs = diff(toolFields(pythonTool), toolFields(rustTool), "");
    const metaDiffs = diff(pythonTool._meta, rustTool._meta, "/_meta");
    customMetadataDiffCount += metaDiffs.length;
    if (recordDiffs.length === 0) fullRecordMatches += 1;
    for (const item of recordDiffs) {
      differences.push({ profile, tool: name, ...item });
      diffCount += 1;
    }
    for (const item of metaDiffs) metadataDifferences.push({ profile, tool: name, ...item });
  }

  const returnDetailReference = returnDetailTools(referenceTools);
  const returnDetailRust = returnDetailTools(rustTools);
  const exact = referenceNames.length === expectedCounts[profile]
    && rustNames.length === expectedCounts[profile]
    && stableJson(referenceNames) === stableJson(rustNames)
    && inputMatches === expectedCounts[profile]
    && outputMatches === expectedCounts[profile]
    && annotationMatches === expectedCounts[profile]
    && fullRecordMatches === expectedCounts[profile]
    && returnDetailReference.length === returnDetailRust.length
    && stableJson(returnDetailReference) === stableJson(returnDetailRust);
  allProfilesExact &&= exact;
  profileResults[profile] = {
    expected_tool_count: expectedCounts[profile],
    reference_tool_count: referenceNames.length,
    rust_tool_count: rustNames.length,
    ordered_names_match: stableJson(referenceNames) === stableJson(rustNames),
    input_schema_matches: inputMatches,
    output_schema_matches: outputMatches,
    annotation_matches: annotationMatches,
    schema_annotation_record_matches: fullRecordMatches,
    custom_metadata_matches: metadataMatches,
    custom_metadata_difference_count: customMetadataDiffCount,
    reference_distinct_output_schemas: new Set(referenceTools.map((tool) => sha256(tool.outputSchema))).size,
    rust_distinct_output_schemas: new Set(rustTools.map((tool) => sha256(tool.outputSchema))).size,
    reference_return_detail_count: returnDetailReference.length,
    rust_return_detail_count: returnDetailRust.length,
    reference_return_detail_tools: returnDetailReference,
    rust_return_detail_tools: returnDetailRust,
    recursive_difference_count: diffCount,
    exact_schema_and_annotation_parity: exact,
    reference_capture: referencePath,
    rust_capture: rustPath,
  };
}

const report = {
  schema_version: 2,
  captured_utc: new Date().toISOString(),
  comparison_scope: "Current designated L reference capture versus live current Rust Release tools/list, compared recursively across complete input/output JSON Schemas and official MCP annotations for all four profiles. Custom _meta is reported separately because it is product metadata rather than schema or ToolAnnotations.",
  schema_dialect: "https://json-schema.org/draft/2020-12/schema",
  reference_identity: readJson(join(referenceDirectory, "reference-identity.json")),
  reference_catalog_capture: join(referenceDirectory, "python-full.json"),
  rust_capture_summary: join(rustDirectory, "rust-capture-summary.json"),
  profiles: profileResults,
  total_recursive_difference_count: differences.length,
  exact_parity: allProfilesExact && differences.length === 0,
  differences,
  custom_metadata_difference_count: metadataDifferences.length,
  custom_metadata_differences: metadataDifferences,
};

writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
console.log(JSON.stringify({
  output: isAbsolute(outputPath) ? outputPath : resolve(outputPath),
  exact_parity: report.exact_parity,
  profiles: Object.fromEntries(Object.entries(profileResults).map(([name, item]) => [name, {
    reference_tool_count: item.reference_tool_count,
    rust_tool_count: item.rust_tool_count,
    input_schema_matches: item.input_schema_matches,
    output_schema_matches: item.output_schema_matches,
    reference_return_detail_count: item.reference_return_detail_count,
    rust_return_detail_count: item.rust_return_detail_count,
    recursive_difference_count: item.recursive_difference_count,
    exact_schema_and_annotation_parity: item.exact_schema_and_annotation_parity,
    custom_metadata_difference_count: item.custom_metadata_difference_count,
  }])),
  total_recursive_difference_count: differences.length,
}, null, 2));

if (!report.exact_parity) process.exitCode = 1;
