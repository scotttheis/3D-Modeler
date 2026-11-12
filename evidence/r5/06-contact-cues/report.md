# R5.6 contact-cue evidence

Status: complete.

`ContactCueCache` derives bounded world-AABB footprints only after the scene is
settled, skips all cue work while camera interaction is active, and reuses the
settled revision cache. Renderer overlays use translucent depth-tested cue
triangles.

Rust cache tests and the Python camera-interaction contact-cue test passed.
