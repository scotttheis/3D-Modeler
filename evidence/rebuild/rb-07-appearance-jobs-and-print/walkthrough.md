# RB-07 native walkthrough

## Capture setup

The existing Python reference was run first, closed, then the final Rust debug executable was run. The sequence used actual native windows and OS input. Builder3D client size was 1280x840 logical and physical pixels at 96 DPI / 100% display scale. The Rust native identity is SHA-256 0BF7753FE460A6A854906457F86363DDC905B455FA790760B6D9122F6F9CB0D6. Capture routes, themes, frame hashes, build identity, and PNG last-write UTC timestamps are recorded per image in capture-metadata.json.

## Appearance and image

In Light and Dark, the Python reference and Rust app visited the Paint surface; opened Material, Paint, and Color; applied a solid color; canceled a color edit; selected an image path; sampled a valid image; showed the resulting solid color; and opened a corrupt image path to verify the visible error and successful retry. Rust also exported the authored red material to 3MF and re-imported it; the re-imported face retained #FF2207FF.

The picker accepts an image to sample its average color. This contract creates one opaque solid RGBA material color and does not create a texture. Missing, unreadable, oversized-byte, oversized-pixel, and unsupported-format limits are exercised by automated tests; native failure frames cover the corrupt-image case.

## Background operations

Both Python themes used a closed 1,920,000-face mesh fixture. The native sequence captured scene ready, Smooth running with Cancel enabled, cancel request, terminal cancellation, and a successful Smooth retry. The reference reports cancellation and retry success without a partial history commit.

Rust Dark used the same heavy fixture and captured import, Smooth running, Cancel request, terminal “Smooth cancelled; no scene changes were applied”, retry success, a controlled Subdivide resource-limit failure, and a later successful retry. Rust Light captured the running state and successful retry. Additional Light Subdivide evidence records a successful 3,840,000-face result followed by a controlled failure at 7,680,000 faces over the 4,000,000-face limit.

Rust Light native cancellation was attempted several times, but no captured frame proves a cancelled terminal state in that theme. Frames where the operation had already completed, the wrong command was active, or the state was not verifiable were rejected. Rust shell tests verify cancel/no-commit/retry, duplicate busy status, selection/revision guards, and dialog close/reopen.

## Print preparation and slicer handoff

Python clean Print Prep reports were manually captured in both themes after the native runner's menu QAction route failed to open its report dialog. Rust captured the clean report in Light, clean and warning reports in Dark, progress, staging-ready state, slicer selection, and launch state. The report is read-only; automated tests cover fit, below-bed, height, scale, health, hidden, empty, failure, retry, and scene preservation.

The Rust Bambu path staged a binary STL for the visible 40 mm cube. Independent verification found a nonzero 684-byte binary STL with 12 triangles and SHA-256 EC1B527705B8045D5748A532519BF9EAE4AD3B7DDAD6081B85C3386669A8C15C. Rust launched the installed C:\Program Files\Bambu Studio\bambu-studio.exe; the slicer showed Builder3D_Print_001.stl as 40 x 40 x 40 mm, 64,000 mm3, 12 triangles. The test process was closed after capture. Python external launch is covered by focused reference adapter tests but was not natively captured.

## Cleanup and disposition

Builder3D and Bambu Studio test processes were closed after each native sequence. The final accepted set has 117 PNGs. 142 rejected PNGs plus their raw logs are retained under rejected/; none is used for a parity claim. See rejected-attempts.md.

