import type { Page } from '@playwright/test';

/**
 * A small stand-in for the Tauri IPC, to run the desktop code paths of the app in a plain
 * browser. It defines `window.__TAURI_INTERNALS__.invoke` (what `isTauriContext()` looks for and
 * what `@tauri-apps/api/core` calls), so the pages take their desktop branch and every command
 * is answered from the fixtures given here instead of the demo data.
 *
 * - `responses`: the answer of each command. An answer `{ __error: { code, message } }` rejects.
 *   A command without an answer rejects with `tauri_unavailable`.
 * - `state.prefs`: when given, `get_app_preferences` and `set_app_preferences` read and write it,
 *   like the real command does; a test can change it with `setStubState` to simulate another
 *   window or page saving the settings.
 * - `calls`: every command received, with its arguments, readable with `ipcCalls`.
 */
export interface IpcStub {
  responses: Record<string, unknown>;
  state?: { prefs?: Record<string, unknown> };
}

export interface IpcCall {
  cmd: string;
  args: unknown;
}

interface StubWindow {
  __sobriaIpc: { calls: IpcCall[]; state: { prefs?: Record<string, unknown> } };
}

export async function installTauriStub(page: Page, stub: IpcStub): Promise<void> {
  await page.addInitScript((options) => {
    const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value ?? null));
    const calls: { cmd: string; args: unknown }[] = [];
    const state: { prefs?: Record<string, unknown> } = clone(options.state ?? {});
    const invoke = async (cmd: string, args?: Record<string, unknown>) => {
      calls.push({ cmd, args: clone(args) });
      if (state.prefs && cmd === 'get_app_preferences') return clone(state.prefs);
      if (state.prefs && cmd === 'set_app_preferences') {
        state.prefs = clone(args?.prefs) as Record<string, unknown>;
        return null;
      }
      const answer = options.responses[cmd] as { __error?: unknown } | undefined;
      if (answer && typeof answer === 'object' && '__error' in answer) throw answer.__error;
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

/** Preferences as the desktop app returns them once the onboarding is done. */
export const ONBOARDED_PREFS = {
  persona: null,
  enabled_modules: ['m1', 'm3', 'm7', 'm8', 'm9', 'm12', 'm13', 'm15', 'm17', 'm20', 'm22', 'm25'],
  onboarded: true,
  lang: 'fr',
  default_method: 'afnor_sobria',
  also_show_methods: ['ecologits']
};
