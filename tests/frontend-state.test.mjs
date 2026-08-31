import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

process.env.TZ = 'Asia/Shanghai';

const html = readFileSync(new URL('../src/index.html', import.meta.url), 'utf8');
const logicMatch = html.match(/<script type="text\/x-dc"[^>]*>([\s\S]*?)<\/script>/);
if (!logicMatch) throw new Error('LavaTimer component logic was not found in src/index.html');
const componentSource = logicMatch[1] + '\n;globalThis.__LavaTimerComponent = Component;';

function createComponent({ saved = null, now }) {
  const clock = { now };
  class FakeDate extends Date {
    constructor(...args) {
      if (args.length) super(...args);
      else super(clock.now);
    }

    static now() {
      return clock.now;
    }
  }

  const values = new Map();
  if (saved) values.set('lava-timer-v2', JSON.stringify(saved));
  const localStorage = {
    getItem(key) {
      return values.has(key) ? values.get(key) : null;
    },
    setItem(key, value) {
      values.set(key, String(value));
    },
  };

  class FakeDCLogic {
    constructor() {
      this.props = {};
    }

    setState(patch, callback) {
      this.state = { ...this.state, ...patch };
      if (callback) callback();
    }

    forceUpdate() {}
  }

  const window = {
    crypto: { randomUUID: () => 'test-uuid' },
    confirm: () => true,
  };
  const context = vm.createContext({
    Array,
    Boolean,
    Date: FakeDate,
    DCLogic: FakeDCLogic,
    Element: class {},
    Event: class {},
    JSON,
    Map,
    Math,
    Number,
    Object,
    Promise,
    RegExp,
    Set,
    String,
    clearInterval() {},
    clearTimeout() {},
    console,
    document: {},
    localStorage,
    requestAnimationFrame: () => 1,
    cancelAnimationFrame() {},
    setInterval: () => 1,
    setTimeout: () => 1,
    window,
  });
  vm.runInContext(componentSource, context, { filename: 'src/index.html' });
  const component = new context.__LavaTimerComponent();
  return { clock, component, localStorage, window };
}

function savedState(overrides = {}) {
  return {
    view: 'panel',
    projects: [
      { id: 'project-a', name: '项目 A', goal: 7200, sec: 100 },
      { id: 'project-b', name: '项目 B', goal: 3600, sec: 0 },
    ],
    proj: 0,
    dateKey: '2026-08-30',
    runStart: null,
    lastActiveAt: null,
    history: {},
    projectHistory: {},
    projectLabels: { 'project-a': '项目 A', 'project-b': '项目 B' },
    ...overrides,
  };
}

test('cold start settles only through the last heartbeat and remains paused', () => {
  const runStart = new Date(2026, 7, 30, 10, 0, 0).getTime();
  const lastActiveAt = runStart + 5_000;
  const now = runStart + 60 * 60 * 1000;
  const { component } = createComponent({
    now,
    saved: savedState({ runStart, lastActiveAt }),
  });

  assert.equal(component.state.projects[0].sec, 105);
  assert.equal(component.state.runStart, null);
  assert.equal(component.state.lastActiveAt, null);
});

test('legacy running state without a heartbeat does not count offline time', () => {
  const runStart = new Date(2026, 7, 30, 10, 0, 0).getTime();
  const { component } = createComponent({
    now: runStart + 60 * 60 * 1000,
    saved: savedState({ runStart, lastActiveAt: undefined }),
  });

  assert.equal(component.state.projects[0].sec, 100);
  assert.equal(component.state.runStart, null);
});

test('a running interval crossing midnight is split into the correct dates', () => {
  const runStart = new Date(2026, 7, 29, 23, 59, 0).getTime();
  const now = new Date(2026, 7, 30, 0, 1, 0).getTime();
  const { component } = createComponent({ now });
  const state = savedState({
    dateKey: '2026-08-29',
    runStart,
    lastActiveAt: runStart,
    projects: [
      { id: 'project-a', name: '项目 A', goal: 7200, sec: 120 },
      { id: 'project-b', name: '项目 B', goal: 3600, sec: 0 },
    ],
  });

  const next = component.rolloverState(state, '2026-08-30', now);
  assert.equal(next.history['2026-08-29'], 180);
  assert.equal(next.projectHistory['2026-08-29']['project-a'], 180);
  assert.equal(next.projects[0].sec, 60);
  assert.equal(next.runStart, now);
});

test('switching projects settles the old project and keeps the timer running', () => {
  const now = new Date(2026, 7, 30, 15, 0, 0).getTime();
  const { component } = createComponent({ now });
  component.state = savedState({ runStart: now - 10_000, lastActiveAt: now - 5_000 });

  component.switchProj(1);

  assert.equal(component.state.projects[0].sec, 110);
  assert.equal(component.state.proj, 1);
  assert.equal(component.state.runStart, now);
  assert.equal(component.state.lastActiveAt, now);
});

test('screen inactivity pauses at the native event timestamp', () => {
  const now = new Date(2026, 7, 30, 15, 0, 0).getTime();
  const { component } = createComponent({ now });
  component.state = savedState({ runStart: now - 10_000, lastActiveAt: now - 5_000 });

  component.pauseAt(now - 3_000);

  assert.equal(component.state.projects[0].sec, 107);
  assert.equal(component.state.runStart, null);
  assert.equal(component.state.lastActiveAt, null);
});

test('selected history range is persisted', () => {
  const now = new Date(2026, 7, 30, 15, 0, 0).getTime();
  const { component, localStorage } = createComponent({ now });

  component.renderVals().setOverviewRange30();

  const saved = JSON.parse(localStorage.getItem('lava-timer-v2'));
  assert.equal(component.state.overviewRange, '30');
  assert.equal(saved.overviewRange, '30');
});

test('destructive reset and delete actions require confirmation', () => {
  const now = new Date(2026, 7, 30, 15, 0, 0).getTime();
  const { component, window } = createComponent({ now });
  component.state = savedState();

  window.confirm = () => false;
  let values = component.renderVals();
  values.reset();
  values.projectEdits[1].onDelete();
  assert.equal(component.state.projects[0].sec, 100);
  assert.equal(component.state.projects.length, 2);

  window.confirm = () => true;
  values = component.renderVals();
  values.reset();
  values = component.renderVals();
  values.projectEdits[1].onDelete();
  assert.equal(component.state.projects[0].sec, 0);
  assert.equal(component.state.projects.length, 1);
});

test('macOS bundle includes the tray icon required during setup', () => {
  const config = JSON.parse(
    readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'),
  );

  assert.ok(config.bundle.resources.includes('icons/tray.png'));
});

test('main window is visible on launch and Dock remains as a recovery entry', () => {
  const config = JSON.parse(
    readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'),
  );
  const nativeSource = readFileSync(
    new URL('../src-tauri/src/lib.rs', import.meta.url),
    'utf8',
  );

  assert.equal(config.app.windows[0].visible, true);
  assert.doesNotMatch(nativeSource, /ActivationPolicy::Accessory/);
  assert.match(nativeSource, /RunEvent::Reopen/);
});

test('every view exposes a close-window action', () => {
  assert.ok((html.match(/onClick="\{\{ hideWindow \}\}"/g) || []).length >= 4);
});

test('frontend runtime is bundled locally and loaded before support.js', () => {
  const reactScript = './vendor/react.production.min.js';
  const reactDomScript = './vendor/react-dom.production.min.js';
  const supportScript = './support.js';

  assert.ok(html.indexOf(reactScript) < html.indexOf(supportScript));
  assert.ok(html.indexOf(reactDomScript) < html.indexOf(supportScript));
  assert.ok(readFileSync(new URL(`../src/${reactScript.slice(2)}`, import.meta.url)).length > 0);
  assert.ok(readFileSync(new URL(`../src/${reactDomScript.slice(2)}`, import.meta.url)).length > 0);
});
