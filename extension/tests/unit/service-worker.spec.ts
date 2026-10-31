// SOB-158 — characterization of `background/service-worker.ts`: the messages
// it accepts, what it answers, what it stores and where it forwards an
// estimation (native bridge, team server, or both). The real worker is
// imported against an in-memory chrome.storage, a fake native bridge and a
// stubbed fetch.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { todayKey } from '../../src/content/shared/storage.js';
import { registryLabel } from '../../src/lib/registry-meta.js';
import type { Estimate } from '../../src/lib/types.js';
import {
  createFakePort,
  eventually,
  installChrome,
  type BridgeRequest
} from './helpers/chrome-stub.js';

// Importing the real scripts is slow on a loaded machine (coverage, parallel builds).
vi.setConfig({ testTimeout: 30_000, hookTimeout: 30_000 });

const NOW = new Date('2026-05-16T12:00:00.000Z');
const NOW_ISO = NOW.toISOString();
const TODAY = todayKey(NOW);

const ESTIMATE = {
  method: 'afnor_sobria',
  modelId: 'gpt-4o',
  gco2eq: 0.5,
  waterMl: 2,
  energyWh: 0.25
} as unknown as Estimate;

const SUBMISSION = {
  type: 'estimation_submitted',
  estimate: ESTIMATE,
  host: 'chatgpt',
  modelDisplayName: 'GPT-4o'
} as const;

const PAIRING = { secret: 's3cret', pairingId: 'pair-1', fingerprint: 'fp-1', pairedAt: 'earlier' };

type BridgeReply = (request: BridgeRequest) => Record<string, unknown> | undefined;

/** A native bridge: every connection gets a fresh fake port answering with `reply`. */
function bridgeFactory(reply: BridgeReply) {
  const ports: Array<ReturnType<typeof createFakePort>> = [];
  const sent = (): BridgeRequest[] => ports.flatMap((p) => p.sent());
  return {
    ports,
    connectNative: () => {
      const fake = createFakePort((request) =>
        request.type === 'ping' ? { ok: true, pong: true } : reply(request)
      );
      ports.push(fake);
      return fake.port;
    },
    sent,
    ofType: (type: string) => sent().filter((request) => request.type === type)
  };
}

type Bridge = ReturnType<typeof bridgeFactory>;

let stub: ReturnType<typeof installChrome>;

/** Imports a fresh worker and returns a function that sends it a message. */
async function startWorker(bridge?: Bridge) {
  stub = installChrome(bridge ? { connectNative: bridge.connectNative } : {});
  vi.resetModules();
  await import('../../src/background/service-worker.js');
  const listener = stub.messageListeners[0];
  if (!listener) throw new Error('the worker registered no onMessage listener');
  return (message: object, senderUrl?: string): Promise<unknown> =>
    new Promise((resolve) => {
      const keepsChannelOpen = listener(
        message,
        senderUrl ? { tab: { url: senderUrl } } : {},
        resolve
      );
      expect(keepsChannelOpen).toBe(true);
    });
}

function stubFetch(handler: (url: string, init: RequestInit) => Response | Promise<Response>) {
  const fetchMock = vi.fn(async (url: string, init: RequestInit = {}) => handler(url, init));
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

/** Lets the fire-and-forget forwards of the worker run, before asserting that one did not happen. */
const settle = (): Promise<void> => new Promise((resolve) => setTimeout(resolve, 25));

const jsonResponse = (body: unknown, status = 200): Response =>
  new Response(JSON.stringify(body), { status });

async function enrollInTeam(mode: 'local' | 'team' | 'both'): Promise<void> {
  await stub.local.set({
    team_url: 'https://team.example',
    team_mode: mode,
    team_access: 'jwt-1',
    team_refresh: 'ulid.uuid'
  });
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] });
  vi.setSystemTime(NOW);
  vi.spyOn(console, 'info').mockImplementation(() => undefined);
  vi.spyOn(console, 'warn').mockImplementation(() => undefined);
  vi.spyOn(console, 'error').mockImplementation(() => undefined);
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('lifecycle', () => {
  it('logs the registry label when installed and when activated', async () => {
    await startWorker();
    self.dispatchEvent(new Event('install'));
    self.dispatchEvent(new Event('activate'));
    expect(console.info).toHaveBeenCalledWith(
      `[sobria] service worker installé (${registryLabel()})`
    );
    expect(console.info).toHaveBeenCalledWith('[sobria] service worker activé');
  });
});

describe('simple messages', () => {
  it('get_method answers the default method, then the one set by set_method', async () => {
    const send = await startWorker();
    expect(await send({ type: 'get_method' })).toEqual({ method: 'afnor_sobria' });
    expect(await send({ type: 'set_method', method: 'ecologits' })).toEqual({ ok: true });
    expect(await send({ type: 'get_method' })).toEqual({ method: 'ecologits' });
  });

  it('get_daily_total answers zeros for the local day when nothing was measured', async () => {
    const send = await startWorker();
    expect(await send({ type: 'get_daily_total' })).toEqual({
      total: { date: TODAY, count: 0, gco2eq: 0, waterMl: 0, energyWh: 0 }
    });
  });

  it('get_last_estimate answers null when nothing was measured', async () => {
    const send = await startWorker();
    expect(await send({ type: 'get_last_estimate' })).toEqual({ last: null });
  });

  it('purge_data clears the whole storage: settings, pairing and Team mode', async () => {
    const send = await startWorker();
    await send({ type: 'set_method', method: 'ecologits' });
    await send(SUBMISSION);
    await stub.local.set({ pairing_state: PAIRING, team_url: 'https://team.example' });

    expect(await send({ type: 'purge_data' })).toEqual({ ok: true });

    expect(stub.local.data.size).toBe(0);
    expect(await send({ type: 'get_method' })).toEqual({ method: 'afnor_sobria' });
  });
});

describe('estimation_submitted — local storage', () => {
  it('adds the estimate to the daily total and keeps it as the last estimate', async () => {
    const send = await startWorker();

    expect(await send(SUBMISSION)).toEqual({ ok: true });
    expect(await send({ ...SUBMISSION, host: 'claude', modelDisplayName: 'Claude' })).toEqual({
      ok: true
    });

    expect(await send({ type: 'get_daily_total' })).toEqual({
      total: { date: TODAY, count: 2, gco2eq: 1, waterMl: 4, energyWh: 0.5 }
    });
    expect(await send({ type: 'get_last_estimate' })).toEqual({
      last: { estimate: ESTIMATE, host: 'claude', modelDisplayName: 'Claude', ts: NOW_ISO }
    });
  });

  // Known bug, fixed by SOB-118: the content script already adds the estimate
  // to the daily total before sending it, and the worker adds it again.
  it('counts again an estimate the content script already counted (double counting, SOB-118)', async () => {
    const send = await startWorker();
    const { appendToDailyTotal } = await import('../../src/content/shared/storage.js');
    await appendToDailyTotal({ gco2eq: 0.5, waterMl: 2, energyWh: 0.25 }); // content script

    await send(SUBMISSION); // same estimate, sent to the worker

    expect(await send({ type: 'get_daily_total' })).toEqual({
      total: { date: TODAY, count: 2, gco2eq: 1, waterMl: 4, energyWh: 0.5 }
    });
  });

  it('does not contact the bridge nor the team server by default (local mode, not paired)', async () => {
    const bridge = bridgeFactory(() => ({ ok: true }));
    const send = await startWorker(bridge);
    const fetchMock = stubFetch(() => jsonResponse({}));

    await send(SUBMISSION);
    await settle();

    expect(stub.runtime.connectNative).not.toHaveBeenCalled();
    expect(fetchMock).not.toHaveBeenCalled();
  });
});

describe('estimation_submitted — native bridge (local and both modes)', () => {
  it('forwards the estimate with the pairing secret, then closes the connection', async () => {
    const bridge = bridgeFactory(() => ({ ok: true }));
    const send = await startWorker(bridge);
    await stub.local.set({ pairing_state: PAIRING });

    expect(await send(SUBMISSION)).toEqual({ ok: true });

    await eventually(() => expect(bridge.ofType('estimate')).toHaveLength(1));
    expect(bridge.ofType('estimate')[0]).toEqual({
      type: 'estimate',
      reqId: expect.any(String),
      secret: 's3cret',
      payload: {
        estimate: ESTIMATE,
        host: 'chatgpt',
        modelDisplayName: 'GPT-4o',
        ts: NOW_ISO
      }
    });
    await eventually(() => expect(bridge.ports[0]?.port.disconnect).toHaveBeenCalled());
    expect(bridge.ports).toHaveLength(1);
  });

  it('answers ok even when the bridge is not installed', async () => {
    const send = await startWorker();
    await stub.local.set({ pairing_state: PAIRING });
    expect(await send(SUBMISSION)).toEqual({ ok: true });
    expect(
      ((await send({ type: 'get_daily_total' })) as { total: { count: number } }).total.count
    ).toBe(1);
  });

  it('answers ok and logs a warning when the forward fails', async () => {
    const bridge = bridgeFactory((request) => {
      if (request.type === 'estimate') throw new Error('port closed');
      return { ok: true };
    });
    const send = await startWorker(bridge);
    await stub.local.set({ pairing_state: PAIRING });

    expect(await send(SUBMISSION)).toEqual({ ok: true });

    await eventually(() =>
      expect(console.warn).toHaveBeenCalledWith('[sobria] forward bridge échec:', expect.any(Error))
    );
  });

  it('forwards to the bridge in both mode too, but not in team mode', async () => {
    const bridge = bridgeFactory(() => ({ ok: true }));
    const send = await startWorker(bridge);
    stubFetch(() => jsonResponse({ id: 'x', ack: true }));
    await stub.local.set({ pairing_state: PAIRING });

    await enrollInTeam('both');
    await send(SUBMISSION);
    await eventually(() => expect(bridge.ofType('estimate')).toHaveLength(1));

    await enrollInTeam('team');
    await send(SUBMISSION);
    await settle();
    expect(bridge.ofType('estimate')).toHaveLength(1);
  });
});

describe('estimation_submitted — team server (team and both modes)', () => {
  const CHAT_URL = 'https://claude.ai/chat/abc-123?x=1';

  it('pushes the estimate to the server with the bearer token', async () => {
    const send = await startWorker();
    const fetchMock = stubFetch(() => jsonResponse({ id: 'est-1', ack: true }));
    await enrollInTeam('team');

    expect(await send(SUBMISSION)).toEqual({ ok: true });

    await eventually(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe('https://team.example/api/v1/estimations');
    expect(init.method).toBe('POST');
    expect(new Headers(init.headers).get('authorization')).toBe('Bearer jwt-1');
    expect(JSON.parse(init.body as string)).toEqual({
      estimate: ESTIMATE,
      host: 'chatgpt',
      modelDisplayName: 'GPT-4o',
      ts: NOW_ISO
    });
  });

  it('adds the project assigned to the conversation of the sender tab', async () => {
    const send = await startWorker();
    const fetchMock = stubFetch(() => jsonResponse({ id: 'est-1', ack: true }));
    await enrollInTeam('team');
    await stub.local.set({ sobria_thread_projects: { 'claude.ai/chat/abc-123': 'Alpha' } });

    await send(SUBMISSION, CHAT_URL);

    await eventually(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    const init = fetchMock.mock.calls[0]?.[1] as RequestInit;
    expect((JSON.parse(init.body as string) as { project?: string }).project).toBe('Alpha');
  });

  it('sends no project label for a conversation without project, an unknown sender or a non-https tab', async () => {
    const send = await startWorker();
    const fetchMock = stubFetch(() => jsonResponse({ id: 'est-1', ack: true }));
    await enrollInTeam('team');
    await stub.local.set({ sobria_thread_projects: { 'claude.ai/chat/abc-123': 'Alpha' } });

    await send(SUBMISSION, 'https://claude.ai/chat/other');
    await send(SUBMISSION);
    await send(SUBMISSION, 'http://claude.ai/chat/abc-123');

    await eventually(() => expect(fetchMock).toHaveBeenCalledTimes(3));
    for (const [, init] of fetchMock.mock.calls) {
      expect(JSON.parse((init as RequestInit).body as string)).not.toHaveProperty('project');
    }
  });

  it('does not push when the user is not enrolled', async () => {
    const send = await startWorker();
    const fetchMock = stubFetch(() => jsonResponse({}));
    await stub.local.set({ team_url: 'https://team.example', team_mode: 'team' });

    await send(SUBMISSION);
    await settle();

    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('does not push in local mode, even when enrolled', async () => {
    const send = await startWorker();
    const fetchMock = stubFetch(() => jsonResponse({}));
    await enrollInTeam('local');

    await send(SUBMISSION);
    await settle();

    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('pushes in both mode', async () => {
    const send = await startWorker();
    const fetchMock = stubFetch(() => jsonResponse({ id: 'est-1', ack: true }));
    await enrollInTeam('both');

    await send(SUBMISSION);

    await eventually(() => expect(fetchMock).toHaveBeenCalledTimes(1));
  });

  it('answers ok and logs a warning when the server cannot be reached', async () => {
    const send = await startWorker();
    stubFetch(() => {
      throw new TypeError('Failed to fetch');
    });
    await enrollInTeam('team');

    expect(await send(SUBMISSION)).toEqual({ ok: true });

    await eventually(() =>
      expect(console.warn).toHaveBeenCalledWith(
        '[sobria] forward team échec:',
        expect.any(TypeError)
      )
    );
    expect(
      ((await send({ type: 'get_daily_total' })) as { total: { count: number } }).total.count
    ).toBe(1);
  });
});

describe('get_pairing_status', () => {
  it('is not paired and without bridge on a fresh install', async () => {
    const send = await startWorker();
    expect(await send({ type: 'get_pairing_status' })).toEqual({
      status: { paired: false, bridgeAvailable: false }
    });
  });

  it('reports the bridge as available when it answers the ping', async () => {
    const bridge = bridgeFactory(() => ({ ok: true }));
    const send = await startWorker(bridge);
    expect(await send({ type: 'get_pairing_status' })).toEqual({
      status: { paired: false, bridgeAvailable: true }
    });
    expect(bridge.ports[0]?.port.disconnect).toHaveBeenCalled();
  });

  it('describes the pairing without ever exposing the secret', async () => {
    const bridge = bridgeFactory(() => ({ ok: true }));
    const send = await startWorker(bridge);
    await stub.local.set({ pairing_state: PAIRING });

    const answer = (await send({ type: 'get_pairing_status' })) as { status: object };

    expect(answer).toEqual({
      status: {
        paired: true,
        pairingId: 'pair-1',
        fingerprint: 'fp-1',
        pairedAt: 'earlier',
        bridgeAvailable: true
      }
    });
    expect(JSON.stringify(answer)).not.toContain('s3cret');
  });

  it('stays paired when the bridge is gone', async () => {
    const send = await startWorker();
    await stub.local.set({ pairing_state: PAIRING });
    expect(await send({ type: 'get_pairing_status' })).toMatchObject({
      status: { paired: true, bridgeAvailable: false }
    });
  });
});

describe('pair_with_code', () => {
  it('refuses with a message when the bridge is not installed', async () => {
    const send = await startWorker();
    expect(await send({ type: 'pair_with_code', code: '123456' })).toEqual({
      ok: false,
      error: 'Bridge natif indisponible — installer l’app Sobr.ia desktop.'
    });
    expect(stub.local.data.has('pairing_state')).toBe(false);
  });

  it('stores the pairing and answers its status (without the secret) when the code is valid', async () => {
    const bridge = bridgeFactory((request) =>
      request.type === 'pair'
        ? { ok: true, secret: 'new-secret', pairingId: 'pair-9', fingerprint: 'fp-9' }
        : { ok: true }
    );
    const send = await startWorker(bridge);

    const answer = await send({ type: 'pair_with_code', code: '123456' });

    expect(answer).toEqual({
      ok: true,
      status: {
        paired: true,
        pairingId: 'pair-9',
        fingerprint: 'fp-9',
        pairedAt: NOW_ISO,
        bridgeAvailable: true
      }
    });
    expect(bridge.ofType('pair')[0]).toMatchObject({ code: '123456' });
    expect(stub.local.data.get('pairing_state')).toEqual({
      secret: 'new-secret',
      pairingId: 'pair-9',
      fingerprint: 'fp-9',
      pairedAt: NOW_ISO
    });
    expect(bridge.ports[0]?.port.disconnect).toHaveBeenCalled();
  });

  it('relays the error of the bridge, or a default message', async () => {
    const answers: Array<Record<string, unknown>> = [
      { ok: false, error: 'Code expiré' },
      { ok: false },
      { ok: true, secret: 's', pairingId: 'p' } // fingerprint missing
    ];
    const bridge = bridgeFactory((request) =>
      request.type === 'pair' ? answers.shift() : { ok: true }
    );
    const send = await startWorker(bridge);

    expect(await send({ type: 'pair_with_code', code: '1' })).toEqual({
      ok: false,
      error: 'Code expiré'
    });
    expect(await send({ type: 'pair_with_code', code: '2' })).toEqual({
      ok: false,
      error: 'Code invalide ou expiré.'
    });
    expect(await send({ type: 'pair_with_code', code: '3' })).toEqual({
      ok: false,
      error: 'Code invalide ou expiré.'
    });
    expect(stub.local.data.has('pairing_state')).toBe(false);
  });
});

describe('revoke_pairing', () => {
  it('asks the bridge to revoke the secret, then forgets the pairing', async () => {
    const bridge = bridgeFactory(() => ({ ok: true }));
    const send = await startWorker(bridge);
    await stub.local.set({ pairing_state: PAIRING });

    expect(await send({ type: 'revoke_pairing' })).toEqual({ ok: true });

    expect(bridge.ofType('revoke')).toEqual([
      { type: 'revoke', reqId: expect.any(String), secret: 's3cret' }
    ]);
    expect(stub.local.data.has('pairing_state')).toBe(false);
  });

  it('forgets the pairing even when the bridge refuses or fails', async () => {
    const bridge = bridgeFactory((request) => {
      if (request.type === 'revoke') throw new Error('port closed');
      return { ok: true };
    });
    const send = await startWorker(bridge);
    await stub.local.set({ pairing_state: PAIRING });

    expect(await send({ type: 'revoke_pairing' })).toEqual({ ok: true });
    expect(stub.local.data.has('pairing_state')).toBe(false);
  });

  it('forgets the pairing when the bridge is not installed', async () => {
    const send = await startWorker();
    await stub.local.set({ pairing_state: PAIRING });
    expect(await send({ type: 'revoke_pairing' })).toEqual({ ok: true });
    expect(stub.local.data.has('pairing_state')).toBe(false);
  });

  it('does not contact the bridge when there is no pairing', async () => {
    const bridge = bridgeFactory(() => ({ ok: true }));
    const send = await startWorker(bridge);
    expect(await send({ type: 'revoke_pairing' })).toEqual({ ok: true });
    expect(stub.runtime.connectNative).not.toHaveBeenCalled();
  });
});

describe('errors', () => {
  it('answers ok: false with the error text for an unknown message type, and logs it', async () => {
    const send = await startWorker();
    // The text names no type: the worker stringifies the whole message object.
    expect(await send({ type: 'does_not_exist' })).toEqual({
      ok: false,
      error: 'Error: message type inconnu: [object Object]'
    });
    expect(console.error).toHaveBeenCalledWith('[sobria] handleMessage error :', expect.any(Error));
  });

  it('answers ok: false when the storage fails', async () => {
    const send = await startWorker();
    stub.local.get.mockRejectedValueOnce(new Error('quota exceeded'));
    expect(await send({ type: 'get_method' })).toEqual({
      ok: false,
      error: 'Error: quota exceeded'
    });
  });
});
