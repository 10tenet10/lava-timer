# LavaTimer 1.0.6 — release draft

This candidate repairs production rendering and adds another way to reopen the timer when the macOS menu bar entry is unavailable.

- Restore runtime styles and same-origin template loading in packaged builds while retaining script CSP injection.
- Enable native event subscriptions for screen inactivity and expansion direction.
- Add **LavaTimer → 显示计时器** and **⌘⇧L** while the app is active. Dock / Applications reopening remains available.
- Keep compact history labels from rounding focus time up to a minute or hour that has not elapsed; show `<1m` for positive sub-minute durations.
- Retain the SVG timer readout and native scroll locking from 1.0.5.

## Release gate

Do not mark this candidate as the latest stable release while the menu bar blocker remains unresolved. The current macOS 26.3 machine logs `Starting to track blocked host` for the installed application. The system settings entry named `lava-timer` is enabled, but that alone does not prove the bundled app is allowed or visible. An explicitly authorized off/on check of that single switch and normal app relaunch did not unblock the installed app. No preference databases were deleted and no other system settings were changed.

The release verification document tracks physical lock, display-off, sleep/wake and duration checks. A passing CI run verifies tests and production app packaging; it does not prove menu bar visibility or physical power-state behavior.

## Platform

Apple Silicon, macOS 11+. Ad-hoc signed; not notarized. Timer records stay on this Mac, with no built-in backup or cloud sync. macOS 11 compatibility has not been exercised on hardware in this release audit.

The final publication date and DMG checksum must be taken from the accepted candidate, not the September build.
