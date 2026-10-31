import type { Page } from '@playwright/test';

/**
 * Stand-in for the Tauri IPC: defines `window.__TAURI_INTERNALS__.invoke` so the pages take
 * their desktop branch in a plain browser.
 * - `responses`: answer per command; `{ __error: { code, message } }` rejects; no entry
 *   rejects with `tauri_unavailable`.
 * - `state.prefs`: backs `get_app_preferences` / `set_app_preferences`; kept in `sessionStorage`,
 *   so it survives a reload; change it from a test with `setStubPrefs`.
 * - `ipcCalls` lists the commands received since the page loaded (it restarts on a reload).
 */
type StubState = { prefs?: Record<string, unknown> };

export interface IpcStub {
  responses: Record<string, unknown>;
  state?: StubState;
}

export interface IpcCall {
  cmd: string;
  args: unknown;
}

interface StubWindow {
  __sobriaIpc: {
    calls: IpcCall[];
    state: StubState;
    setPrefs(prefs: Record<string, unknown>): void;
  };
}

export async function installTauriStub(page: Page, stub: IpcStub): Promise<void> {
  await page.addInitScript((options) => {
    const KEY = '__sobriaIpcState';
    const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value ?? null));
    const calls: IpcCall[] = [];
    let state: StubState = clone(options.state ?? {});
    try {
      const kept = sessionStorage.getItem(KEY);
      if (kept) state = JSON.parse(kept) as StubState;
    } catch {
      // No storage on this document: the state then lasts one page load.
    }
    const save = () => {
      try {
        sessionStorage.setItem(KEY, JSON.stringify(state));
      } catch {
        // See above.
      }
    };
    const setPrefs = (prefs: Record<string, unknown>) => {
      state.prefs = clone(prefs);
      save();
    };
    const invoke = async (cmd: string, args?: Record<string, unknown>) => {
      calls.push({ cmd, args: clone(args) });
      const answer = options.responses[cmd] as { __error?: unknown } | undefined;
      if (answer && typeof answer === 'object' && '__error' in answer) throw answer.__error;
      if (state.prefs && cmd === 'get_app_preferences') return clone(state.prefs);
      if (state.prefs && cmd === 'set_app_preferences') {
        setPrefs(args?.prefs as Record<string, unknown>);
        return null;
      }
      if (cmd in options.responses) return clone(options.responses[cmd]);
      throw { code: 'tauri_unavailable', message: `stub: no answer for ${cmd}` };
    };
    Object.defineProperty(window, '__sobriaIpc', {
      value: {
        calls,
        setPrefs,
        get state() {
          return state;
        }
      }
    });
    Object.assign(window, {
      __TAURI_INTERNALS__: { invoke, transformCallback: () => 0, metadata: {} }
    });
  }, stub);
}

export async function ipcCalls(page: Page, cmd?: string): Promise<IpcCall[]> {
  const calls = await page.evaluate(() => (window as unknown as StubWindow).__sobriaIpc.calls);
  return cmd === undefined ? calls : calls.filter((c) => c.cmd === cmd);
}

export async function stubPrefs(page: Page): Promise<Record<string, unknown> | undefined> {
  return page.evaluate(() => (window as unknown as StubWindow).__sobriaIpc.state.prefs);
}

export async function setStubPrefs(page: Page, prefs: Record<string, unknown>): Promise<void> {
  await page.evaluate((next) => {
    (window as unknown as StubWindow).__sobriaIpc.setPrefs(next);
  }, prefs);
}
