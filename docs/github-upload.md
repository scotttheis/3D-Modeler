# Uploading 3D Modeler to GitHub

Owner goal (2026-11-16): a **public** GitHub repository. Not uploaded yet;
the owner says when. Until then the "no push" rule in `AGENTS.md` stands.

## Release plan (owner, 2026-11-20)

Two stages. When the owner says "put 3D Modeler on GitHub as v0.99" (or
similar), run **Stage 1**. When they say the friend's feedback is done and
it is time for v1.00, make the requested edits as normal tasks first, then
run **Stage 2**.

**Stage 1: v0.99, private test.**

1. Create the repository on GitHub as **private** (`gh repo create
   <owner>/3D-Modeler --private`). Use the final name now: the repository is
   later switched to public under the same address, so the update feed in
   the v0.99 build already points at the right place.
2. Do the "Steps at upload time" below (gate, history email rewrite after a
   `G:\` snapshot, licence, README, `Configure-UpdateFeed.ps1`, push `main`).
3. Build installers at version `0.99.0` (`Build-Msi.ps1 -Version 0.99.0
   -Name 3D-Modeler-0.99 -Build -PublicRelease -Verify`; Linux/Mac only if
   the owner asks for them in this stage). 0.99.0 is higher than every RC
   (0.1.x), so it upgrades an RC install in place.
4. Tag `v0.99.0` and publish a GitHub **pre-release** with the installers
   attached (`gh release create v0.99.0 --prerelease ...`).
5. Give access only to the people the owner names: ask for each tester's
   GitHub username and run `gh api -X PUT
   repos/<owner>/3D-Modeler/collaborators/<username> -f permission=pull`.
   On a personal (non-organisation) account GitHub gives collaborators write
   access regardless of the permission asked for; tell the owner that in
   plain words. A tester who has no GitHub account, or who should not get
   write access, gets the MSI file directly instead (any file share).
6. Tell the owner, in plain words: what the tester does (accept the GitHub
   invitation email, open the Releases page, download the installer; Windows
   SmartScreen shows "More info, Run anyway" because the installer is not
   code-signed), and that a private repository uses GitHub's limited free
   build minutes, Mac minutes counting ten times.
7. Record in `docs/STATUS.md`: repository address, v0.99.0, testers given
   access, date.

While private, the in-app update check cannot see releases (GitHub hides
them without a login). That is expected; it starts working when Stage 2
makes the repository public, so a v0.99 tester is then offered v1.00.

**Stage 2: v1.00, public.**

1. Apply the owner's post-test edits as normal tasks (gate, captures,
   owner approval).
2. Rebuild every platform the owner wants at version `1.0.0` with
   `-PublicRelease`; run the Full gate.
3. Switch the repository to public (`gh repo edit --visibility public
   --accept-visibility-change-consequences`), after confirming with the
   owner that it goes public now.
4. Tag `v1.0.0` and publish a normal (not pre-release) GitHub release with
   the installers. Check from an installed v0.99 that Update and Restart
   offers 1.0.0.
5. Remove testers' collaborator access if the owner wants (they no longer
   need it once public). Turn on GitHub Sponsors if the owner is ready.
6. Record the public release in `docs/STATUS.md`.

## Decided

- Public repository, free for others to use; not sold. The owner will turn
  on GitHub Sponsors (donations).
- Repository and product name: **3D Modeler**.
- Commit email (owner, 2026-11-16): `scotttheisw@gmail.com`, confirmed and
  added to the owner's GitHub account. Set as this repository's `user.email`
  from 2026-11-16; earlier commits are rewritten to it at upload time, as the
  owner asked.
- Licence: **Apache 2.0** (owner, 2026-11-16).
- Artwork: the owner made every icon, light and dark (owner, 2026-11-16).
- The README and disclaimer wording below is approved by the owner; use all
  of it.
- How to mention Microsoft 3D Builder (agreed approach, not legal advice):
  say it plainly and factually, never as a brand. README wording:
  "A free, open-source 3D modeling app for 3D printing, inspired by
  Microsoft 3D Builder." plus, at the end, "3D Modeler is an independent
  project. It is not affiliated with, endorsed by or sponsored by Microsoft.
  3D Builder is a trademark of Microsoft Corporation." Do not use "3D
  Builder" in the app name, icon, logo, window title or repository name, and
  do not ship any Microsoft artwork, icons or screenshots.

## Open (ask the owner at upload time; do not guess)

1. **Linux and Mac.** Owner wants to install on a Linux box and a Mac before
   release. Plan: build and test Linux here in WSL Ubuntu first; Mac needs a
   Mac or GitHub's Mac build machines (free for public repositories), so it
   can follow the upload. An unsigned Mac app shows a security warning; the
   owner allows it once in System Settings.

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
   Configure the real GitHub update repository before the first public build
   and follow [`updates.md`](updates.md): rebuild every platform with the
   public-release guard, publish the matching installers, and exercise an
   installed-version upgrade. An unconfigured first build cannot find updates.
4. Run the Full gate (`scripts/Invoke-Gate.ps1 -Tier full`).
5. Sign in: the owner types `! gh auth login` in the session and follows the
   browser prompt. The session never sees the password; the owner can sign
   out afterwards with `! gh auth logout`.
6. Fast-forward `main` to `rebuild/work` (`main` is behind, nothing to merge),
   add the remote and push `main`. Update the "no push" rule in `AGENTS.md`
   to say what is now allowed.
