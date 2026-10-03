# MyOS Architecture

## Overview

The project is organized as an incremental operating-system architecture with a clear separation between boot, kernel, GUI, filesystem, applications, and documentation.

## Milestone 1 foundation

The first milestone established the boot path from UEFI into a minimal x86-64 kernel context.

Completed responsibilities:

- carry the kernel entry point through UEFI
- print boot diagnostics to the UEFI console
- prepare a framebuffer abstraction
- expose a planned memory abstraction
- halt cleanly on unrecoverable faults

## Milestone 2 foundation

The second milestone adds the graphics output protocol to the kernel workflow.

Current responsibilities:

- locate the GOP handle and open the graphics protocol
- choose the closest supported display mode to the desired resolution
- activate the display mode
- decode and embed the root-level `canvas.png` at build time
- scale and center-crop the wallpaper into the active display mode
- keep debug output active while the graphics pipeline is initialized

## Milestone 3 foundation

Keyboard polling uses UEFI standard input. Pointer polling checks UEFI relative and absolute pointer protocols, then falls back to a polled PS/2 auxiliary-port driver when firmware does not deliver reports. Relative deltas are amplified 4x. This is early firmware/legacy-controller input, not a native USB HID driver.

## Milestone 4 foundation

- `gui-core` is a `no_std` crate containing reusable points, rectangles, color, and button state transitions.
- `kernel/src/gui.rs` provides a GOP painter with clipped rectangle fills, 8x8 bitmap text, and beveled button rendering.
- The kernel displays a centered test button that updates its label on Space, Enter, or pointer click.
- The software pointer cursor saves and restores the pixels beneath it, and appears after movement or button input is received.
- Host tests cover rectangle boundaries, pointer capture/click behavior, and disabled buttons.

The GUI framework is in progress. Keyboard and PS/2 mouse input have been verified in QEMU. Native USB HID drivers, windows, panels, focus navigation, taskbar, Start menu, and applications are not implemented.

## Milestone 5 foundation

- `WindowManager<N>` stores a fixed number of windows without heap allocation.
- Windows have stable IDs, bounds, visibility state, focus, and z-order.
- The manager supports hit-testing, title-bar dragging, moving/resizing, minimize, maximize, restore, and close.
- Dragging redraws only the union of the old and new window bounds, restoring the wallpaper and any intersecting content beneath.
- The kernel paints overlapping window frames and routes keyboard actions for focus, movement, maximize/restore, close, and reopening a test window.
- Host tests cover z-order, focus cycling, minimize fallback, restore bounds, and dragging.
- QEMU keyboard testing verified movement, focus switching, button activation, maximize/restore, close, and reopen.

Window mouse input is wired to the manager and receives QEMU PS/2 events. A native USB mouse driver, resize handles, multiple real applications, and a desktop shell remain future work.

## Milestone 6 foundation

- The kernel begins on the wallpaper desktop with a selected `Window Demo` shortcut.
- The shortcut reuses the GUI button state model for selection and pointer activation.
- Enter or Space launches the window-manager demo; F1 reopens it after all windows are closed.
- QEMU verified the initial desktop frame and the Enter-to-launch transition.

The desktop is not yet a full shell: it has no desktop icon grid, context menu, taskbar, Start menu, system tray, or user applications.

## Milestone 7 foundation

- The bottom taskbar shows a Start control, visible-window buttons, and a clock read from UEFI runtime time.
- F2 toggles the Start menu, which contains Window Demo and Notepad launchers.
- The Start menu launches both Window Demo and the filesystem-backed Notepad application.
- Arrow keys move menu selection; Enter or Space activates the selected item; Escape closes the menu before affecting a window.
- QEMU verified opening the menu, launching Window Demo, and closing the menu while leaving the launched window active.
- Pointer release hit-testing is implemented for Start, menu entries, and taskbar window buttons; end-to-end validation of those shell controls remains pending.

This is an early shell slice: no shutdown action, pinned apps, or system tray services are exposed.

## Milestone 8 foundation

- `filesystem` is a separate `no_std` crate with a 512-byte `BlockDevice` interface.
- The on-disk format uses a superblock, fixed-capacity directory-entry table, and contiguous extents for file data.
- Format v2 stores an entry kind and parent-directory slot in each record. Directory IDs are stable entry slots; an empty directory must be deleted before its slot can be reused.
- The filesystem API supports nested directory listing/creation/rename/removal and file list/read/create/overwrite/rename/copy/delete within any v2 directory without heap allocation.
- Format v1 remains mountable and root files remain readable and writable. Directories are unavailable on v1 volumes; the kernel never upgrades or formats an existing volume implicitly.
- Host tests use a memory-backed block device and verify data after remount.

This filesystem is a small MyOS-specific format, not FAT. The kernel discovers UEFI Block I/O devices with 512-byte blocks and mounts a volume only when its MyOS signature and metadata validate. It skips logical partitions, read-only media, unsupported block sizes, and unsupported I/O alignment. It never formats an unrecognized disk. QEMU boot verified that a dedicated AHCI image is mounted.

## Milestone 9 foundation

- `gui-core` provides a fixed-capacity ASCII text buffer with bounded insertion, replacement, and backspace operations.
- The Start menu launches a Notepad window which edits a 2 KiB in-memory document.
- F5 opens a fixed-capacity file picker backed by `FileSystem::list`; F6 saves the current file, and F7 opens Save As.
- Save As validates names through the filesystem and requires confirmation before replacing an existing file. Opening another file or closing Notepad prompts before discarding dirty edits.
- File-picker rows and toolbar/dialog controls support pointer input; arrow keys and Enter/Escape provide keyboard selection and confirmation.
- Filesystem, storage-unavailable, invalid-name, and unsupported-text failures are surfaced in the window status and UEFI log.
- The initial editor supports ASCII keyboard input and caret navigation; Unicode, folders, and mouse text selection are not implemented.
- A two-boot QEMU test saved a new named file, reopened it after reboot, cancelled an unsaved-change prompt, and confirmed an overwrite; the test verified the exact file contents on disk. A separate QEMU run confirmed that closing with unsaved edits and reopening the app restores the last saved contents.

## Milestone 10: hierarchical Explorer

- Explorer uses the filesystem's fixed-capacity directory listing and stores a bounded directory-ID stack and path for navigation without heap allocation.
- Filesystem v2 supports folder creation, rename, empty-folder deletion, and per-directory file operations. Copy/paste supports a source folder distinct from the destination folder.
- Explorer displays file/folder metadata, enters folders through Enter, double-click, or Open, and returns to the parent through Up or Backspace. Folder operations are scoped to the current directory.
- Notepad's dialog records its directory context, and its Open, Save, and Save As operations use that directory. Explorer-launched Notepad is initialized from the selected item's folder; standalone Notepad remains at the volume root.
- Filesystem v1 remains root-only and read/write compatible; it is never migrated or formatted implicitly. Directory actions report that v2 is required.
- When no MyOS filesystem is mounted, Explorer shows an explicit no-volume state and does not attempt to format or write to other disks.
- QEMU verified folder creation, folder entry and parent navigation, nested file opening and saving in Notepad, and cross-folder paste on a dedicated v2 AHCI image; the host verified the exact saved bytes in both files. Filesystem host tests also verify nested and cross-directory operations survive remount.

## Milestone 11: Calculator

- `gui-core::Calculator` owns fixed-capacity ASCII display/input state and checked fixed-point arithmetic, keeping numeric behavior independent of UEFI and GOP.
- The display uses a scale of 1,000,000: input and results support six fractional decimal places, and division truncates beyond that precision. Arithmetic overflow and division by zero produce explicit display errors.
- The kernel launches Calculator from Start, renders an on-screen keypad, routes keyboard and mouse keys through the same model, and allows a fifth concurrent window.
- Host tests cover decimal arithmetic, chaining, negative results, division truncation and zero, input limits, clear/backspace, and overflow. QEMU verified Start-menu launch, keyboard evaluation of `12.5 + 0.5 = 13`, and mouse-keypad evaluation of `1 + 2 = 3`.

## Planned subsystem layering

```text
UEFI firmware
  ↓
MyOS kernel entry
  ↓
Kernel runtime
  ├── GOP framebuffer and wallpaper renderer
  ├── UEFI input polling
  ├── UEFI Block I/O discovery and MyOS volume mounting
  └── myos-gui-core window manager + GOP painter
       ↓
  ├── wallpaper desktop and Window Demo launcher
    ├── taskbar, live window buttons, and UEFI clock
    ├── Start menu with functional Window Demo entry
    ├── myos-filesystem block interface and file store
    ├── Notepad dialogs and documents using the mounted filesystem
    └── flat-file Explorer managing documents and opening them in Notepad
```

## Notes

This repository intentionally avoids claiming full kernel functionality before the foundation is verified. The goal for the first milestone is correctness and a stable boot path, not an oversized feature set.
