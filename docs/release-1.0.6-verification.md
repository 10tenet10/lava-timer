# LavaTimer 1.0.6 verification

Status: unreleased candidate; stable release is blocked by the unresolved menu bar issue and incomplete system acceptance.

## Original candidate — 2026-09-07

The installed application was 1.0.4. Native visual inspection reproduced an empty timer ring, displaced readout, and broken history styles.

## Corrections

- Ship the existing 1.0.5 SVG readout and scrolling fixes.
- Allow same-origin template fetch in connect-src, matching the runtime's raw-template reload.
- Preserve the explicitly configured style-src policy by excluding only style-src from Tauri's build-time CSP injection. The renderer creates stylesheet elements at runtime; injected hashes otherwise override unsafe-inline and block those styles. Script CSP injection remains enabled; external network scripts remain disallowed.
- Enable withGlobalTauri for the event listener used by screen-off and expansion-direction handlers.

## Verified

- 17 frontend tests and 4 Rust tests passed.
- Production app and Apple Silicon DMG built successfully.
- codesign --verify --deep --strict passed on candidate and installed app.
- hdiutil verify passed on the DMG.
- Native production UI: panel readout is inside the ring, capsule is complete, history grid and range controls render correctly, settings layout is complete, system scrollbars are absent.
- Installed version in /Applications/LavaTimer.app is 1.0.6 and was reopened successfully.
- Existing records were retained; no project edits or timer resets were performed.

The native window intentionally remains transparent, so its background color depends on the desktop. Pixel-identical rendering across browser engines is not asserted. Physical sleep/lock and long-duration timer behavior were not exercised in this session. Signature is ad-hoc; Apple notarization was not performed. No GitHub release was published.

Original 2026-09-07 DMG SHA-256 (superseded by the current rebuild): ad3b92e5f1eccc1f51bebc283c803458c0ad245c5017d6b919a030631da96a22

Old application backup: /Users/kenji/Library/Application Support/LavaTimer-release-backups/LavaTimer-1.0.4-before-1.0.6-20260907.zip

Build log: /tmp/lava-timer-1.0.6-build.log
Test log: /tmp/lava-timer-1.0.6-check.log


## Release audit — 2026-11-03

The release remains a candidate. GitHub stable release and main were still 1.0.5 at the start of this audit.

### Changes in this candidate

- Add an application-menu recovery action and app-local shortcut Cmd+Shift+L. This is not a global shortcut or a fix for the blocked status item.
- Correct compact history time rounding. For example, 21:32 now shows 21m in both the total and the daily summary; positive values below a minute show <1m.
- Make English the default README, retain Chinese in README.zh-CN.md, document recovery and the menu bar limitation, and correct the claim that removing only the app necessarily erases its WebView records.
- Extend CI from tests alone to formatting checks, tests, and production app packaging.

### Completed

- `npm run check`: 18 frontend and 4 Rust tests pass, including history rounding boundaries.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check` and `git diff --check` pass.
- Production Apple Silicon App and DMG build succeeds. Candidate signature verification and DMG checksum verification pass.
- Built-in-browser smoke test on an isolated localhost origin: start/pause, panel/capsule transitions, 7/30/all history ranges, range persistence after reload, settings controls, and panel rendering.
- Native candidate launch retained the paused 21:32 record. The app-menu action is present and works; Cmd+Shift+L recovers the timer window.
- Physical macOS lock/unlock on the native recovery candidate: timer advanced from 21:32 to 21:35, then stayed paused across the locked interval and unlock. No offline time was added.

### Still required before stable publication

- Resolve and reverify actual menu bar visibility and click-to-reopen behavior. Current ControlCenter logs still report a blocked host for `com.kenji.lava-timer`. System Settings now lists an enabled `lava-timer` entry; that does not establish permission or visibility for the bundled app identity. No system preferences were changed.
- Display-off/wake: actual display sleep triggered; result awaits manual unlock and readback.
- Whole-system sleep/wake: not yet performed.
- Final rebuilt candidate native readback after the history rounding correction.
- Long-duration hidden-window timing check and final cloud CI results.
- macOS 11 hardware compatibility is not tested on this machine. Signature remains ad-hoc and notarization is not performed.

### Artifacts and recovery

Current candidate DMG: `src-tauri/target/release/bundle/dmg/LavaTimer_1.0.6_aarch64.dmg`

Current candidate DMG SHA-256: `c7a1a04c711ce02d1789d7b60da9f793d358473c77618ead207344f991959f49`

This hash supersedes the September candidate. Recompute it if the candidate is rebuilt.

Pre-test application and WebView backups (local only):
`/Users/kenji/Library/Application Support/LavaTimer-release-backups/20261103/`
The paused WebView backup was taken after normal application exit. No user project edits or timer resets were performed; system acceptance uses short explicitly authorized timer runs.

Current local logs: `/tmp/lava-timer-1.0.6-check-20261103.log`, `/tmp/lava-timer-1.0.6-build-20261103.log`, `/tmp/lava-timer-1.0.6-dmg-verify-20261103.log`.
Browser and native screenshots: `/tmp/lava-timer-release-20261103/`.
