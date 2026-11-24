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

The easiest way is inside 3D Modeler: **Settings > Report a problem**, or the
**Report** button beside an error message. Say what went wrong and, if you
like, how it could be fixed. The app adds its version, your system and the
recent errors from its log (your home folder is hidden, and no models or files
are included), shows you all of it, and opens a filled-in **Bug report** on
GitHub for you to check and post. The app never sends anything by itself. If
3D Modeler hit an internal error, it offers this the next time it starts.

You can also fill in the **Bug report** form by hand. It asks for your
version, your system and the log file, `modeler3d.log`, found here:

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
