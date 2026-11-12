# R4b — catalog, profile, and contract evidence

R4b ports the Python reference catalog into a deterministic Rust catalog with
the exact 60-tool order, profile minima, categories, mutability annotations,
idempotency flags, open-world flags, and long-operation flags. The Rust server
publishes strict input schemas, bounded output schemas, titles, annotations,
catalog metadata, and structured results through `tools/list` and
`tools/call`.

Focused contract coverage includes:

- exact catalog length/order and monotonic `observe`/`model`/`files`/`full`
  profile filtering;
- strict unknown-field rejection and bounded arrays, strings, numbers,
  enums, object references, revision/project context, and idempotency keys;
- official-client validation of the model/view/print/operation/catalog path;
- grouped `apply_actions` with one core `GroupedCommand` history entry;
- real clipboard copy/cut/paste through the core scene command boundary;
- platform-owned slicer discovery metadata rather than a generic process tool.

The official behavior probe completed with 60 tools, structured results, one
scene object, a completed asynchronous smooth operation, and a discovered
slicer count that is host-dependent. The Python reference catalog and SDK
tests passed 9 tests.

The next ordered checkpoint is R4c: behavior, security, replay, revision
conflict handling, and bounded asynchronous operations.
