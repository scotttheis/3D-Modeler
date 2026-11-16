# Uploading 3D Modeler to GitHub

Owner goal (2026-11-16): a **public** GitHub repository. Not uploaded yet;
the owner says when. Until then the "no push" rule in `AGENTS.md` stands.

## Decided

- Public repository.
- The licence is chosen by the owner just before the upload. Do not pick one.

## Open (ask the owner at upload time; do not guess)

1. **Commit email.** All commits so far carry `josiph@byui.edu`, which a
   public repository shows. Owner may use any email added to their GitHub
   account, or GitHub's private no-reply address (Settings → Emails → "Keep
   my email addresses private"). Changing the existing commits means
   rewriting history; do it only with the owner's explicit yes.
2. **Licence** (see above).
3. **Repository name and account.** The owner creates the empty repository or
   asks the session to create it.

## Things that become public

- The app artwork in `crates/modeler3d-app/assets` (about 40 MB).
- The Windows username and drive letters in some notes (`docs/STATUS.md`,
  `docs/owner-feedback-2026-11-14.md`, a few files under `evidence/`).
- Test models up to 30 MB in Git history (under GitHub's 100 MB file limit).
- Checked 2026-11-16: no passwords, keys or tokens are stored in the tree.
  The MCP bridge secret is generated fresh at each start.

## Steps at upload time

1. Ask the open questions above; record the answers in `docs/STATUS.md`.
2. If the owner approves an email change, rewrite the author email
   (owner-approved history rewrite) before the first push, after a `G:\`
   snapshot.
3. Add `LICENSE` (owner's choice) and a short `README.md` (what the app is,
   how to build: `docs/building.md`, how to run).
4. Run the Full gate (`scripts/Invoke-Gate.ps1 -Tier full`).
5. Sign in: the owner types `! gh auth login` in the session and follows the
   browser prompt. The session never sees the password; the owner can sign
   out afterwards with `! gh auth logout`.
6. Fast-forward `main` to `rebuild/work` (`main` is behind, nothing to merge),
   add the remote and push `main`. Update the "no push" rule in `AGENTS.md`
   to say what is now allowed.
