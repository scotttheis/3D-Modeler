//! MCP catalog parity check (plan section 3.7, Full gate).
//!
//! Every feature row in `evidence/rebuild/parity-matrix.toml` must name the MCP
//! tool or tools that perform the desktop action (`mcp_tool = [...]`, each one
//! in the published catalog) or be exempted as presentation only
//! (`mcp_exempt = "visual only"`). A row with neither fails.

use std::{collections::BTreeSet, fs, path::PathBuf};

use builder3d_mcp::TOOL_CATALOG;

struct Row {
    id: String,
    tools: Option<Vec<String>>,
    exempt: Option<String>,
}

fn ledger_rows() -> Vec<Row> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../evidence/rebuild/parity-matrix.toml");
    let text = fs::read_to_string(&path).expect("parity ledger is readable");
    let mut rows: Vec<Row> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line == "[[feature]]" {
            rows.push(Row {
                id: String::new(),
                tools: None,
                exempt: None,
            });
        } else if let Some(row) = rows.last_mut() {
            if let Some(value) = line.strip_prefix("id = ") {
                row.id = value.trim_matches('"').to_owned();
            } else if let Some(value) = line.strip_prefix("mcp_tool = ") {
                row.tools = Some(
                    value
                        .trim_start_matches('[')
                        .trim_end_matches(']')
                        .split(',')
                        .map(|name| name.trim().trim_matches('"').to_owned())
                        .filter(|name| !name.is_empty())
                        .collect(),
                );
            } else if let Some(value) = line.strip_prefix("mcp_exempt = ") {
                row.exempt = Some(value.trim_matches('"').to_owned());
            }
        }
    }
    rows
}

#[test]
fn every_desktop_action_has_an_mcp_tool_or_a_visual_only_exemption() {
    let catalog: BTreeSet<&str> = TOOL_CATALOG.iter().map(|spec| spec.name).collect();
    let rows = ledger_rows();
    assert!(rows.len() >= 147, "ledger rows: {}", rows.len());
    let mut problems = Vec::new();
    for row in &rows {
        match (&row.tools, &row.exempt) {
            (Some(tools), None) if !tools.is_empty() => {
                for tool in tools {
                    if !catalog.contains(tool.as_str()) {
                        problems.push(format!("{}: unknown tool {tool}", row.id));
                    }
                }
            }
            (None, Some(reason)) if reason == "visual only" => {}
            _ => problems.push(format!(
                "{}: needs mcp_tool = [..] or mcp_exempt = \"visual only\"",
                row.id
            )),
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_f12_tools_are_reachable_from_the_ledger() {
    let rows = ledger_rows();
    for tool in ["rename_object", "set_visibility", "launch_slicer"] {
        assert!(
            rows.iter().any(|row| row
                .tools
                .as_ref()
                .is_some_and(|t| t.iter().any(|n| n == tool))),
            "no ledger row maps to {tool}"
        );
    }
}
