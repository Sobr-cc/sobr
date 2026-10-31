// Test helpers for the SOB-158 characterization tests: an in-memory
// `chrome.storage.local`, a stub of the `chrome.runtime` calls the extension
// makes, a fake native messaging port, a `fetch` stub and a few small
// utilities. Not a spec file: Vitest only collects `*.spec.ts` and `*.test.ts`.

import { vi } from 'vitest';
import type { Estimate } from '../../../src/lib/types.js';

export type StorageData = Record<string, unknown>;

/** Chrome serializes the values it stores as JSON (`undefined` is dropped). */
const serialize = <T>(value: T): T =>
  value === undefined ? value : (JSON.parse(JSON.stringify(value)) as T);

/**
 * In-memory `chrome.storage.local`. Values go through JSON on the way in and
 * out, so a test cannot depend on object identity or on a value that Chrome
 * would not keep.
 */
function createStorageLocal() {
  const data = new Map<string, unknown>();
  return {
    data,
    get: vi.fn(async (keys?: string | string[] | StorageData | null) => {
      if (keys === undefined || keys === null) {
        return Object.fromEntries([...data].map(([key, value]) => [key, serialize(value)]));
      }
      const defaults: StorageData = typeof keys === 'string' || Array.isArray(keys) ? {} : keys;
      const names =
        typeof keys === 'string' ? [keys] : Array.isArray(keys) ? keys : Object.keys(keys);
      const out: StorageData = {};
      for (const name of names) {
        if (data.has(name)) out[name] = serialize(data.get(name));
        else if (name in defaults) out[name] = defaults[name];
      }
      return out;
    }),
    set: vi.fn(async (items: StorageData) => {
      for (const [key, value] of Object.entries(items)) {
        if (value !== undefined) data.set(key, serialize(value));
      }
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

/** Answer of the fake bridge: fields of the reply, or `DISCONNECT`, or nothing. */
export type BridgeAnswer = Record<string, unknown>;
export type BridgeReply = (request: BridgeRequest) => BridgeAnswer | typeof DISCONNECT | undefined;

/**
 * Returned by a `BridgeReply` to make the browser report the native host as
 * unavailable through `onDisconnect`, which is how Chrome and Firefox tell a
 * missing or crashed host once the port is open.
 */
export const DISCONNECT = Symbol('disconnect');

/** A bridge that answers pings, and everything else with `more`. */
export function withPing(more: BridgeReply = () => undefined): BridgeReply {
  return (request) => (request.type === 'ping' ? { ok: true, pong: true } : more(request));
}

/**
 * Fake native messaging port. The answer of `reply` is delivered in a
 * microtask with the `reqId` of the request; returning `undefined` means the
 * bridge stays silent.
 */
export function createFakePort(reply: BridgeReply = () => undefined) {
  const messageListeners: Array<(message: unknown) => void> = [];
  const disconnectListeners: Array<() => void> = [];
  const emit = (message: unknown): void => {
    for (const listener of messageListeners) listener(message);
  };
  const emitDisconnect = (): void => {
    for (const listener of disconnectListeners) listener();
  };
  const port = {
    onMessage: {
      addListener: (listener: (message: unknown) => void) => messageListeners.push(listener)
    },
    onDisconnect: { addListener: (listener: () => void) => disconnectListeners.push(listener) },
    postMessage: vi.fn((request: BridgeRequest) => {
      const answer = reply(request);
      if (answer === DISCONNECT) queueMicrotask(emitDisconnect);
      else if (answer) queueMicrotask(() => emit({ reqId: request.reqId, ...answer }));
    }),
    disconnect: vi.fn()
  };
  return {
    port,
    emit,
    emitDisconnect,
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

export type ChromeStub = ReturnType<typeof installChrome>;

// ─── fetch ───────────────────────────────────────────────────────────────────

/** Replaces `fetch` with `handler` and returns the mock. */
export function stubFetch(
  handler: (url: string, init: RequestInit) => Response | Promise<Response>
) {
  const fetchMock = vi.fn<(url: string, init?: RequestInit) => Promise<Response>>(
    async (url, init = {}) => handler(url, init)
  );
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

export const jsonResponse = (body: unknown, status = 200): Response =>
  new Response(JSON.stringify(body), { status });

/** The n-th call of a `stubFetch` mock, headers lower-cased and JSON body parsed. */
export function request(fetchMock: ReturnType<typeof stubFetch>, n: number) {
  const [url, init = {}] = fetchMock.mock.calls[n] ?? [];
  const headers: Record<string, string> = {};
  new Headers(init.headers).forEach((value, key) => {
    headers[key] = value;
  });
  return {
    url,
    method: init.method,
    headers,
    body: typeof init.body === 'string' ? (JSON.parse(init.body) as unknown) : init.body
  };
}

// ─── small utilities ─────────────────────────────────────────────────────────

/**
 * Waits until an assertion holds. The forwards of the worker and the bootstrap
 * of the content scripts are asynchronous; the default timeout of `vi.waitFor`
 * (1 s) is too short on a loaded machine.
 */
export const eventually = <T>(assertion: () => T | Promise<T>): Promise<T> =>
  vi.waitFor(assertion, { timeout: 8000, interval: 2 });

/**
 * Runs everything that is already queued (promises, microtasks) before an
 * assertion that something did NOT happen. The stubs answer in microtasks, so
 * one turn of the event loop is enough, however loaded the machine is.
 */
export const flush = (): Promise<void> => new Promise((resolve) => setTimeout(resolve, 0));

/** Importing the real worker or content scripts is slow on a loaded machine. */
export function allowSlowImports(): void {
  vi.setConfig({ testTimeout: 30_000, hookTimeout: 30_000 });
}

/** Silences the given console methods and returns their spies. */
export function silenceConsole(...methods: Array<'info' | 'warn' | 'error'>) {
  return methods.map((method) => vi.spyOn(console, method).mockImplementation(() => undefined));
}

/** An estimate with the three quantities that the extension adds up. */
export function fakeEstimate(overrides: StorageData = {}): Estimate {
  return {
    method: 'afnor_sobria',
    modelId: 'gpt-4o',
    gco2eq: 0.5,
    waterMl: 2,
    energyWh: 0.25,
    ...overrides
  } as unknown as Estimate;
}
