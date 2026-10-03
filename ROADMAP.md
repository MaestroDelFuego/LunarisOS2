# MYOS — WINDOWS XP-INSPIRED DESKTOP OPERATING SYSTEM

You are the primary software engineer for a long-term operating-system project called **MyOS**.

Your job is to design and implement a **real, bootable, graphical x86-64 operating system**, inspired visually and functionally by the classic Windows XP desktop experience.

This is NOT a website.

This is NOT an Electron application.

This is NOT a Python GUI.

This is NOT a visual mockup.

This must eventually boot on x86-64 hardware or in QEMU/VirtualBox and provide a genuine graphical desktop environment with persistent storage and native applications.

The project should be developed incrementally. Do not attempt to implement the entire OS in one enormous response. Build a solid foundation first and expand it subsystem by subsystem.

---

# 1. PRIMARY GOAL

The final goal is a small but genuinely usable desktop operating system.

The user should eventually be able to:

1. Boot MyOS.
2. Reach a graphical desktop.
3. Move the mouse.
4. Use the keyboard.
5. Open applications.
6. Move, resize, minimize, maximize and close application windows.
7. Use a Start menu.
8. Create and open files.
9. Save files permanently to disk.
10. Browse the filesystem.
11. Use Notepad.
12. Use a calculator.
13. Use a terminal.
14. Change basic system settings.
15. Shut down/restart the system.
16. Eventually connect to a network.
17. Eventually browse the web.

The initial development target is NOT a full Windows replacement.

The initial target is a small, stable desktop OS that feels like an old-school Windows desktop.

---

# 2. HARDWARE TARGET

Primary development hardware:

CPU:
AMD Ryzen 7 5800X

RAM:
32 GB DDR4

GPU:
NVIDIA RTX 2080 Ti 11 GB

Secondary GPU:
AMD RX 6600 8 GB

The OS must initially target standard x86-64 PCs.

Do not make the operating system dependent on the RTX 2080 Ti or RX 6600.

Graphics should initially use a generic framebuffer / VESA / UEFI-compatible mechanism so the OS can boot in QEMU and on ordinary PCs without requiring proprietary GPU drivers.

Advanced GPU acceleration can be implemented later.

---

# 3. DEVELOPMENT ENVIRONMENT

The host development machine is Windows.

Use a development workflow compatible with:

* Windows 10/11
* VS Code
* Git
* QEMU
* standard Windows build tools

The OS should be testable entirely inside QEMU before attempting physical hardware.

Every milestone should produce something that can be compiled and booted.

Never require the developer to install obscure tools without explaining exactly why they are required.

---

# 4. ARCHITECTURE

Use a modular architecture.

Separate the project into clearly defined subsystems.

Suggested structure:

```text
MyOS/

boot/

kernel/
    arch/
    memory/
    interrupts/
    processes/
    scheduler/
    ipc/
    syscalls/
    timers/

drivers/
    framebuffer/
    keyboard/
    mouse/
    storage/
    pci/
    usb/
    audio/
    network/

filesystem/

gui/
    core/
    compositor/
    windowmanager/
    widgets/
    controls/
    desktop/
    taskbar/
    startmenu/
    dialogs/
    theme/

userspace/

apps/
    explorer/
    notepad/
    calculator/
    terminal/
    settings/

libc/

tools/

build/

docs/
```

The exact directory structure can change if there is a technically better design, but maintain a clear separation between kernel, drivers, GUI infrastructure and applications.

---

# 5. LANGUAGE

Prefer **Rust** for the kernel and most OS components because memory safety is important.

Use C/C++ only where there is a compelling technical reason.

Do not introduce multiple programming languages unnecessarily.

Keep the architecture understandable to a single developer.

---

# 6. BOOT PROCESS

The OS must eventually:

1. Boot through UEFI.
2. Load the kernel.
3. Establish the CPU execution environment.
4. Detect available memory.
5. Initialize basic hardware.
6. Initialize the framebuffer.
7. Initialize keyboard input.
8. Initialize mouse input.
9. Initialize the filesystem.
10. Start the userspace environment.
11. Start the graphical desktop.

During early development, booting through QEMU is the primary requirement.

Provide serial/debug output for diagnosing boot failures.

Never remove useful debugging information simply because the graphical interface exists.

---

# 7. KERNEL

Implement the kernel incrementally.

Required eventual subsystems:

* x86-64 initialization
* GDT
* IDT
* interrupt handling
* exception handling
* physical memory manager
* virtual memory
* paging
* heap allocator
* timers
* processes
* threads
* scheduler
* IPC
* system calls
* userspace isolation
* basic permissions
* device abstraction

The kernel should not contain GUI application logic.

Keep the kernel as small and modular as reasonably possible.

---

# 8. GRAPHICS

The GUI is a major priority.

The desktop must be genuinely graphical rather than text-based.

Initial graphics target:

* 1280x720
* 1920x1080

The graphics subsystem should support:

* framebuffer access
* pixels
* rectangles
* lines
* images
* bitmap fonts initially
* later scalable fonts
* clipping
* double buffering
* dirty-region updates where useful
* mouse cursor rendering
* window rendering

Avoid unnecessary GPU complexity initially.

A software-rendered desktop is completely acceptable for the first versions.

---

# 9. WINDOW MANAGER

Implement a real window manager.

Windows must support:

* creation
* destruction
* movement
* resizing
* minimizing
* maximizing
* restoring
* focusing
* z-order
* activation
* keyboard focus
* mouse focus
* overlapping windows
* title bars
* borders
* close button
* minimize button
* maximize button

Windows must not simply be screenshots drawn onto the desktop.

Applications must communicate with the GUI framework.

---

# 10. WINDOWS XP-INSPIRED VISUAL DESIGN

The GUI should be heavily inspired by the classic Windows XP Luna-era desktop.

Use the following visual characteristics:

* blue taskbar
* green Start button
* blue window title bars
* rounded title-bar styling
* beveled buttons
* classic Windows-style controls
* desktop icons
* system tray
* digital clock
* Start menu
* traditional menus
* dialog boxes
* scrollbars
* familiar desktop metaphors

Do NOT use Microsoft's original copyrighted assets.

Create original assets that evoke the same general era and visual style.

The goal is:

"Windows XP-inspired"

not:

"pixel-perfect copy of Microsoft's proprietary assets."

---

# 11. DESKTOP

The desktop must provide:

* wallpaper
* desktop icons
* right-click context menu
* icon selection
* icon activation
* drag-and-drop icons eventually
* taskbar
* Start button
* Start menu
* system tray
* clock

The desktop should remain responsive while applications are running.

---

# 12. TASKBAR

The taskbar should contain:

LEFT:

* Start button
* pinned/quick-launch applications eventually

CENTER:

* running application buttons

RIGHT:

* system tray
* clock

Clicking an application taskbar button should activate/minimize the corresponding window.

The taskbar should update dynamically when applications open and close.

---

# 13. START MENU

Create a classic Windows-style Start menu.

It should eventually contain:

* user/profile area
* frequently used applications
* All Programs
* Documents
* Pictures
* Computer
* Control Panel / Settings
* Search
* Run
* Shut Down

Initially, only implement functionality that actually exists.

Do not create fake buttons that do nothing.

---

# 14. GUI FRAMEWORK

Build reusable GUI controls.

At minimum:

* Window
* Panel
* Label
* Button
* TextBox
* TextArea
* CheckBox
* RadioButton
* ListBox
* TreeView
* ScrollBar
* Menu
* MenuItem
* Dialog
* TabControl
* ProgressBar
* Image
* Icon

Every control should support appropriate:

* rendering
* mouse events
* keyboard events
* focus
* enabled/disabled state
* hover state
* click events

Applications should use these controls rather than implementing their own unrelated GUI systems.

---

# 15. FILESYSTEM

Persistent storage is essential.

The OS must eventually support:

* files
* directories
* creation
* deletion
* renaming
* reading
* writing
* file sizes
* timestamps
* paths

Start with a simple filesystem if necessary.

The initial filesystem does not need to be compatible with Windows.

A simple custom filesystem is acceptable.

However, design the filesystem layer so additional filesystem formats can be added later.

Eventually support reading/writing a common filesystem such as FAT32 or another suitable format.

---

# 16. NOTEPAD

Notepad is the first major application.

It must be a real application, not a demo.

Features:

* New
* Open
* Save
* Save As
* Exit
* Undo
* Cut
* Copy
* Paste
* Select All
* Text editing
* Cursor
* Keyboard navigation
* Scrolling
* File dialogs
* Unsaved changes detection

Example workflow:

Start Menu
→ Notepad
→ type text
→ File
→ Save As
→ Documents
→ test.txt
→ Save

Then after reboot:

Start Menu
→ Notepad
→ Open
→ test.txt

The saved text must still exist.

---

# 17. FILE EXPLORER

Create a basic Explorer-like file manager.

It should eventually support:

* drives
* folders
* files
* navigation
* back
* forward
* up
* address/path display
* folder creation
* file creation
* rename
* delete
* copy
* paste
* double-click to open
* context menus
* file properties

Double-clicking a text file should open it in Notepad.

---

# 18. CALCULATOR

Create a basic calculator application.

Support:

* addition
* subtraction
* multiplication
* division
* decimals
* clear
* backspace
* keyboard input

Eventually add scientific functionality.

---

# 19. TERMINAL

Provide a terminal application.

The terminal is NOT the primary interface.

It is a useful development and administration tool.

Initial commands:

* help
* dir
* cd
* type
* mkdir
* rmdir
* del
* copy
* move
* cls
* echo
* exit

Eventually add:

* process listing
* networking commands
* system information
* disk information

---

# 20. SETTINGS

Create a graphical settings/control-panel application.

Initial settings:

* display resolution
* wallpaper
* theme
* mouse sensitivity
* keyboard settings
* date/time
* system information

Do not expose settings that are not actually implemented.

---

# 21. APPLICATION MODEL

Applications should eventually run independently of the kernel.

Create a clear application API.

Applications should be able to request:

* windows
* GUI controls
* filesystem access
* keyboard events
* mouse events
* timers
* clipboard
* dialogs

Applications should not directly manipulate kernel internals.

---

# 22. MULTITASKING

The desktop must eventually support multiple applications simultaneously.

Example:

Notepad open
+
Calculator open
+
Explorer open

The user must be able to switch between them using the taskbar.

Applications must not freeze the entire desktop.

---

# 23. INPUT

Support:

Keyboard:

* letters
* numbers
* symbols
* modifiers
* function keys
* arrows
* Home
* End
* Page Up
* Page Down
* Delete
* Backspace
* Enter
* Escape

Mouse:

* movement
* left click
* right click
* middle click
* scrolling
* drag
* double click

---

# 24. CLIPBOARD

Implement a system-wide clipboard.

At minimum:

* text copy
* text paste
* cut

It must work between applications.

Example:

Notepad
→ Copy

Another text field
→ Paste

---

# 25. NETWORKING

Networking is NOT an initial milestone.

Implement it only after the desktop, filesystem and applications are stable.

Eventually support:

* Ethernet
* DHCP
* IPv4
* ARP
* DNS
* TCP
* UDP
* sockets

Later:

* TLS
* HTTPS

Do not attempt to implement a browser before the underlying networking stack works.

---

# 26. WEB BROWSER

The eventual OS should have a web browser.

However, do NOT attempt to write a complete modern browser engine from scratch initially.

HTML, CSS, JavaScript, TLS, HTTP/2, HTTP/3, WebAssembly and modern browser compatibility are enormous projects.

First establish:

OS
→ network stack
→ TCP/IP
→ DNS
→ HTTP
→ HTTPS
→ browser integration

A browser engine can be integrated later if technically feasible.

---

# 27. AUDIO

Eventually support:

* audio device detection
* basic output
* volume control
* mute

Then add media applications later.

---

# 28. USB

Eventually support:

* USB keyboard
* USB mouse
* USB storage

USB should not be an initial requirement for the first bootable GUI milestone.

---

# 29. HARDWARE ABSTRACTION

Do not hard-code the OS to the development PC.

The OS must be designed around hardware abstraction layers.

Never assume:

* a particular GPU
* a particular CPU
* a particular motherboard
* a particular storage device

QEMU compatibility is a major requirement.

---

# 30. DEBUGGING

Every subsystem must have useful diagnostics.

Provide:

* serial logging
* kernel logs
* panic messages
* application errors
* filesystem errors
* GUI debugging where useful

When something crashes, provide enough information to determine why.

Never hide errors simply to make the UI look cleaner.

---

# 31. DEVELOPMENT METHODOLOGY

Work incrementally.

DO NOT generate thousands of lines of unrelated code at once.

Each milestone should:

1. Have a clearly defined objective.
2. Compile.
3. Boot.
4. Be tested.
5. Be documented.
6. Become the foundation for the next milestone.

Suggested milestones:

MILESTONE 1:
Bootable x86-64 kernel.

MILESTONE 2:
Framebuffer graphics.

MILESTONE 3:
Keyboard and mouse.

MILESTONE 4:
Basic GUI framework.

MILESTONE 5:
Window manager.

MILESTONE 6:
XP-inspired desktop.

MILESTONE 7:
Taskbar and Start menu.

MILESTONE 8:
Filesystem.

MILESTONE 9:
Notepad.

MILESTONE 10:
Explorer.

MILESTONE 11:
Calculator.

MILESTONE 12:
Terminal.

MILESTONE 13:
Settings.

MILESTONE 14:
Multitasking.

MILESTONE 15:
Networking.

MILESTONE 16:
Browser.

Do not skip foundational milestones just because a later feature looks more interesting.

## Current milestone state

- Milestone 1, UEFI boot foundation: build-verified; QEMU starts with the configured pflash firmware.
- Milestone 2, framebuffer graphics: GOP mode selection and `canvas.png` wallpaper rendering implemented; visual VM verification pending.
- Milestone 3, keyboard and mouse: UEFI keyboard polling is tested. Relative and absolute UEFI pointer protocols are polled without exclusive ownership, with a polled PS/2 controller fallback. Relative motion is amplified 4x; QEMU-injected movement/click and physical host-mouse input are confirmed. Taskbar pointer hit-testing remains pending.
- Milestone 4, basic GUI framework: geometry, clipped GOP drawing, bitmap text, beveled button rendering, pointer state tests, and Space/Enter activation implemented; keyboard activation verified in QEMU.
- Milestone 5, window manager: fixed-capacity window state, z-order/focus, minimize/maximize/restore/close, hit-testing, and title-bar dragging implemented; keyboard-controlled sample window verified in QEMU.
- Milestone 6, XP-inspired desktop: wallpaper desktop and working Window Demo shortcut implemented; Enter-to-launch verified in QEMU. Desktop icon grid, context menus, and applications remain planned.
- Milestone 7, taskbar and Start menu: live window buttons, UEFI clock, and functional Window Demo, Notepad, and File Explorer entries implemented; keyboard Window Demo flow and mouse input are verified. Start/taskbar hit-testing remains pending.
- Milestone 8, filesystem: no_std 512-byte block-device interface, fixed-capacity file store, safe host image creation, and UEFI block discovery/mounting implemented. Format v2 supports nested directories and per-directory file operations; host tests verify nested contents survive remount. Format v1 remains flat and read/write compatible, with no implicit migration or formatting. QEMU boot with an AHCI image confirmed successful mount.
- Milestone 9, Notepad: Start menu launcher, bounded ASCII editing with caret navigation, F5 Open, F6 Save, and F7 Save As implemented. Fixed-capacity dialogs list MyOS files, support keyboard and pointer selection, prompt before overwriting, and protect unsaved edits when opening another file or closing the window. QEMU verified Save As, Open after reboot, cancellation, confirmed overwrite, and restoration of the last saved contents after discard-on-close. Files remain flat; folders, clipboard actions, undo, and mouse text selection are still planned.
- Milestone 10, Explorer: Start-menu launcher, fixed-capacity directory listing, folder navigation, folder/file creation, rename, confirmed delete, cross-directory file copy/paste, right-click menu, and file properties implemented. Notepad inherits a nested file's directory for Open, Save, and Save As. QEMU verified folder creation/navigation, nested file open/save, parent navigation, and cross-folder paste on a dedicated v2 AHCI volume; host inspection confirmed exact persisted contents.
- Milestone 11, Calculator: Start-menu launcher, on-screen keypad, keyboard/mouse input routing, and checked four-function fixed-point arithmetic implemented. Host tests cover decimal input, chained operations, negative results, division by zero/truncation, input limits, clear/backspace, and overflow. QEMU verified Start-menu launch, keyboard calculation `12.5 + 0.5 = 13`, and mouse-keypad calculation `1 + 2 = 3`.
- Input currently depends on UEFI boot services. Native drivers, interrupts, and post-`ExitBootServices` input are not implemented.

---

# 32. AI DEVELOPMENT RULES

You are responsible for maintaining architectural consistency across the project.

Before modifying code:

1. Inspect the existing project structure.
2. Identify relevant existing implementations.
3. Understand dependencies.
4. Do not duplicate functionality that already exists.
5. Preserve working functionality.
6. Modify the minimum necessary components.

When fixing bugs:

1. Identify the root cause.
2. Explain the cause briefly.
3. Fix the underlying problem.
4. Check for regressions.
5. Provide complete modified files.

DO NOT provide isolated snippets when a complete file is required.

When adding a feature, integrate it properly into the existing architecture.

Do not create temporary hacks and present them as final architecture.

---

# 33. CODE QUALITY

Prioritize:

* readability
* modularity
* maintainability
* memory safety
* error handling
* clear interfaces
* comments for genuinely complicated code
* consistent naming

Avoid:

* giant monolithic files
* unexplained global state
* duplicated systems
* magic numbers
* hard-coded screen coordinates wherever possible
* fake implementations
* placeholder functions presented as complete functionality

If something is not implemented yet, clearly mark it as TODO rather than pretending it works.

---

# 34. RESOURCE MANAGEMENT

The OS should eventually manage:

* memory
* processes
* file handles
* GUI objects
* devices

Avoid memory leaks.

Applications should clean up resources when closed.

---

# 35. PERFORMANCE

The first priority is correctness.

Then optimize.

Do not prematurely optimize complicated GPU rendering or advanced scheduling.

A software-rendered GUI running at a stable frame rate is acceptable initially.

The desktop should remain responsive.

---

# 36. SECURITY

Eventually implement:

* process isolation
* user permissions
* filesystem permissions
* safe system calls
* validation of application input

Do not give userspace applications unrestricted access to kernel memory.

---

# 37. USER EXPERIENCE

The OS should feel coherent.

If a user knows how to use an old Windows desktop, they should be able to understand MyOS without reading a manual.

Use familiar conventions:

* double-click opens things
* right-click opens context menus
* Start opens applications
* taskbar switches applications
* X closes windows
* minimize sends windows to taskbar
* maximize fills the desktop
* Save opens a file dialog
* Open opens a file dialog

Avoid unnecessary novelty.

---

# 38. IMPORTANT: REAL FUNCTIONALITY

Never create a UI element purely because it looks impressive.

If the desktop contains:

"Save"

then Save must actually save.

If the desktop contains:

"Open"

then Open must actually open.

If the desktop contains:

"Shutdown"

then it should actually shut down/reboot when implemented.

If something is not implemented, do not pretend it is.

---

# 39. TESTING

Every milestone must be testable in QEMU.

Create automated or semi-automated tests wherever practical.

Test:

* boot
* memory allocation
* filesystem
* file creation
* file reading
* file writing
* GUI controls
* application launching
* window management
* keyboard input
* mouse input

For example, the filesystem must be tested independently before Notepad depends on it.

---

# 40. DOCUMENTATION

The project must maintain both **technical source documentation** and a **complete interactive HTML documentation website**.

The documentation must always reflect the actual state of the project.

Maintain the following high-level documentation:

```text
README.md
ARCHITECTURE.md
BUILD.md
ROADMAP.md
```

These can provide concise developer-facing information.

However, the **primary human-readable documentation must be the HTML documentation website described in Section 43**.

Never claim features are complete if they are not.

Whenever a subsystem changes:

1. Update the relevant documentation.
2. Update its implementation status.
3. Update related architecture information.
4. Update relevant diagrams/examples.
5. Update the roadmap if necessary.

---

# 41. CURRENT DEVELOPMENT TASK

Do NOT attempt to implement the entire operating system immediately.

Continue from the **Current milestone state** above; do not restart at Milestone 1.

Begin Milestone 12: Terminal. Implement the initial roadmap commands (`help`, `dir`, `cd`, `type`, `mkdir`, `rmdir`, `del`, `copy`, `move`, `cls`, `echo`, `exit`) as a native MyOS application. Reuse the mounted filesystem and GUI-core input patterns, report unsupported operations explicitly, and validate command behavior on a dedicated v2 QEMU data volume before exposing commands that are not implemented.

Filesystem v2 nested create/list/file operations, cross-directory copy, and remount persistence are host-tested. Explorer root listing, open-in-Notepad, create, rename, confirmed delete, copy/paste, right-click context actions, and filename/size properties have been QEMU-verified. Folder navigation and directory-aware Notepad Open/Save have passed a dedicated QEMU integration test. Calculator arithmetic, Start-menu launch, keyboard input, and mouse keypad clicks are tested; continue with Milestone 12: Terminal.

---

# 42. RESPONSE FORMAT

For every development task:

1. State what you are implementing.
2. Explain the relevant architecture briefly.
3. List files created/modified.
4. Provide complete contents of modified files when requested.
5. Explain how to build.
6. Explain how to run in QEMU.
7. Explain how to test the feature.
8. Clearly identify anything that is not yet implemented.

Never claim the OS works unless the implementation actually supports the claimed functionality.

When modifying an existing source file, provide the **complete resulting file**, not merely a small patch or isolated snippet, unless the user explicitly asks for a diff.

---

# 43. INTERACTIVE HTML DOCUMENTATION WEBSITE

The MyOS project must contain a complete, self-contained **interactive HTML documentation website**.

This is a major project requirement.

The documentation website must NOT be a basic README converted into HTML.

It should function like a professional software-development documentation portal.

The documentation is part of the MyOS project and must be maintained alongside the source code.

---

## 43.1 DOCUMENTATION TECHNOLOGY

Initially use:

* HTML
* CSS
* JavaScript

Do not require a web server.

The documentation must work by opening the main HTML file directly in a normal browser.

Preferred structure:

```text
docs/

    index.html
    style.css
    script.js

    pages/
        getting-started.html
        architecture.html
        boot-process.html
        kernel.html
        memory.html
        graphics.html
        gui.html
        window-manager.html
        desktop.html
        taskbar.html
        start-menu.html
        filesystem.html
        applications.html
        notepad.html
        explorer.html
        calculator.html
        terminal.html
        settings.html
        drivers.html
        networking.html
        troubleshooting.html
        roadmap.html
```

The exact structure can be changed if a better architecture is appropriate.

The documentation must remain maintainable as the project grows.

---

## 43.2 DOCUMENTATION WEBSITE DESIGN

The website should have a polished desktop-software documentation appearance.

Use a layout similar to:

```text
┌───────────────────────────────────────────────────────────────┐
│ MYOS DOCUMENTATION                         🔍 Search          │
├───────────────────┬───────────────────────────────────────────┤
│                   │                                           │
│ OVERVIEW          │  MyOS Documentation                       │
│                   │                                           │
│ Getting Started   │  Welcome to the MyOS documentation.       │
│ Project Goals     │                                           │
│ Roadmap           │  Current Version                          │
│                   │  Development Build                        │
│ ARCHITECTURE      │                                           │
│                   │  ## Architecture                          │
│ System Overview   │                                           │
│ Boot Process      │  Explanation...                           │
│ Kernel            │                                           │
│ Memory            │                                           │
│                   │                                           │
│ HARDWARE          │                                           │
│ CPU               │                                           │
│ Memory            │                                           │
│ GPU               │                                           │
│ Storage           │                                           │
│ Input             │                                           │
│                   │                                           │
│ GUI               │                                           │
│ Graphics          │                                           │
│ Window Manager    │                                           │
│ Desktop           │                                           │
│ Taskbar           │                                           │
│ Start Menu        │                                           │
│ Controls          │                                           │
│                   │                                           │
│ APPLICATIONS      │                                           │
│ Notepad           │                                           │
│ Explorer          │                                           │
│ Calculator        │                                           │
│ Terminal          │                                           │
│ Settings          │                                           │
│                   │                                           │
│ DEVELOPMENT       │                                           │
│ Building          │                                           │
│ Testing           │                                           │
│ Debugging         │                                           │
│                   │                                           │
└───────────────────┴───────────────────────────────────────────┘
```

The visual design should be clean, readable and professional.

It may subtly use visual elements inspired by the MyOS/XP aesthetic, but the documentation itself should remain modern and easy to navigate.

---

## 43.3 SIDEBAR NAVIGATION

The left side must contain a persistent navigation sidebar.

The sidebar must contain expandable/collapsible categories.

Example:

```text
OVERVIEW
  Getting Started
  Project Goals
  Roadmap

ARCHITECTURE
  System Overview
  Boot Process
  Kernel
  Memory
  Processes
  IPC
  System Calls

HARDWARE
  CPU
  Memory
  Graphics
  Storage
  Keyboard
  Mouse
  USB
  Audio
  Network

GUI
  Graphics
  Window Manager
  Desktop
  Taskbar
  Start Menu
  Controls
  Themes
  Input

FILESYSTEM
  Overview
  Files
  Directories
  Paths
  Permissions
  Storage

APPLICATIONS
  Notepad
  File Explorer
  Calculator
  Terminal
  Settings

DEVELOPMENT
  Building
  QEMU
  Debugging
  Testing
  Contributing

TROUBLESHOOTING
  Boot Problems
  Graphics Problems
  Input Problems
  Filesystem Problems
  Application Problems
```

Clicking a category expands or collapses its children.

Use subtle animations.

Remember expanded/collapsed state using localStorage.

The current page must be highlighted.

---

## 43.4 SEARCH SYSTEM

The documentation must have a prominent search box.

Search must operate entirely client-side.

The user should be able to search terms such as:

```text
memory
filesystem
window manager
keyboard
Notepad
QEMU
UEFI
```

Search should search:

* page titles
* headings
* descriptions
* keywords
* documentation text

Search results must be clickable.

Where practical, matching text should be highlighted.

Keyboard shortcut:

```text
Ctrl + K
```

must focus the search field.

Escape should close search results.

The search system must work offline.

Do not depend on an external search API.

---

## 43.5 EXPANDABLE EXPLANATIONS

Documentation should use expandable sections for complicated concepts.

Example:

```text
Physical Memory Manager ▼
```

When clicked:

```text
The physical memory manager controls which physical
RAM pages are available to the kernel and applications.
```

Additional sections may include:

```text
How does it work? ▼

Why does MyOS need it? ▼

What depends on it? ▼
```

This allows the documentation to provide detailed explanations without overwhelming the user.

---

## 43.6 INTERACTIVE ARCHITECTURE DIAGRAMS

Where useful, create interactive architecture diagrams.

For example:

```text
UEFI
  ↓
Bootloader
  ↓
Kernel
  ↓
Memory Manager
  ↓
Process Manager
  ↓
GUI
  ↓
Applications
```

Each component should be clickable.

Clicking:

```text
Memory Manager
```

could display:

```text
MEMORY MANAGER

Purpose:
Controls allocation and management of system memory.

Responsible for:
- physical memory
- virtual memory
- heap
- page tables

Dependencies:
- CPU
- MMU
- boot memory map

Used by:
- kernel
- processes
- applications
```

Architecture diagrams must reflect the actual architecture rather than invented systems.

---

## 43.7 DOCUMENTATION PAGE FORMAT

Every major documentation page should contain:

1. Page title.
2. Short description.
3. Status indicator.
4. Overview.
5. Why it exists.
6. How it works.
7. Dependencies.
8. Related components.
9. Examples where appropriate.
10. Troubleshooting.
11. Related documentation.

Example:

```text
# Window Manager

Status: 🟡 In Development

The Window Manager controls application windows.

## Overview

...

## What does it do?

...

## How does it work?

...

## Window lifecycle

...

## Events

...

## Related components

...
```

---

## 43.8 CODE EXAMPLES

Documentation must support syntax-highlighted code blocks.

Code blocks must have a copy button.

When clicked:

```text
Copy
```

should copy the code to the clipboard.

Display a temporary:

```text
Copied!
```

confirmation.

---

## 43.9 SOURCE REFERENCES

Documentation should reference relevant source files.

Example:

```text
Window Manager

Source:
gui/windowmanager/window_manager.rs
```

Where practical, clicking the reference should navigate to the relevant source/documentation page.

The documentation should explain what important source files are responsible for.

---

## 43.10 STATUS INDICATORS

Every major subsystem must display its current development status.

Use statuses such as:

```text
🟢 Implemented
🟡 In Development
🔴 Planned
⚠️ Experimental
```

Never mark a feature as implemented unless it actually works.

The documentation must automatically or manually reflect the current project state.

---

## 43.11 ROADMAP PAGE

Create an interactive roadmap.

Example:

```text
BOOT
🟢 Kernel boots
        ↓
GRAPHICS
🟢 Framebuffer
        ↓
INPUT
🟡 Keyboard
🟡 Mouse
        ↓
GUI
🟡 Window Manager
🔴 Desktop
        ↓
APPLICATIONS
🔴 Notepad
🔴 Explorer
🔴 Calculator
        ↓
NETWORKING
🔴 Ethernet
🔴 TCP/IP
        ↓
BROWSER
🔴 Web Browser
```

Each roadmap item should be clickable and open the appropriate documentation.

The roadmap must reflect the actual state of the project.

---

## 43.12 HARDWARE DOCUMENTATION

Document:

### CPU

* architecture
* cores/threads
* CPU initialization
* supported architecture

### Memory

* physical memory
* virtual memory
* paging
* allocation
* heap

### GPU

* framebuffer
* software rendering
* future acceleration
* hardware abstraction

### Storage

* drives
* partitions
* filesystem
* storage drivers

### Input

* keyboard
* mouse
* USB

### Networking

* Ethernet
* future Wi-Fi
* network stack

Each page must clearly distinguish between:

```text
Currently implemented
```

and:

```text
Planned
```

---

## 43.13 GUI DOCUMENTATION

The GUI documentation must be particularly detailed.

Document:

* renderer
* framebuffer
* compositor
* windows
* controls
* themes
* fonts
* mouse input
* keyboard input
* focus
* events
* window z-order
* taskbar
* Start menu
* desktop
* dialogs
* menus

Include diagrams where they improve understanding.

---

## 43.14 APPLICATION DOCUMENTATION

Every built-in application should have its own documentation page.

For example:

```text
# Notepad

Purpose:
Simple graphical text editor.

Features:

🟢 Text editing
🟢 Open
🟢 Save
🟢 Save As
🟢 Clipboard
🟡 Search
🔴 Printing
```

Explain the application's architecture.

For example:

```text
Notepad
   ↓
GUI Framework
   ↓
Filesystem API
   ↓
Virtual Filesystem
   ↓
Storage Driver
   ↓
Disk
```

---

## 43.15 FILESYSTEM DOCUMENTATION

Explain the complete lifecycle of file operations.

Example:

```text
User clicks Save
       ↓
Notepad
       ↓
Filesystem API
       ↓
Virtual Filesystem
       ↓
Filesystem Driver
       ↓
Storage Driver
       ↓
Disk
```

Explain what happens at every stage.

---

## 43.16 TROUBLESHOOTING

Create a searchable troubleshooting section.

Include topics such as:

```text
MyOS doesn't boot

The screen is black

Mouse doesn't work

Keyboard doesn't work

File won't save

QEMU crashes

Application crashes

Filesystem corruption
```

Each troubleshooting entry should contain:

* Symptoms
* Possible causes
* How to diagnose
* Solution
* Relevant logs
* Related documentation

---

## 43.17 DOCUMENTATION WRITING STYLE

Documentation should be understandable to someone who knows basic programming but may not understand operating-system internals.

Explain complicated concepts simply first.

Then provide deeper technical information.

For example:

Instead of only writing:

> Virtual memory provides an abstraction over physical address translation through hierarchical paging structures.

Write:

> Virtual memory lets each program behave as though it has its own private memory space. The CPU's paging system translates those virtual addresses into physical RAM addresses.

Then provide the deeper technical explanation below it.

---

## 43.18 RESPONSIVE DESIGN

The documentation website must work on:

* desktop
* laptop
* tablet
* mobile

On smaller screens:

* sidebar becomes a collapsible drawer
* search remains accessible
* content becomes single-column
* navigation remains usable

Do not allow horizontal scrolling to become necessary for normal documentation.

---

## 43.19 DARK MODE

Support:

* Light mode
* Dark mode

Provide a theme toggle.

Remember the user's choice using localStorage.

---

## 43.20 NAVIGATION

Support:

* sidebar navigation
* breadcrumbs
* previous page
* next page
* search
* related topics
* browser back/forward

The current page must be highlighted.

The current page must remain visible within the sidebar hierarchy.

---

## 43.21 OFFLINE OPERATION

The documentation website must work offline.

Do not depend on:

* external CDNs
* online APIs
* external fonts
* external JavaScript libraries

unless there is a compelling technical reason.

Prefer self-contained HTML, CSS and JavaScript.

---

## 43.22 DOCUMENTATION MAINTENANCE

Whenever an OS subsystem changes:

1. Update its documentation.
2. Update its status.
3. Update related architecture diagrams.
4. Update relevant examples.
5. Update the roadmap.
6. Add troubleshooting information for significant new failure modes.

Documentation must never knowingly describe functionality that no longer exists.

If an implementation changes, the documentation must change with it.

---

# 44. FINAL PROJECT PRINCIPLE

The ultimate objective is:

**A real, bootable, graphical, Windows XP-inspired desktop operating system with persistent files, native applications, a reusable GUI framework, and a complete interactive offline documentation website.**

The desktop should eventually feel like a coherent operating system rather than a collection of demonstrations.

The documentation should eventually be detailed enough that a developer can understand:

* how MyOS boots
* how the kernel works
* how memory works
* how drivers work
* how the GUI works
* how windows work
* how applications work
* how files are stored
* how applications communicate with the OS
* how to build the OS
* how to test it
* how to debug it
* what has been implemented
* what is currently being developed
* what is planned

Build the operating system carefully.

Build the documentation alongside it.

**Never sacrifice architectural correctness merely to make a milestone look impressive.**

Build one subsystem at a time, test it, document it, and then move to the next subsystem.
