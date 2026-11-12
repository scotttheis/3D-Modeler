# Rejected native attempts

The accepted captures directory contains exactly 117 PNGs. Rejected attempts are retained separately under rejected/: **142 PNGs among 162 files** (the other 20 files are raw result journals and run logs). Rejected images are never referenced as positive parity evidence.

| Location | Count | Why excluded |
|---|---:|---|
| rejected/pre-final-rust-native/ | 54 PNG | Captured from an earlier Rust executable before the final RB-07 build; superseded by rust-final frames. |
| rejected/python-job-original-1m/ | 10 PNG | The smaller 1,002,252-face fixture finished too quickly for reliable long-running Cancel timing; superseded by the 1,920,000-face paired run. |
| rejected/rust-heavy-job-theme-switch-failure/ | 22 PNG | Theme switch changed focus/state during the operation; includes busy/duplicate frames that did not show the asserted state, a Cancel frame after Smooth completed, and mislabeled operation frames. Raw results are retained beside them. |
| rejected/rust-light-subdivide-cancel-attempt3-raw/ | 10 PNG | Captures did not prove Light cancellation; some showed Imported before the operation began and later completion/limit error. Verified Light completion and limit-failure frames were relabeled and accepted separately. |
| rejected/native-subdivide-cancel-attempt1/ | 10 PNG | Early Subdivide cancellation attempts did not reach a confirmed Cancel terminal state. |
| rejected/rust-light-job-1m-attempts/ | 0 PNG | Raw attempt notes only; the mesh completed before the Cancel observation. |
| Root rejected images | 36 PNG | Wrong operation/theme, stale executable, obscured frame, or captured completion instead of the claimed action. |
| Other root journals/logs | 20 files | Retained run output and raw capture journals for audit; not accepted frames. |

The only accepted Rust native Cancel terminal is Dark. Python Light and Dark show confirmed cancellation. Rust Light still has accepted progress, retry, completion, and controlled failure frames, but its Cancel terminal is not claimed. The raw attempts and helper scripts remain for audit; they do not represent additional accepted evidence.

