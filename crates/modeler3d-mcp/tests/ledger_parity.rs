//! MCP catalog parity check (plan section 3.7, Full gate).
//!
//! Every feature row in `evidence/rebuild/parity-matrix.toml` must name the MCP
//! tool or tools that perform the desktop action (`mcp_tool = [...]`, each one
//! in the published catalog). Owner decision D10 (OF-58, 2026-11-14): there are
//! no "visual only" exemptions any more; a row without tools fails.

use std::{collections::BTreeSet, fs, path::PathBuf};

use modeler3d_mcp::TOOL_CATALOG;

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
fn every_desktop_action_has_an_mcp_tool() {
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
            (_, Some(reason)) => problems.push(format!(
                "{}: mcp_exempt = {reason:?} is no longer allowed (D10); name its MCP tools",
                row.id
            )),
            _ => problems.push(format!("{}: needs mcp_tool = [..]", row.id)),
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_f12_and_of58_tools_are_reachable_from_the_ledger() {
    let rows = ledger_rows();
    for tool in ["rename_object", "set_visibility", "launch_slicer"]
        .into_iter()
        .chain(modeler3d_mcp::SHELL_TOOLS)
    {
        assert!(
            rows.iter().any(|row| row
                .tools
                .as_ref()
                .is_some_and(|t| t.iter().any(|n| n == tool))),
            "no ledger row maps to {tool}"
        );
    }
}
