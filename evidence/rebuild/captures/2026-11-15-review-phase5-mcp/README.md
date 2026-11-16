# Review pack: owner batch 7, Phase 5 — the AI can do everything (2026-11-15)

OF-58. Nothing new is drawn on the screen in this phase: the change is what
an AI connected to Builder3D can do. The three pictures here were **taken by
the AI itself** with the new `capture_window` tool, on the release build
(`target\release\builder3d-app.exe`), while it changed the app's settings.
`of58-live-mcp-log.json` is the full record of that run (17 of 17 checks
passed, ending with the AI closing Builder3D).

| File | What it shows |
|---|---|
| `of58-light-object-tab.png` | The AI switched to the light theme and inches, added Mirror to the favourites strip (top row, fifth icon), and opened the Object tab. |
| `of58-dark-about.png` | The AI switched back to dark and opened Settings and About. Settings shows "in" because the AI set it. |
| `of58-dark-start.png` | The AI went to the start screen; Settings closes on the way, as it does when you click outside it. |

## What to try

Open Builder3D, connect your AI as usual, and ask it things like:

1. "Switch Builder3D to light mode" / "make everything big" / "show sizes in inches".
2. "Add Mirror to my favourites" / "put Mirror first on the Object tab" / "reset my ribbon".
3. "Turn on sticky selection" / "pin the ribbon open" / "make the floor shadow stronger".
4. "What are my recent files?" / "open the second one" / "clear my recent files" (the files themselves are kept).
5. "Go to the start screen" / "show me the About box".
6. "Take a picture of the Builder3D window" (it saves into a folder the AI is allowed to write to).
7. "Close Builder3D". With unsaved work it will not close unless you tell it to discard, or it shows you the usual Save / Don't save / Cancel question.
