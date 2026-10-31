import type { Page } from '@playwright/test';

/**
 * Stand-in for the Tauri IPC: defines `window.__TAURI_INTERNALS__.invoke` so the pages take
 * their desktop branch in a plain browser.
 * - `responses`: answer per command; `{ __error: { code, message } }` rejects; no entry
 *   rejects with `tauri_unavailable`.
 * - `state.prefs`: backs `get_app_preferences` / `set_app_preferences`; see `setStubPrefs`.
 * - `ipcCalls` lists every command received.
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
  __sobriaIpc: { calls: IpcCall[]; state: StubState };
}

export async function installTauriStub(page: Page, stub: IpcStub): Promise<void> {
  await page.addInitScript((options) => {
    const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value ?? null));
    const calls: IpcCall[] = [];
    const state: StubState = clone(options.state ?? {});
    const invoke = async (cmd: string, args?: Record<string, unknown>) => {
      calls.push({ cmd, args: clone(args) });
      const answer = options.responses[cmd] as { __error?: unknown } | undefined;
      if (answer && typeof answer === 'object' && '__error' in answer) throw answer.__error;
      if (state.prefs && cmd === 'get_app_preferences') return clone(state.prefs);
      if (state.prefs && cmd === 'set_app_preferences') {
        state.prefs = clone(args?.prefs) as Record<string, unknown>;
        return null;
      }
      if (cmd in options.responses) return clone(options.responses[cmd]);
      throw { code: 'tauri_unavailable', message: `stub: no answer for ${cmd}` };
    };
    Object.assign(window, {
      __TAURI_INTERNALS__: { invoke, transformCallback: () => 0, metadata: {} },
      __sobriaIpc: { calls, state }
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
    (window as unknown as StubWindow).__sobriaIpc.state.prefs = next;
  }, prefs);
}
