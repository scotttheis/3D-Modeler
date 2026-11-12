# R5.8 central resource-ledger evidence

Status: complete.

`ResourceLedger` now exposes used, budget, pressure, and available bytes and
accounts authoritative scene data, GPU render targets, mesh caches, and
transient overlays. Reservation failure is typed; release is performed when
cached GPU objects and the renderer are dropped.

Ledger tests cover unified memory charging, pressure rejection, release, and
snapshot values. The R5 report records the live Quadro budget and allocation
kind totals.
