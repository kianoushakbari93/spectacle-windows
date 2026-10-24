# Spectacle for Windows

Easily organize windows without using a mouse.

Spectacle is a window-management app driven entirely by keyboard shortcuts.
This is the Windows edition of the macOS app: a native 64-bit Windows 11 app
written in Rust, with the same window actions, the same undo and redo history
and a Settings window built in the Windows 11 style.

## System requirements

Spectacle is designed for Windows 11 (64-bit), and also runs on 64-bit
Windows 10 version 2004 or later. It installs no runtimes or other
dependencies.

It is small and light: about 3 MB of private memory while it runs in the
background, and nothing at all is processed except its own keyboard
shortcuts.

## Installing

Run `Spectacle-3.0.1-Setup.exe` and follow the steps. Spectacle installs to
`C:\Program Files\Spectacle`, adds itself to the Start menu and starts when
setup finishes. The first time it starts it opens its Settings window, where
**Launch Spectacle at sign-in** makes it start with Windows.

Spectacle lives in the system tray. Windows 11 hides new tray icons in the
overflow area (the **^** next to the clock); to keep Spectacle's glasses icon
visible, drag it onto the taskbar, or turn it on in **Settings → Personalization
→ Taskbar → Other system tray icons**.

To uninstall, use **Settings → Apps → Installed apps → Spectacle → Uninstall**.
Your shortcuts are kept in `%APPDATA%\Spectacle` in case you reinstall; delete
that folder to remove them too.

## Keyboard Shortcuts

Spectacle makes use of several keyboard shortcuts that trigger specific
_window actions_. A window action is nothing more than a command that tells
Spectacle how to change the size and/or position of a particular window.

A keyboard shortcut consists of one or more modifier keys paired with another
key. Windows has no Command or Option keys and already uses many Windows key
shortcuts for Snap and virtual desktops, so on Windows Spectacle's main chord
is **Ctrl+Win+Alt**, which Windows leaves free.

The default keyboard shortcuts can be changed at any time in Settings. Use new
key combinations to trigger window actions, or clear particular keyboard
shortcuts to disable window actions entirely.

| Window action      | Default shortcut         |
|:-------------------|:-------------------------|
| Center             | Ctrl+Win+Alt+C           |
| Fullscreen         | Ctrl+Win+Alt+F           |
| Left Half          | Ctrl+Win+Alt+Left        |
| Right Half         | Ctrl+Win+Alt+Right       |
| Top Half           | Ctrl+Win+Alt+Up          |
| Bottom Half        | Ctrl+Win+Alt+Down        |
| Upper Left         | Ctrl+Win+Alt+U           |
| Upper Right        | Ctrl+Win+Alt+I           |
| Lower Left         | Ctrl+Win+Alt+J           |
| Lower Right        | Ctrl+Win+Alt+K           |
| Next Display       | Win+Alt+Shift+Right      |
| Previous Display   | Win+Alt+Shift+Left       |
| Next Third         | Ctrl+Win+Shift+Right     |
| Previous Third     | Ctrl+Win+Shift+Left      |
| Make Larger        | Ctrl+Alt+Shift+Right     |
| Make Smaller       | Ctrl+Alt+Shift+Left      |
| Undo               | Ctrl+Win+Alt+Z           |
| Redo               | Ctrl+Win+Alt+Y           |

The corners sit on U, I, J and K, which are laid out on the keyboard like the
corners of the screen.

### Basic window actions

To move a window to the center of the screen use Ctrl+Win+Alt+C. Centered
windows will __not__ have their size altered. Make a window fill the screen,
leaving the taskbar visible, with Ctrl+Win+Alt+F.

Windows may also be resized between thirds using the half and corner
shortcuts. For example, to have a window resized between 1/3 and 2/3 of the
left side of the screen simply press the *left half* shortcut more than once.
Each time the shortcut is pressed Spectacle will move the window between 1/2,
2/3, and 1/3 of the left side of the screen. This also applies to the corners.

Spectacle can also move windows between horizontal and vertical thirds of the
screen. Ctrl+Win+Shift+Right moves a window to the next third, starting with
the third on the left of the screen, and Ctrl+Win+Shift+Left moves it to the
previous third.

Resizing a window is just as easy. To make a window a bit larger use
Ctrl+Alt+Shift+Right; Ctrl+Alt+Shift+Left makes it smaller. Spectacle will
always attempt to maintain contact between the edges of a window and the edges
of the screen when resizing.

Every window action can also be chosen from the tray icon's menu.

### Multiple displays

Win+Alt+Shift+Right moves a window to the next display and Win+Alt+Shift+Left
to the previous one. Displays are taken in order from left to right (and top
to bottom). A window that fits on the other display keeps its size and is
centered; one that does not fits itself to the display.

### Window action history

Spectacle remembers where every window was prior to executing a window
action, separately for each application. To undo a window action use
Ctrl+Win+Alt+Z, and Ctrl+Win+Alt+Y to redo it. Undoing the first action on a
maximized window maximizes it again.

### Disabling Spectacle

The tray menu's **Disable** submenu turns Spectacle's shortcuts off *for an
hour*, or for the application in front, so that application's own shortcuts
work. Spectacle's shortcuts are always off while its own windows are in
front.

## Common Issues

### A shortcut does not work or is shown with a warning

Another application, or Windows itself, already uses that key combination.
Windows gives each combination to only one application, so Spectacle shows a
warning sign next to the shortcut in Settings. Choose a different shortcut,
or change it in the other application. A common example is Intel graphics
drivers, which use Ctrl+Alt+Arrow keys to rotate the screen; that is why
Spectacle's defaults include the Windows key.

### Spectacle does not move a particular window

Windows protects applications that run as administrator from applications that
do not, so Spectacle, which runs with ordinary rights, cannot move them. The
same applies to the Start menu, the taskbar and other parts of Windows itself.
Spectacle plays the system's default sound whenever a window action cannot be
carried out, for example when the window is already where the action would
put it.

### Spectacle does not resize a particular window as expected

Applications can place constraints on the size of their windows. Fixed-size
windows such as many dialogs are moved and centered in the intended area
without being resized. Windows with a minimum size are made as small as they
allow and centered in the intended area. The constraints placed on windows
will always be respected despite leading to potentially unexpected results.

### Spectacle behaves strangely with some terminal windows

Some terminals resize their windows in whole rows and columns of text. To work
around this Spectacle first tries to resize a window to the desired size. If
the window does not fit, Spectacle makes it a little smaller until it does,
then centers it within the desired area, at the cost of a slightly jittery
experience.

### The Settings window is in the wrong language

Spectacle follows the Windows display language, and is available in English,
Spanish, Finnish, French, Italian and Portuguese.

## Security and privacy

* Spectacle never connects to the network and collects nothing.
* It runs with ordinary user rights and never asks for administrator rights;
  only the installer does, to write to `C:\Program Files`.
* Shortcuts are registered with `RegisterHotKey`, so Windows only ever tells
  Spectacle about its own key combinations. Spectacle installs no keyboard
  hooks and cannot see anything else you type.
* The executable is built with Control Flow Guard, CET shadow stack
  compatibility, ASLR and DEP. It loads system DLLs from System32 only, and at
  start-up it turns on process mitigations that block DLLs from remote or
  low-integrity locations and legacy injection mechanisms.
* Settings are plain JSON files in `%APPDATA%\Spectacle` (`Shortcuts.json`,
  in the same format as the macOS version, and `Settings.json`). Files that
  cannot be read are set aside as `*.json.invalid` rather than overwritten.
* The executable, setup program and uninstaller are code signed when they are
  built on a computer with a code signing certificate (see *Code signing*
  below), so that Windows shows the publisher rather than "Unknown
  publisher".

## Accessibility

The Settings window can be used entirely from the keyboard: Tab and Shift+Tab
move between controls, Space or Enter activates them, Delete or Backspace
clears a shortcut and Esc cancels recording one. It supports screen readers
such as Narrator through UI Automation, follows the light, dark and high
contrast themes and the accent color, and scales with the display.

## Building

Spectacle is a Rust project. To build it you need:

* [Rust](https://rustup.rs) 1.99 or later, with the default
  `x86_64-pc-windows-msvc` toolchain.
* The Microsoft C++ build tools and Windows SDK, from
  [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/)
  with the *Desktop development with C++* workload.
* [Inno Setup 7](https://jrsoftware.org/isinfo.php), for the installer.

With winget:

    winget install Rustlang.Rustup
    winget install Microsoft.VisualStudio.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
    winget install JRSoftware.InnoSetup.7

Then build everything (formatting check, lints, tests, release build and
installer) with:

    pwsh scripts\build.ps1

The installer is written to `dist\Spectacle-3.0.1-Setup.exe`. For development,
`cargo run` starts a debug build and `cargo test` runs the test suite, which
includes every window position calculation test of the macOS version.

### Code signing

When the build finds a code signing certificate issued to "Kianoush Akbari"
in the current user's certificate store, it signs `Spectacle.exe`, the setup
program and the uninstaller with it, and timestamps the signatures so they
stay valid after the certificate expires. Use `-CertificateThumbprint` (or the
`SPECTACLE_SIGNING_CERTIFICATE` environment variable) to pick another
certificate, or `-Unsigned` to skip signing. Without a certificate the build
is unsigned and Windows shows "Unknown publisher".

A certificate from a public certificate authority is trusted by every
Windows PC. A self-signed certificate is trusted only by the PCs it is added
to, which suits builds for your own computer. To create one and trust it on
this PC (the second command needs an administrator PowerShell):

    New-SelfSignedCertificate -Type CodeSigningCert -Subject "CN=Kianoush Akbari" -KeyAlgorithm RSA -KeyLength 3072 -KeyExportPolicy NonExportable -CertStoreLocation Cert:\CurrentUser\My -NotAfter (Get-Date).AddYears(5) | Export-Certificate -FilePath KianoushAkbari.cer
    Import-Certificate -FilePath KianoushAkbari.cer -CertStoreLocation Cert:\LocalMachine\Root

### Project layout

| Path                    | Contents                                               |
|:------------------------|:-------------------------------------------------------|
| `src/calculations.rs`   | Window position calculations, ported from the macOS app |
| `src/position_manager.rs`, `src/movers.rs`, `src/history.rs` | Performing window actions and their undo history |
| `src/window.rs`, `src/screens.rs` | Windows and displays, through the Win32 API   |
| `src/hotkeys.rs`, `src/shortcut.rs` | Keyboard shortcuts and their key bindings   |
| `src/app.rs`, `src/tray.rs` | The background app, its tray icon and menu         |
| `src/preferences.rs`, `src/accessibility.rs` | The Settings window, drawn with Direct2D |
| `src/storage.rs`, `src/login_item.rs` | Settings files and launching at sign-in   |
| `installer/Spectacle.iss` | The Inno Setup installer script                      |
| `tools/make_icon.py`    | Builds `resources/spectacle.ico` from the original artwork |

# License

Copyright (c) 2017 Eric Czarny / 2026 Kianoush Akbari.

Spectacle should be accompanied by a LICENSE file containing the license relevant to this distribution.
