# Building MYOS

## Required tools

- Rust stable toolchain
- `rustup` for the `x86_64-unknown-uefi` target
- QEMU for testing
- QEMU's x86-64 EDK2/UEFI firmware image

## Install Rust

```powershell
curl.exe -L -o $env:TEMP\rustup-init.exe https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe
$env:TEMP\rustup-init.exe -y --default-toolchain stable
$env:USERPROFILE\.cargo\bin\rustup.exe default stable
$env:USERPROFILE\.cargo\bin\rustup.exe target add x86_64-unknown-uefi
```

## Build the kernel

```powershell
cd C:\Users\obarn\Desktop\MYOS
cargo build --workspace --target x86_64-unknown-uefi
```

The kernel build decodes the root-level `canvas.png` with a host-side PNG build dependency and embeds its pixels in the EFI executable. At boot, it scales the image to cover the active display while preserving its aspect ratio; replacing `canvas.png` and rebuilding updates the wallpaper.

Run the GUI-core interaction tests on the host with:

```powershell
cargo test -p myos-gui-core
```

Run filesystem block-volume tests with:

```powershell
cargo test -p myos-filesystem
```

Run the host-image formatting and persistence test with:

```powershell
cargo test -p myos-filesystem --features host-tools
```

The filesystem core uses a fixed 512-byte block interface and a small MyOS-specific format. New images use v2, which supports nested directories and file operations within a directory without heap allocation. Existing v1 volumes remain mountable and their root-level files remain readable/writable, but v1 does not support directories. There is no automatic in-place upgrade: keep v1 images unchanged and use a separately created v2 image when testing directory APIs. The kernel never formats an unrecognized disk.

Create a new, dedicated raw image for QEMU with the host utility. It uses exclusive file creation and refuses to overwrite an existing path:

```powershell
$data = "$env:TEMP\myos-data.img"
cargo run -p myos-filesystem --features host-tools --bin mkmyosfs -- $data 131072
```

Attach it to the QEMU invocation as an AHCI disk:

```powershell
-device ich9-ahci,id=sata `
-drive "if=none,id=myos-data,format=raw,file=$data" `
-device ide-hd,drive=myos-data,bus=sata.0
```

The kernel mounts only a valid MyOS signature on a compatible whole-disk UEFI Block I/O device. It does not format unknown media. The filesystem image utility is a host tool; it is not an in-OS formatter.

The desktop has a taskbar clock and window buttons. Press `F2` to open Start, use arrows to select `Window Demo`, `Notepad`, `File Explorer`, or `Calculator`, and press `Enter` or `Space` to launch the selection; `Escape` closes Start. In the demo, use `Tab` to switch focused windows, the arrow keys to move the active window, `Enter` to maximize/restore, `Escape` to close it, and `F1` to open another test window. `Space` activates the test button when its window is focused.

Start > File Explorer opens at the mounted MyOS volume root and shows files and folders. Use Up/Down to select an item, Enter or double-click to open a folder, and double-click a file (or choose Open from its context menu) to load it in Notepad. The Up button or Backspace returns to the parent; the current path appears above the list. F2 creates a file, F4 or Folder creates a folder, F3 renames the selected item, and Delete opens a confirmation. F6 copies a file and F7 pastes it into the current folder with a suggested unique `COPY_` name; paste works across folders and never replaces an existing file. Right-click a listed item for Open, Rename, Delete, Copy, Paste, and Properties. Properties identifies files and folders and reports file size. All file/folder operations are scoped to the current folder. On format-v1 volumes, existing root files remain available, but folders are unsupported. Explorer and standalone Notepad start at root; when Notepad opens a nested file from Explorer, its Open, Save, and Save As operations use that file's folder. If the status says `No MyOS filesystem mounted`, attach a dedicated MyOS data image using the AHCI options below; Explorer does not format disks or create a filesystem automatically.

Relative mouse motion is amplified 4x. Window dragging repaints only the old/new window region instead of redrawing the full desktop.

In Notepad, type ASCII text; Enter adds a newline, the arrow keys move the caret, Home/End move to the current line's boundaries, Backspace removes the previous character, and Delete removes the next one. `F5` opens a file picker scoped to the current folder; use Up/Down and Enter to open a listed file, or Escape to cancel. `F6` saves the current file. `F7` opens Save As: the current name is prefilled, Backspace edits it, and Up/Down can copy a listed filename into the name field. Enter saves a new name; if it already exists, Enter confirms overwrite and Escape returns without replacing it. Opening another file or closing Notepad while there are unsaved edits requires confirmation. The Open, Save, and Save As toolbar buttons are clickable, as are listed files and dialog buttons. A Notepad window opened from Explorer inherits that file's folder; a standalone window starts at the volume root. Text editing is ASCII-only.

Save requires a mounted MyOS filesystem. If QEMU was started without the dedicated MyOS data disk, the Notepad status line now reports `Save failed: no MyOS filesystem mounted`; attach a formatted image using the AHCI options below and reboot.

Start > Calculator supports digit entry, decimal points, `+`, `-`, `*`, `/`, Enter/`=`, `C`, and Backspace from the keyboard or on-screen keypad. It evaluates chained operations left-to-right, reports divide-by-zero and overflow, and uses six decimal places for input/results (division truncates beyond six places). QEMU verified the Start-menu launch, keyboard calculation `12.5 + 0.5 = 13`, and mouse-keypad calculation `1 + 2 = 3`.

## Run in QEMU on Windows

The Chocolatey QEMU package installs its executable and x86-64 firmware under `C:\Program Files\qemu`. Stage the EFI application at the UEFI fallback boot path on a temporary FAT-backed drive:

```powershell
$qemu = "C:\Program Files\qemu\qemu-system-x86_64.exe"
$firmware = "C:\Program Files\qemu\share\edk2-x86_64-code.fd"
$testDir = Join-Path $env:TEMP ("myos-" + [guid]::NewGuid().ToString("N"))
$vars = Join-Path $testDir "vars.fd"
$esp = Join-Path $testDir "esp"
$serial = Join-Path $testDir "serial.txt"
New-Item -ItemType Directory -Force (Join-Path $esp "EFI\BOOT") | Out-Null
Copy-Item "C:\Program Files\qemu\share\edk2-i386-vars.fd" $vars
Copy-Item ".\target\x86_64-unknown-uefi\debug\myos-kernel.efi" "$esp\EFI\BOOT\BOOTX64.EFI" -Force
$builtHash = (Get-FileHash ".\target\x86_64-unknown-uefi\debug\myos-kernel.efi" -Algorithm SHA256).Hash
$stagedHash = (Get-FileHash "$esp\EFI\BOOT\BOOTX64.EFI" -Algorithm SHA256).Hash
if ($builtHash -ne $stagedHash) { throw "Staged EFI does not match the build." }
Write-Host "Staged current kernel: $stagedHash"
$firmwarePath = $firmware -replace '\\', '/'
$varsPath = $vars -replace '\\', '/'
$espPath = $esp -replace '\\', '/'
& $qemu -machine q35 -m 1G `
	-drive "if=pflash,format=raw,unit=0,readonly=on,file=$firmwarePath" `
	-drive "if=pflash,format=raw,unit=1,file=$varsPath" `
	-drive "format=raw,file=fat:rw:$espPath" `
	-display sdl `
	-monitor stdio `
	-serial "file:$serial" -net none
```

This QEMU package omits an x86-64 variable-store template. The command uses its same-sized i386 EDK2 variable-store template as a temporary copy; the installed package files remain untouched. The example creates a fresh ESP, variable store, and serial log for each run, then verifies that the staged EFI SHA-256 exactly matches the freshly built kernel. This avoids accidentally booting an older EFI from a previous QEMU run. QEMU's built-in PS/2 mouse is the input device for this test; it forwards the host mouse into the virtual PS/2 controller, which MyOS polls as a fallback when firmware pointer protocols do not deliver events. Do not add a USB tablet for this test. Focus the QEMU display; press `Ctrl+Alt+G` to grab the mouse if needed, then move and click inside the VM. MyOS should show its custom cursor after the first movement or click. Press `F2` to verify keyboard input independently. After closing QEMU, inspect `$serial`: `PS/2 mouse fallback initialized.` confirms the driver started, and `Mouse input received.` confirms delivered movement or button events.

The command above intentionally has no data disk, so it can be used directly for mouse testing. To test Notepad persistence as well, create a dedicated MyOS data image using the earlier `mkmyosfs` command, then add these options to the QEMU invocation (after defining `$data` to the image path):

```powershell
-device ich9-ahci,id=sata `
-drive "if=none,id=myos-data,format=raw,file=$data" `
-device ide-hd,drive=myos-data,bus=sata.0
```

## Current verification status

Filesystem host tests cover nested directory creation, per-directory name isolation and file operations, cross-directory copy, rename, empty-directory deletion rules, stable directory handles after slot reuse, v1 read/write compatibility, and nested data after image remount. The UEFI kernel build and QEMU mount check cover v2 volumes. A QMP-controlled QEMU run with a dedicated v2 AHCI image created a folder and nested file through Explorer, entered and left the folder, opened the file in Notepad, saved `hi` in that folder, and pasted a copy back to the volume root. The host inspected the raw image and verified both files contain the exact saved bytes.

A separate QEMU test with a mounted AHCI image verified right-clicking a listed Explorer file, opening its context menu, launching Properties, and dismissing the properties view with Escape.

The workspace builds for `x86_64-unknown-uefi`; GUI-core tests cover Start selection and bounded editor operations, and filesystem tests cover host-image persistence, multi-block copying, remounts, and filesystem errors. QEMU boot with a dedicated AHCI MyOS image printed `MyOS filesystem mounted from UEFI block storage.` A two-boot QMP test created `test.txt` with Save As, opened it after reboot, exercised unsaved-edit cancellation, and confirmed an overwrite; the host then verified the exact persisted contents and directory entry. QEMU also verified Explorer create, rename, confirmed delete, and copy/paste; the host inspected the raw disk image and confirmed the expected entries persisted without removing an unrelated file. A separate QEMU test confirmed that discarding an unsaved edit on close and reopening Notepad restored the last saved text. QEMU-injected relative movement and a click through the built-in PS/2 mouse reached MyOS's polled fallback (`Mouse input received.`). Physical mouse input has also been confirmed by the user; taskbar pointer hit-testing remains to be checked.
