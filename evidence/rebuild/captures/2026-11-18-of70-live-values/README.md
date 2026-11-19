# OF-70 - Live Move and Rotate numbers

Task: [`docs/tasks/move-rotate-live-values.md`](../../../../docs/tasks/move-rotate-live-values.md).

| File | What it shows |
|---|---|
| `before-rotate-drag-{light,dark}.png` | Old build (board `gate-20261118-120020`): cube turned 40 degrees mid-drag, boxes still read 25 / 35 / 20 deg. |
| `after-rotate-drag-{light,dark}.png` | Same staged drag: X box reads 65 deg while the pointer is held. |
| `after-move-drag-{light,dark}.png` | New state `move-drag`: cube held 25 mm along +X, X box reads 25 mm. |
| `release-*-{light,dark}.png` | The same two states from the Release build. |

There is no "before" for `move-drag`: the state is new. Before the fix the
position boxes stayed at the starting value during a Move drag, exactly as the
rotate boxes above.

Images only; owner review pending.
