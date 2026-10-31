// SOB-158 — characterization of `background/service-worker.ts`: the messages
// it accepts, what it answers, what it stores and where it forwards an
// estimation (native bridge, team server, or both). The real worker is
// imported against an in-memory chrome.storage, a fake native bridge and a
// stubbed fetch.

import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { appendToDailyTotal, setPairingState, todayKey } from '../../src/content/shared/storage.js';
import { setProjectForThread } from '../../src/content/shared/projects.js';
import { setTeamMode, setTeamTokens, setTeamUrl } from '../../src/content/shared/team-storage.js';
import { registryLabel } from '../../src/lib/registry-meta.js';
import {
  allowSlowImports,
  createFakePort,
  DISCONNECT,
  eventually,
  fakeEstimate,
  flush,
  installChrome,
  jsonResponse,
  request,
  silenceConsole,
  stubFetch,
  withPing,
  type BridgeReply,
  type BridgeRequest,
  type ChromeStub
} from './helpers/chrome-stub.js';

allowSlowImports();

const NOW = new Date('2026-05-16T12:00:00.000Z');
const NOW_ISO = NOW.toISOString();
const TODAY = todayKey(NOW);

const ESTIMATE = fakeEstimate(); // 0.5 gCO2eq, 2 mL, 0.25 Wh

const SUBMISSION = {
  type: 'estimation_submitted',
  estimate: ESTIMATE,
  host: 'chatgpt',
  modelDisplayName: 'GPT-4o'
} as const;

const PAIRING = { secret: 's3cret', pairingId: 'pair-1', fingerprint: 'fp-1', pairedAt: 'earlier' };

/** A native bridge: every connection gets a fresh fake port, answering pings and then `reply`. */
function bridgeFactory(reply: BridgeReply) {
  const ports: Array<ReturnType<typeof createFakePort>> = [];
  const sent = (): BridgeRequest[] => ports.flatMap((p) => p.sent());
  return {
    ports,
    connectNative: () => {
      const fake = createFakePort(withPing(reply));
      ports.push(fake);
      return fake.port;
    },
    sent,
    ofType: (type: string) => sent().filter((request) => request.type === type)
  };
}

type Bridge = ReturnType<typeof bridgeFactory>;

let stub: ChromeStub;
let listener: ChromeStub['messageListeners'][number];

// The worker keeps no state of its own and reads the `chrome` global when it
// runs, so it is imported once; each test installs its own chrome stub.
beforeAll(async () => {
  const first = installChrome();
  await import('../../src/background/service-worker.js');
  const registered = first.messageListeners[0];
  if (!registered) throw new Error('the worker registered no onMessage listener');
  listener = registered;
  vi.unstubAllGlobals();
});

/** Installs a fresh chrome (with the given native bridge) and returns a function that sends the worker a message. */
function startWorker(bridge?: Pick<Bridge, 'connectNative'>) {
  stub = installChrome(bridge ? { connectNative: bridge.connectNative } : {});
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

async function enrollInTeam(mode: 'local' | 'team' | 'both'): Promise<void> {
  await setTeamUrl('https://team.example');
  await setTeamMode(mode);
  await setTeamTokens({ access: 'jwt-1', refresh: 'ulid.uuid' });
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] });
  vi.setSystemTime(NOW);
  silenceConsole('info', 'warn', 'error');
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

const DEV_LOG = expect.stringContaining('[sobria]');

describe('lifecycle', () => {
  it('logs the registry label when installed and when activated', () => {
    startWorker();
    self.dispatchEvent(new Event('install'));
    self.dispatchEvent(new Event('activate'));
    expect(console.info).toHaveBeenCalledWith(expect.stringContaining(registryLabel()));
    expect(console.info).toHaveBeenCalledTimes(2);
  });
});

describe('simple messages', () => {
  it('get_method answers the default method, then the one set by set_method', async () => {
    const send = startWorker();
    expect(await send({ type: 'get_method' })).toEqual({ method: 'afnor_sobria' });
    expect(await send({ type: 'set_method', method: 'ecologits' })).toEqual({ ok: true });
    expect(await send({ type: 'get_method' })).toEqual({ method: 'ecologits' });
  });

  it('get_daily_total answers zeros for the local day when nothing was measured', async () => {
    const send = startWorker();
    expect(await send({ type: 'get_daily_total' })).toEqual({
      total: { date: TODAY, count: 0, gco2eq: 0, waterMl: 0, energyWh: 0 }
    });
  });

  it('get_last_estimate answers null when nothing was measured', async () => {
    const send = startWorker();
    expect(await send({ type: 'get_last_estimate' })).toEqual({ last: null });
  });

  it('purge_data clears the whole storage: settings, pairing and Team mode', async () => {
    const send = startWorker();
    await send({ type: 'set_method', method: 'ecologits' });
    await send(SUBMISSION);
    await setPairingState(PAIRING);
    await setTeamUrl('https://team.example');

    expect(await send({ type: 'purge_data' })).toEqual({ ok: true });

    expect(stub.local.data.size).toBe(0);
    expect(await send({ type: 'get_method' })).toEqual({ method: 'afnor_sobria' });
  });
});

describe('estimation_submitted — local storage', () => {
  it('adds the estimate to the daily total and keeps it as the last estimate', async () => {
    const send = startWorker();

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
    const send = startWorker();
    await appendToDailyTotal({ gco2eq: 0.5, waterMl: 2, energyWh: 0.25 }); // content script

    await send(SUBMISSION); // same estimate, sent to the worker

    expect(await send({ type: 'get_daily_total' })).toEqual({
      total: { date: TODAY, count: 2, gco2eq: 1, waterMl: 4, energyWh: 0.5 }
    });
  });

  it('does not contact the bridge nor the team server by default (local mode, not paired)', async () => {
    const bridge = bridgeFactory(() => ({ ok: true }));
    const send = startWorker(bridge);
    const fetchMock = stubFetch(() => jsonResponse({}));

    await send(SUBMISSION);
    await flush();

    expect(stub.runtime.connectNative).not.toHaveBeenCalled();
    expect(fetchMock).not.toHaveBeenCalled();
  });
});

describe('estimation_submitted — native bridge (local and both modes)', () => {
  it('forwards the estimate with the pairing secret, then closes the connection', async () => {
    const bridge = bridgeFactory(() => ({ ok: true }));
    const send = startWorker(bridge);
    await setPairingState(PAIRING);

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
    const send = startWorker();
    await setPairingState(PAIRING);
    expect(await send(SUBMISSION)).toEqual({ ok: true });
    expect(await send({ type: 'get_daily_total' })).toMatchObject({ total: { count: 1 } });
  });

  it('answers ok and logs a warning when the forward fails', async () => {
    const bridge = bridgeFactory((request) => {
      if (request.type === 'estimate') throw new Error('port closed');
      return { ok: true };
    });
    const send = startWorker(bridge);
    await setPairingState(PAIRING);

    expect(await send(SUBMISSION)).toEqual({ ok: true });

    await eventually(() => expect(console.warn).toHaveBeenCalledWith(DEV_LOG, expect.any(Error)));
  });

  it('forwards to the bridge in both mode too, but not in team mode', async () => {
    const bridge = bridgeFactory(() => ({ ok: true }));
    const send = startWorker(bridge);
    stubFetch(() => jsonResponse({ id: 'x', ack: true }));
    await setPairingState(PAIRING);

    await enrollInTeam('both');
    await send(SUBMISSION);
    await eventually(() => expect(bridge.ofType('estimate')).toHaveLength(1));

    await enrollInTeam('team');
    await send(SUBMISSION);
    await flush();
    expect(bridge.ofType('estimate')).toHaveLength(1);
  });
});

describe('estimation_submitted — team server (team and both modes)', () => {
  const CHAT_URL = 'https://claude.ai/chat/abc-123?x=1';
  const CHAT_THREAD_KEY = 'claude.ai/chat/abc-123';

  it('pushes the estimate to the server with the bearer token', async () => {
    const send = startWorker();
    const fetchMock = stubFetch(() => jsonResponse({ id: 'est-1', ack: true }));
    await enrollInTeam('team');

    expect(await send(SUBMISSION)).toEqual({ ok: true });

    await eventually(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    expect(request(fetchMock, 0)).toMatchObject({
      url: 'https://team.example/api/v1/estimations',
      method: 'POST',
      headers: { authorization: 'Bearer jwt-1' },
      body: { estimate: ESTIMATE, host: 'chatgpt', modelDisplayName: 'GPT-4o', ts: NOW_ISO }
    });
  });

  it('adds the project assigned to the conversation of the sender tab', async () => {
    const send = startWorker();
    const fetchMock = stubFetch(() => jsonResponse({ id: 'est-1', ack: true }));
    await enrollInTeam('team');
    await setProjectForThread(CHAT_THREAD_KEY, 'Alpha');

    await send(SUBMISSION, CHAT_URL);

    await eventually(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    expect(request(fetchMock, 0).body).toMatchObject({ project: 'Alpha' });
  });

  it('sends no project label for a conversation without project, an unknown sender or a non-https tab', async () => {
    const send = startWorker();
    const fetchMock = stubFetch(() => jsonResponse({ id: 'est-1', ack: true }));
    await enrollInTeam('team');
    await setProjectForThread(CHAT_THREAD_KEY, 'Alpha');

    await send(SUBMISSION, 'https://claude.ai/chat/other');
    await send(SUBMISSION);
    await send(SUBMISSION, 'http://claude.ai/chat/abc-123');

    await eventually(() => expect(fetchMock).toHaveBeenCalledTimes(3));
    for (const n of [0, 1, 2]) {
      expect(request(fetchMock, n).body).not.toHaveProperty('project');
    }
  });

  it('does not push when the user is not enrolled', async () => {
    const send = startWorker();
    const fetchMock = stubFetch(() => jsonResponse({}));
    await setTeamUrl('https://team.example');
    await setTeamMode('team');

    await send(SUBMISSION);
    await flush();

    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('does not push in local mode, even when enrolled', async () => {
    const send = startWorker();
    const fetchMock = stubFetch(() => jsonResponse({}));
    await enrollInTeam('local');

    await send(SUBMISSION);
    await flush();

    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('pushes in both mode', async () => {
    const send = startWorker();
    const fetchMock = stubFetch(() => jsonResponse({ id: 'est-1', ack: true }));
    await enrollInTeam('both');

    await send(SUBMISSION);

    await eventually(() => expect(fetchMock).toHaveBeenCalledTimes(1));
  });

  it('answers ok and logs a warning when the server cannot be reached', async () => {
    const send = startWorker();
    stubFetch(() => {
      throw new TypeError('Failed to fetch');
    });
    await enrollInTeam('team');

    expect(await send(SUBMISSION)).toEqual({ ok: true });

    await eventually(() =>
      expect(console.warn).toHaveBeenCalledWith(DEV_LOG, expect.any(TypeError))
    );
    expect(await send({ type: 'get_daily_total' })).toMatchObject({ total: { count: 1 } });
  });
});

describe('get_pairing_status', () => {
  it('is not paired and without bridge on a fresh install', async () => {
    const send = startWorker();
    expect(await send({ type: 'get_pairing_status' })).toEqual({
      status: { paired: false, bridgeAvailable: false }
    });
  });

  it('reports the bridge as available when it answers the ping', async () => {
    const bridge = bridgeFactory(() => ({ ok: true }));
    const send = startWorker(bridge);
    expect(await send({ type: 'get_pairing_status' })).toEqual({
      status: { paired: false, bridgeAvailable: true }
    });
    expect(bridge.ports[0]?.port.disconnect).toHaveBeenCalled();
  });

  it('describes the pairing without ever exposing the secret', async () => {
    const bridge = bridgeFactory(() => ({ ok: true }));
    const send = startWorker(bridge);
    await setPairingState(PAIRING);

    const answer = await send({ type: 'get_pairing_status' });

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
    const send = startWorker();
    await setPairingState(PAIRING);
    expect(await send({ type: 'get_pairing_status' })).toMatchObject({
      status: { paired: true, bridgeAvailable: false }
    });
  });
});

describe('pair_with_code', () => {
  it('refuses with a message when the bridge is not installed', async () => {
    const send = startWorker();
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
    const send = startWorker(bridge);

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

  it.each([
    ['an error', { ok: false, error: 'Code expiré' }, 'Code expiré'],
    ['no error text', { ok: false }, 'Code invalide ou expiré.'],
    ['ok but no fingerprint', { ok: true, secret: 's', pairingId: 'p' }, 'Code invalide ou expiré.']
  ])('relays a bridge answer with %s', async (_name, answer, error) => {
    const bridge = bridgeFactory((request) => (request.type === 'pair' ? answer : { ok: true }));
    const send = startWorker(bridge);

    expect(await send({ type: 'pair_with_code', code: '1' })).toEqual({ ok: false, error });
    expect(stub.local.data.has('pairing_state')).toBe(false);
  });
});

describe('revoke_pairing', () => {
  it('asks the bridge to revoke the secret, then forgets the pairing', async () => {
    const bridge = bridgeFactory(() => ({ ok: true }));
    const send = startWorker(bridge);
    await setPairingState(PAIRING);

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
    const send = startWorker(bridge);
    await setPairingState(PAIRING);

    expect(await send({ type: 'revoke_pairing' })).toEqual({ ok: true });
    expect(stub.local.data.has('pairing_state')).toBe(false);
  });

  it('forgets the pairing when the bridge is not installed', async () => {
    const send = startWorker();
    await setPairingState(PAIRING);
    expect(await send({ type: 'revoke_pairing' })).toEqual({ ok: true });
    expect(stub.local.data.has('pairing_state')).toBe(false);
  });

  it('does not contact the bridge when there is no pairing', async () => {
    const bridge = bridgeFactory(() => ({ ok: true }));
    const send = startWorker(bridge);
    expect(await send({ type: 'revoke_pairing' })).toEqual({ ok: true });
    expect(stub.runtime.connectNative).not.toHaveBeenCalled();
  });
});

// Chrome and Firefox open the port, then report a native host that is not
// installed through onDisconnect: this is what a user without the desktop app
// gets. No timer is advanced here, so waiting for the 5 s timeout would hang.
describe('native host reported missing by the browser', () => {
  const missingHost = { connectNative: () => createFakePort(() => DISCONNECT).port };

  it('get_pairing_status answers without bridge, at once', async () => {
    const send = startWorker(missingHost);
    await setPairingState(PAIRING);
    expect(await send({ type: 'get_pairing_status' })).toMatchObject({
      status: { paired: true, bridgeAvailable: false }
    });
  });

  it('pair_with_code refuses with the "bridge not installed" message', async () => {
    const send = startWorker(missingHost);
    expect(await send({ type: 'pair_with_code', code: '123456' })).toMatchObject({
      ok: false,
      error: expect.stringContaining('Bridge natif indisponible')
    });
  });

  it('revoke_pairing still forgets the pairing', async () => {
    const send = startWorker(missingHost);
    await setPairingState(PAIRING);
    expect(await send({ type: 'revoke_pairing' })).toEqual({ ok: true });
    expect(stub.local.data.has('pairing_state')).toBe(false);
  });

  it('estimation_submitted still answers ok and stores the estimate', async () => {
    const send = startWorker(missingHost);
    await setPairingState(PAIRING);
    expect(await send(SUBMISSION)).toEqual({ ok: true });
    expect(await send({ type: 'get_daily_total' })).toMatchObject({ total: { count: 1 } });
  });
});

describe('errors', () => {
  it('answers ok: false with an error text for an unknown message type, and logs it', async () => {
    const send = startWorker();
    expect(await send({ type: 'does_not_exist' })).toEqual({
      ok: false,
      error: expect.stringMatching(/message type inconnu/)
    });
    expect(console.error).toHaveBeenCalledWith(DEV_LOG, expect.any(Error));
  });

  it('answers ok: false when the storage fails', async () => {
    const send = startWorker();
    stub.local.get.mockRejectedValueOnce(new Error('quota exceeded'));
    expect(await send({ type: 'get_method' })).toEqual({
      ok: false,
      error: 'Error: quota exceeded'
    });
  });
});
