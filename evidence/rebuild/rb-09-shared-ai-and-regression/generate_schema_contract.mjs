import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

const runRoot = resolve(
  process.argv[2] ??
    "evidence/rebuild/rb-09-shared-ai-and-regression/reference-current-20261107-084248",
);
const capturePath = join(runRoot, "python-full.json");
const outputPath = resolve(
  "crates/builder3d-mcp/src/r4/schema_contract.json",
);
const captureBytes = readFileSync(capturePath);
const capture = JSON.parse(captureBytes.toString("utf8"));

if (capture.profile !== "full" || capture.tool_count !== 60) {
  throw new Error("Expected the refreshed 60-tool full-profile reference catalog");
}

const canonical = (value) => {
  if (Array.isArray(value)) return value.map(canonical);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((key) => [key, canonical(value[key])]),
    );
  }
  return value;
};
const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");
const outputSchemas = new Map();
const tools = {};

for (const tool of capture.tools) {
  const canonicalOutput = JSON.stringify(canonical(tool.outputSchema));
  const outputId = sha256(canonicalOutput);
  if (!outputSchemas.has(outputId)) outputSchemas.set(outputId, tool.outputSchema);
  tools[tool.name] = {
    input_schema: tool.inputSchema,
    output_schema_id: outputId,
  };
}

if (Object.keys(tools).length !== 60 || outputSchemas.size !== 18) {
  throw new Error("The reference tool or distinct output-schema count changed");
}

const contract = {
  schema_dialect: "https://json-schema.org/draft/2020-12/schema",
  source_capture: {
    artifact: "python-full.json",
    artifact_sha256: sha256(captureBytes),
    captured_utc: capture.captured_utc,
    protocol_version: capture.client.protocol_version,
    python_version: capture.client.python_version,
    sdk_package: capture.client.sdk_package,
    sdk_version: capture.client.sdk_version,
    reference_path: capture.reference_identity.path,
    reference_git_head: capture.reference_identity.git_head,
    reference_git_status_sha256: capture.reference_identity.git_status_sha256,
  },
  output_schemas: Object.fromEntries(
    [...outputSchemas.entries()].sort(([left], [right]) => left.localeCompare(right)),
  ),
  tools,
};

writeFileSync(outputPath, `${JSON.stringify(contract, null, 2)}\n`, "utf8");
console.log(
  JSON.stringify({
    output: outputPath,
    tool_count: Object.keys(tools).length,
    distinct_output_schemas: outputSchemas.size,
    source_capture_sha256: contract.source_capture.artifact_sha256,
  }, null, 2),
);
