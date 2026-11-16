# Uploading 3D Modeler to GitHub

Owner goal (2026-11-16): a **public** GitHub repository. Not uploaded yet;
the owner says when. Until then the "no push" rule in `AGENTS.md` stands.

## Decided

- Public repository, free for others to use; not sold. The owner will turn
  on GitHub Sponsors (donations).
- Repository and product name: **3D Modeler**.
- Commit email (owner, 2026-11-16): `scotttheisw@gmail.com` (confirm the
  spelling once; it must be added to the owner's GitHub account). Existing
  commits are rewritten to it at upload time, as the owner asked.
- How to mention Microsoft 3D Builder (agreed approach, not legal advice):
  say it plainly and factually, never as a brand. README wording:
  "A free, open-source 3D modeling app for 3D printing, inspired by
  Microsoft 3D Builder." plus, at the end, "3D Modeler is an independent
  project. It is not affiliated with, endorsed by or sponsored by Microsoft.
  3D Builder is a trademark of Microsoft Corporation." Do not use "3D
  Builder" in the app name, icon, logo, window title or repository name, and
  do not ship any Microsoft artwork, icons or screenshots.

## Open (ask the owner at upload time; do not guess)

1. **Licence.** Session recommendation: Apache 2.0 (MIT is also fine).
   Owner decides just before the upload.
2. **Artwork origin.** Confirm the icons in
   `crates/modeler3d-app/assets/original` are the owner's own (made by them
   or generated for them), not copied from 3D Builder or other apps.

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
