# MYOS

MYOS is a Windows XP-inspired x86-64 desktop operating system project being developed in incremental milestones.

## Current milestone

Milestone 11: Native Calculator application.

The kernel mounts existing MyOS volumes over UEFI Block I/O. New volumes use filesystem format v2 with nested directory metadata; existing v1 volumes remain readable and writable as flat volumes and are never upgraded or formatted automatically. Explorer navigates folders, creates folders and files, performs current-folder CRUD, and supports copy/paste across folders. Opening a nested file in Notepad carries its directory context into Open, Save, and Save As. Explorer and standalone Notepad start at the volume root. UEFI build, filesystem tests, and QEMU verification of nested open/save, parent navigation, and cross-folder paste pass.

## Project status

- Boot foundation: complete
- Graphics foundation: framebuffer wallpaper rendering implemented; QEMU visual validation pending
- UEFI keyboard input: Space/Enter activation verified in QEMU
- Mouse input: UEFI pointer protocols plus a polled PS/2 fallback; relative motion amplified 4x, window drags repaint only damaged bounds; QEMU-injected and physical host-mouse movement/clicks confirmed
- GUI framework: in progress; geometry, buttons, and GOP painting implemented
- Window manager: in progress; core state and keyboard-operated window demo implemented
- Desktop: in progress; wallpaper and working Window Demo launcher implemented
- Taskbar/Start menu: in progress; live window buttons, UEFI clock, and Window Demo, Notepad, and File Explorer launchers implemented
- Filesystem: in progress; no_std block filesystem supports flat v1 compatibility and v2 nested directories with per-directory file operations; host remount tests pass and kernel mounts v2 AHCI volumes
- Notepad: in progress; ASCII editing, Open/Save/Save As dialogs, overwrite and unsaved-edit confirmation, and QEMU-verified persistence implemented
- Explorer: in progress; folder navigation, current-folder operations, cross-folder file copy, and path-aware Notepad Open/Save implemented and QEMU-verified on a v2 AHCI image
- Calculator: in progress; Start-menu launcher, mouse/keypad UI, and checked four-function decimal arithmetic implemented; keyboard and mouse arithmetic flows QEMU-verified
- Other applications: planned

## Build

See [BUILD.md](BUILD.md) for the current commands.

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md).

## Roadmap

See [ROADMAP.md](ROADMAP.md).
