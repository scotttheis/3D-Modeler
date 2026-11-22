# Contributing to 3D Modeler

Thank you for helping. 3D Modeler is built on **extreme simplicity**: a person
should be able to use it without learning it first. That rule decides most
questions, and when in doubt the answer is "not now".

## What is welcome

1. **Fixes for broken things.** Always welcome. Open an issue (use the bug form)
   or a pull request. These are reviewed together and usually accepted quickly.
2. **Additions** (a feature, button, option or dependency), even when nothing is
   broken. Open an idea issue first and say what it adds and **what it costs in
   simplicity**. Each one is decided on its own; please do not send code before
   the idea is accepted.
3. **Changes to how the app looks** (colours, icons, layout, spacing, words on
   screen). Decided one by one by the maintainer with before and after
   pictures, even when the change is also a fix.

If your change mixes kinds, please split it.

## Reporting a bug

Use the **Bug report** form. It asks for your version, your system and the log
file, `modeler3d.log`, found here:

- Windows: `%LOCALAPPDATA%\Modeler3D\logs`
- macOS: `~/Library/Logs/Modeler3D`
- Linux: `~/.local/state/Modeler3D`

Remove anything private before attaching it.

## Sending code

- Build and test as described in [`docs/building.md`](docs/building.md).
- Before opening a pull request run `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings` and `cargo test --workspace`.
- The rules the code follows are in [`AGENTS.md`](AGENTS.md): no `unwrap()` in
  library code, every model change is one undoable command, and the desktop and
  the MCP server stay equal (anything the desktop can do, the MCP can too).
- Keep pull requests small and say plainly what they change.
- All proposed code is read as untrusted: new dependencies, network calls and
  unexplained files will be questioned.

How suggestions are handled is described in
[`docs/community-suggestions.md`](docs/community-suggestions.md).

## Conduct

Please follow the [Code of Conduct](CODE_OF_CONDUCT.md).
