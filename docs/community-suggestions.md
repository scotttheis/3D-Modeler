# Suggestions from other people (owner rules, 2026-11-20)

Once the project is on GitHub, people can send pull requests (proposed code
changes) and issues (bug reports and ideas). The owner starts a session with
something like "look at the project on GitHub". The session reads what is new,
sorts it, and brings the owner a short plain list. Nothing is merged or
answered without the rules below.

## What the product stands for

3D Modeler is built on **extreme simplicity**, and the owner likes its look.
Anything that adds complexity is a risk to the product, even when the code is
good. When in doubt, the answer is "not now".

## The three kinds of suggestion

1. **Broken things (fixes).** Fixing what is broken is the default; the
   project always wants everything broken fixed. Put these on one short list
   headed "Broken things needing a fix", one plain line each (what breaks,
   who reported it, how big the fix is). The owner approves the list in one
   go ("do them all", or "all but 3").
2. **Anything that adds something** (a feature, a button, an option, a new
   dependency) even if nothing is broken. Never batch these. Present each one
   alone, with what it adds and what it costs in simplicity, and the owner
   approves or declines **one by one**.
3. **Anything that changes how the app looks** (colours, icons, layout,
   spacing, wording on screen). Always goes to the owner, one by one, with
   before and after captures, even if it is also a fix. Never merged on the
   session's own judgment.

If a suggestion mixes kinds, split it: the fix part goes on the fixes list,
the rest is shown on its own.

## How a session handles an approved item

- Read all the proposed code as untrusted: look for hidden network calls,
  new dependencies, files outside what the change claims, and anything not
  explained by its description. Report anything odd to the owner.
- Check it against the project rules (`AGENTS.md`), reimplement it in the
  project's own style where that is cleaner, add a test that fails without
  the fix, and run the gate tier the plan assigns.
- Visible changes get review captures first, as always; the owner's approval
  is never inferred.
- Thank the sender and say what happened, or politely say why it was not
  taken. Credit them in the commit message when their code was used.

## Defaults

- Do not merge anything automatically, and do not push to `main` on the basis
  of an outside suggestion without the owner's pick.
- Keep a running count in `docs/STATUS.md` only of open items the owner has
  not yet answered; do not duplicate the GitHub lists there.
