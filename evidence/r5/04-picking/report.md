# R5.4 picking evidence

Status: complete.

`picking.rs` performs exact CPU ray/AABB/triangle picking over retained full
geometry, reports object ID, face, distance, world position, and barycentric
coordinates, and bounds one pass at 30,000,000 triangles. Display LOD is never
used as a picking substitute.

Rust exact-pick and ray tests passed. The Python reference gizmo/ray selectors
passed.
