// Test helpers for the SOB-158 characterization tests: an in-memory
// `chrome.storage.local`, a stub of the `chrome.runtime` calls the extension
// makes, and a fake native messaging port. Not a spec file: Vitest only
// collects `*.spec.ts` and `*.test.ts`.

import { vi } from 'vitest';

export type StorageData = Record<string, unknown>;

/**
 * In-memory `chrome.storage.local`. Values are cloned on the way in and out,
 * like the real storage serializes them, so a test cannot depend on object
 * identity.
 */
export function createStorageLocal(initial: StorageData = {}) {
  const data = new Map<string, unknown>(Object.entries(structuredClone(initial)));
  return {
    data,
    get: vi.fn(async (keys: string | string[]) => {
      const out: StorageData = {};
      for (const key of Array.isArray(keys) ? keys : [keys]) {
        if (data.has(key)) out[key] = structuredClone(data.get(key));
      }
      return out;
    }),
    set: vi.fn(async (items: StorageData) => {
      for (const [key, value] of Object.entries(items)) data.set(key, structuredClone(value));
    }),
    remove: vi.fn(async (keys: string | string[]) => {
      for (const key of Array.isArray(keys) ? keys : [keys]) data.delete(key);
    }),
    clear: vi.fn(async () => {
      data.clear();
    })
  };
}

/** What the extension sends to the native bridge. */
export type BridgeRequest = { type: string; reqId: string; [field: string]: unknown };

/**
 * Fake native messaging port. `reply` decides what the bridge answers to each
 * request (the answer is delivered in a microtask, with the request's `reqId`);
 * returning `undefined` means the bridge stays silent.
 */
export function createFakePort(
  reply: (request: BridgeRequest) => StorageData | undefined = () => undefined
) {
  const messageListeners: Array<(message: unknown) => void> = [];
  const disconnectListeners: Array<() => void> = [];
  const emit = (message: unknown): void => {
    for (const listener of messageListeners) listener(message);
  };
  const port = {
    onMessage: {
      addListener: (listener: (message: unknown) => void) => messageListeners.push(listener)
    },
    onDisconnect: { addListener: (listener: () => void) => disconnectListeners.push(listener) },
    postMessage: vi.fn((request: BridgeRequest) => {
      const answer = reply(request);
      if (answer) queueMicrotask(() => emit({ reqId: request.reqId, ...answer }));
    }),
    disconnect: vi.fn()
  };
  return {
    port,
    emit,
    emitDisconnect: (): void => {
      for (const listener of disconnectListeners) listener();
    },
    sent: (): BridgeRequest[] => port.postMessage.mock.calls.map(([request]) => request)
  };
}

type Port = ReturnType<typeof createFakePort>['port'];

type ChromeStubOptions = {
  /** Absent by default: no native bridge installed. */
  connectNative?: (name: string) => Port;
};

/**
 * Installs a `chrome` global with the storage and the runtime calls used by
 * the extension. Call `vi.unstubAllGlobals()` afterwards.
 */
export function installChrome(options: ChromeStubOptions = {}) {
  const local = createStorageLocal();
  const messageListeners: Array<
    (
      message: unknown,
      sender: { tab?: { url?: string } },
      sendResponse: (response: unknown) => void
    ) => unknown
  > = [];
  const runtime = {
    onMessage: {
      addListener: vi.fn((listener: (typeof messageListeners)[number]) =>
        messageListeners.push(listener)
      )
    },
    sendMessage: vi.fn<(message: unknown) => Promise<unknown>>(async () => undefined),
    lastError: undefined as { message?: string } | undefined,
    // Undefined unless a bridge is installed, like in a browser without the native app.
    connectNative: options.connectNative ? vi.fn(options.connectNative) : undefined
  };
  vi.stubGlobal('chrome', { storage: { local }, runtime });
  return { local, runtime, messageListeners };
}

/**
 * Waits until an assertion holds. The forwards of the worker and the bootstrap
 * of the content scripts are asynchronous; the default timeout of `vi.waitFor`
 * (1 s) is too short on a loaded machine.
 */
export const eventually = <T>(assertion: () => T | Promise<T>): Promise<T> =>
  vi.waitFor(assertion, { timeout: 8000, interval: 20 });
