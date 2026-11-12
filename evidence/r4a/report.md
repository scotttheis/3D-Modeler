# R4a — portable MCP connection and protocol evidence

R4a establishes the Rust server’s portable `rmcp` stdio boundary and keeps
protocol/session state in the MCP adapter rather than in the core scene or a
desktop UI. The server emits only protocol JSON on stdout, publishes the
trusted launch profile, and exposes bounded operation/replay state.

Evidence captured on 2026-10-29:

- The official Python MCP client discovered all 60 tools in deterministic
  order, received structured results, and negotiated `2025-11-25` through the
  installed Python SDK.
- The raw pre-initialize discovery probe reported support for
  `2024-11-05`, `2025-03-26`, `2025-06-18`, `2025-11-25`, and `2026-07-28`,
  plus the tools capability and `builder3d-rust-mcp` server identity.
- `cargo fmt --all -- --check`, `cargo check --workspace`, workspace tests,
  and workspace clippy with `-D warnings` passed at the completed R4 run.
- The approved-root policy defaults to deny-all, rejects canonical paths that
  traverse symbolic-link/reparse-point components, and refuses overwrite
  unless explicitly requested.

The protocol assumptions were checked against the [official MCP tools
specification](https://modelcontextprotocol.io/specification/2026-07-28/server/tools),
[official transport specification](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports),
and [official authorization specification](https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization).
The Rust SDK dependency is pinned to `rmcp = 3.8.0`; the official SDK source is
the [modelcontextprotocol/rust-sdk repository](https://github.com/modelcontextprotocol/rust-sdk).

The next ordered checkpoint is R4b: exact catalog, profile, metadata, and
strict schema parity.
