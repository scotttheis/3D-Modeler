# Changelog

All notable changes, in plain words. Newest first.

## Unreleased

## 1.0.6 (2026-11-24)

- **Report a problem from inside the app.** Settings has a new **Report a
  problem** button, and error messages have a **Report** button. Say what went
  wrong and, if you like, how it could be fixed. 3D Modeler adds its version,
  your system and its recent errors (your home folder is hidden, and no models
  or files are included), shows you everything, and opens a filled-in report
  on GitHub for you to check and post. Nothing is sent without you.
- **After a crash, 3D Modeler offers to report it** the next time it starts.
- **Errors are now written to the app's log**, so a report says what actually
  went wrong.
- **AI assistants can prepare a problem report too** (new MCP tool
  `report_problem`); you still post it yourself.
- **Repair keeps the fixes it can make.** When a model can't be fully
  repaired, Repair now keeps every fix that helps and tells you what it
  couldn't fix, instead of changing nothing.
- **The bottom bar is easier to use.** The size and position boxes show plain
  numbers, one click selects the whole value, and Enter applies it.
- **The title bar matches the app.** In dark mode it stays dark on every
  Windows version, including after an update.
- **Fixed a crash** when another program stretched the window larger than the
  graphics card can draw.
- **No more false update offers** on a copy built from the source code.
- **Double-clicking a file opens it.** Double-click a `.3dp` project and it
  opens in 3D Modeler, ready to work on. Before, the app started empty.
- **3D Modeler shows up in Windows.** It is listed under "Open with" for STL,
  3MF, OBJ, PLY, glTF/GLB and WRL files, and in Settings > Apps > Default apps,
  so you can make it the program that opens them. `.3dp` files open in it
  automatically.

## 1.0.5 (2026-11-24)

- **Models from other programs open the right way up.** STL, 3MF, OBJ and PLY
  files from 3D Builder, slicers and most 3D-printing tools now open standing
  the same way they do there, instead of tipped onto their side. Files saved
  from 3D Modeler in these formats also open the right way up in those
  programs. Your `.3dp` projects are not affected.

## 1.0.4 (2026-11-24)

- **Repair is much better.** On Windows, Repair now uses the advanced
  3D-printing repair engine built into Windows. Mac and Linux don't have it,
  so 3D Modeler uses its own repair there, which is also much improved: it
  closes holes of any size, joins overlapping parts into one solid and keeps
  hollow models hollow. On badly damaged models it may not be as thorough as
  the Windows engine.
- **Repair never makes a model worse.** If it can't improve a model, it says
  so and leaves it unchanged. When it works, it tells you what it fixed in
  plain words.
- **Home screen:** a new Open File button opens an STL, 3MF or other model
  straight from the home page.
- The unfinished Emboss button is hidden until it is ready.

## 1.0.3 (2026-11-22)

- **The AI (MCP) gateway now comes inside the Windows, Mac and Linux installers,**
  next to the app, so there is nothing extra to build.
- **Paste goes where the original was.** If you move a model after copying it,
  the pasted copy appears at the spot it was copied from. A model you have not
  moved still pastes beside itself, as before.
- **Settle drops straight down.** It keeps the model where it is and only lowers
  it until it rests on the floor, instead of sending it back to the middle.
- **Save opens the Windows save window straight away**, with a "Save as type"
  list (STL or 3MF), instead of showing a dialog of ours first. Cancel simply
  closes it. Export is unchanged.

## 1.0.2 (2026-11-21)

- **Subtract is simpler.** Select one object and click Subtract: it cuts away
  everything it touches, then disappears. No second click needed. If nothing is
  touching it, nothing changes and the status says so.
- **Home screen:** click a recent project's tile to open it. The separate Open
  button is gone.
- **Save** now saves an opened STL or 3MF file back to where it came from.
  Save As is for a new file.
- **Side bar:** a status line at the bottom shows what the app is doing, with
  Abort while a job can be stopped. The Items list has a new row layout: icon,
  name, a warning mark for models needing repair, and the show/hide eye.
- Left-dragging in the view works during Subtract; a plain click picks.
- Boolean operations write clearer entries to the log file.

## 1.0.1 (2026-11-21)

- Merge, Subtract and Intersect run in the background with a progress area and
  an Abort button, so the window no longer freezes while they work.
- Faster mesh checks and fewer redraws of unchanged objects.
- New users start with an empty shortcut bar.

## 1.0.0 (2026-11-21)

- First full release for Windows, macOS and Linux.
- Smooth Brush default size raised so strokes feel stronger.
- Whole-object Smooth keeps sharp edges and corners instead of melting shapes.
- The Hand is the default grab cursor (the Claw is one click away in Settings).
- Resize handles no longer pile up when zoomed far in.
- Subtract now works on nearly perfect meshes (for example, with one empty triangle).

## 0.99.0 (2026-11-20)

- First private test release, with installers for all three systems and
  built-in updates. Installer icons are shrunk to 128 px (downloads of 13 to
  26 MB).
