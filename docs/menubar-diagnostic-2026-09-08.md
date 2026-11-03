# Menu bar investigation — unresolved

Installed macOS: 26.3 (25D2125), built-in display 2940 × 1912 physical pixels.

The menu bar item is created, remains retained by Tauri's resource table, has a nonempty title and an 18-point template image, and reports `isVisible = true`. Its measured rectangle remains at physical (0, 1912), size (136, 44) after startup: outside the display.

ControlCenter logs explicitly report `Starting to track blocked host` for `com.kenji.lava-timer`. System Settings > Menu Bar > Allow in Menu Bar does not list LavaTimer. The behavior was reproduced both for a terminal-launched diagnostic build and the installed app launched normally through Finder. TCC logs confirm the Finder-launched process is attributed to LavaTimer itself, excluding the hypothesis that the ChatGPT menu bar setting explains that run.

Attempts that did not resolve it:

- Setting a stable NSStatusItem autosave name and requesting visibility. Removed this unsuccessful candidate change.
- Restarting ControlCenter and StatusKitAgent.
- Force-registering only /Applications/LavaTimer.app with LaunchServices and reopening System Settings.

No menu bar settings were changed and no system preference databases or timer records were deleted. Restored the installed app from the pre-investigation 1.0.6 backup; its signature verifies. The existing 1.0.6 DMG is unchanged. Source retains opt-in `LAVA_TRAY_DIAGNOSTICS` startup/first-five-tick logging for follow-up; this is not a release fix.

Next: after a user-performed macOS restart, open /Applications/LavaTimer.app normally and recheck menu bar registration and actual visibility. A restart is an unverified diagnostic step, not a promised cure.

Diagnostic output: /tmp/lava-tray-diagnostic.log
Build output: /tmp/lava-tray-diagnostic-build.log

Related primary reports (similar symptoms, not proof of the same root cause):
- https://github.com/tauri-apps/tauri/issues/13770
- https://github.com/robinebers/openusage/issues/517


## Recheck — 2026-11-03

The installed app is still logged as `Starting to track blocked host` by ControlCenter. It is not sufficient to infer menu bar visibility from a successful build, signature verification, or a working window.

System Settings now shows an enabled entry named `lava-timer`. A separately launched raw binary was logged as `.adhocBinary(lava-timer)` while the installed app was logged by bundle identifier, so the enabled row is not proof that the installed bundle's status item is permitted. No toggles or preference databases were changed.

The candidate adds an explicit application-menu action, `LavaTimer → 显示计时器`, with app-local Cmd+Shift+L. Native verification confirms the action is present and the window can be recovered. This mitigates loss of access but does not resolve the menu bar blocker.


### Authorized off/on check — 2026-11-03 10:13–10:14 (Asia/Shanghai)

With explicit user approval, toggled only the `lava-timer` Allow in Menu Bar switch from on to off and back on. The UI confirmed both transitions and the final on state. No preference databases were deleted and no other system settings were changed.

Quit and relaunched `/Applications/LavaTimer.app` normally. At 10:14:20.560, ControlCenter still logged `Starting to track blocked host; (bid:com.kenji.lava-timer-Item-0-83293)`. Therefore this off/on workaround did not unblock the installed bundle; actual menu bar display/click acceptance remains blocked.

The timer was running at the start of this check. Settled it at 24:11 before restart, confirmed 24:11 was retained on reopen, then restored the original running state. No project edits or resets occurred.

Local evidence: `/tmp/lava-timer-menubar-retoggle-relaunch-20261103.log` and `/tmp/lava-timer-release-20261103/menubar-retoggled.png`.
