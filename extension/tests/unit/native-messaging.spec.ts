// SOB-158 — characterization of `background/native-messaging.ts`: how the
// extension talks to the native bridge (requests, matching of answers,
// timeouts, disconnections).

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { BridgeClient } from '../../src/background/native-messaging.js';
import type { Estimate } from '../../src/lib/types.js';
import { createFakePort, installChrome, type BridgeRequest } from './helpers/chrome-stub.js';

const ESTIMATE = { method: 'afnor_sobria', modelId: 'gpt-4o', gco2eq: 1.5 } as unknown as Estimate;
const REQ_ID = /^r[0-9a-z]+-\d+$/;

/** A bridge that answers pings (and nothing else unless `more` handles it). */
function pongBridge(
  more: (request: BridgeRequest) => Record<string, unknown> | undefined = () => undefined
) {
  return createFakePort((request) =>
    request.type === 'ping' ? { ok: true, pong: true } : more(request)
  );
}

function connectedChrome(bridge = pongBridge()) {
  const stub = installChrome({ connectNative: () => bridge.port });
  return { ...stub, bridge };
}

beforeEach(() => {
  vi.spyOn(console, 'warn').mockImplementation(() => undefined);
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('connect', () => {
  it('opens the port of the native app com.sobria.bridge and pings it', async () => {
    const { runtime, bridge } = connectedChrome();
    const client = new BridgeClient();

    expect(await client.connect()).toBe(true);

    expect(runtime.connectNative).toHaveBeenCalledWith('com.sobria.bridge');
    const [ping] = bridge.sent();
    expect(ping).toMatchObject({ type: 'ping' });
    expect(ping?.reqId).toMatch(REQ_ID);
  });

  it('is false when the browser has no native messaging', async () => {
    installChrome();
    expect(await new BridgeClient().connect()).toBe(false);
  });

  it('is false, and logs a warning, when connectNative throws (bridge not installed)', async () => {
    installChrome({
      connectNative: () => {
        throw new Error('Specified native messaging host not found.');
      }
    });
    const client = new BridgeClient();
    expect(await client.connect()).toBe(false);
    expect(console.warn).toHaveBeenCalledWith(
      '[sobria bridge] connectNative throws:',
      expect.any(Error)
    );
    await expect(client.pair('123456')).rejects.toThrow('Bridge non connecté');
  });

  it('is false when the bridge answers the ping without pong: true', async () => {
    connectedChrome(createFakePort(() => ({ ok: true, pong: false })));
    expect(await new BridgeClient().connect()).toBe(false);
  });

  it('is false when the bridge stays silent for 5 seconds', async () => {
    vi.useFakeTimers();
    connectedChrome(createFakePort());
    const result = new BridgeClient().connect();
    await vi.advanceTimersByTimeAsync(4999);
    let settled = false;
    void result.then(() => {
      settled = true;
    });
    await vi.advanceTimersByTimeAsync(0);
    expect(settled).toBe(false);

    await vi.advanceTimersByTimeAsync(1);
    expect(await result).toBe(false);
  });
});

describe('requests', () => {
  it('sends pair, estimate and revoke with the documented fields', async () => {
    const { bridge } = connectedChrome(pongBridge(() => ({ ok: true })));
    const client = new BridgeClient();
    await client.connect();
    const payload = {
      estimate: ESTIMATE,
      host: 'claude' as const,
      modelDisplayName: 'Claude Sonnet 4.6',
      ts: '2026-05-16T08:00:00.000Z'
    };

    await client.pair('123456');
    await client.sendEstimate('s3cret', payload);
    await client.revoke('s3cret');

    const [, pair, estimate, revoke] = bridge.sent();
    expect(pair).toEqual({ type: 'pair', reqId: expect.stringMatching(REQ_ID), code: '123456' });
    expect(estimate).toEqual({
      type: 'estimate',
      reqId: expect.stringMatching(REQ_ID),
      secret: 's3cret',
      payload
    });
    expect(revoke).toEqual({
      type: 'revoke',
      reqId: expect.stringMatching(REQ_ID),
      secret: 's3cret'
    });
  });

  it('numbers the requests of one client 1, 2, 3, …', async () => {
    const { bridge } = connectedChrome(pongBridge(() => ({ ok: true })));
    const client = new BridgeClient();
    await client.connect();
    await client.pair('1');
    await client.revoke('s');
    expect(bridge.sent().map((r) => r.reqId.split('-')[1])).toEqual(['1', '2', '3']);
  });

  it('resolves with the answer of the bridge, unchanged (errors included)', async () => {
    const { bridge } = connectedChrome(
      pongBridge((request) =>
        request.type === 'pair'
          ? { ok: false, error: 'Code invalide ou expiré.' }
          : { ok: true, secret: 's', pairingId: 'p', fingerprint: 'f' }
      )
    );
    const client = new BridgeClient();
    await client.connect();

    expect(await client.pair('000000')).toMatchObject({
      ok: false,
      error: 'Code invalide ou expiré.'
    });
    expect(await client.revoke('s')).toMatchObject({ ok: true, secret: 's', pairingId: 'p' });
    expect(bridge.port.postMessage).toHaveBeenCalledTimes(3);
  });

  it('matches answers to requests by reqId, whatever the order', async () => {
    const { bridge } = connectedChrome();
    const client = new BridgeClient();
    await client.connect();

    const first = client.pair('111111');
    const second = client.pair('222222');
    const [, one, two] = bridge.sent();
    bridge.emit({ reqId: two?.reqId, ok: true, secret: 'for-second' });
    bridge.emit({ reqId: one?.reqId, ok: true, secret: 'for-first' });

    expect((await first).secret).toBe('for-first');
    expect((await second).secret).toBe('for-second');
  });

  it('rejects when called before connect()', async () => {
    installChrome();
    await expect(new BridgeClient().revoke('s')).rejects.toThrow('Bridge non connecté');
  });

  it('rejects with the error of postMessage and forgets the request', async () => {
    const { bridge } = connectedChrome();
    const client = new BridgeClient();
    await client.connect();
    bridge.port.postMessage.mockImplementationOnce(() => {
      throw new Error('port closed');
    });
    await expect(client.pair('1')).rejects.toThrow('port closed');
  });
});

describe('answers that are ignored', () => {
  it('ignores non-objects, answers without a string reqId and unknown reqIds', async () => {
    vi.useFakeTimers();
    const { bridge } = connectedChrome();
    const client = new BridgeClient();
    const connected = client.connect();
    await vi.advanceTimersByTimeAsync(0);
    expect(await connected).toBe(true);

    const pending = client.pair('1').catch((e: unknown) => e);
    for (const noise of [
      'pong',
      42,
      null,
      { ok: true },
      { reqId: 7, ok: true },
      { reqId: 'other' }
    ]) {
      bridge.emit(noise);
    }
    await vi.advanceTimersByTimeAsync(5000);

    expect(await pending).toEqual(new Error('Bridge timeout (5000 ms)'));
  });
});

describe('timeout', () => {
  it('rejects a request after 5000 ms with "Bridge timeout (5000 ms)"', async () => {
    vi.useFakeTimers();
    connectedChrome();
    const client = new BridgeClient();
    const connected = client.connect();
    await vi.advanceTimersByTimeAsync(0);
    await connected;

    const request = client.revoke('s').catch((e: unknown) => e);
    await vi.advanceTimersByTimeAsync(5000);
    expect(await request).toEqual(new Error('Bridge timeout (5000 ms)'));
  });
});

describe('disconnect', () => {
  it('closes the port and refuses new requests', async () => {
    const { bridge } = connectedChrome();
    const client = new BridgeClient();
    await client.connect();

    client.disconnect();

    expect(bridge.port.disconnect).toHaveBeenCalledTimes(1);
    await expect(client.pair('1')).rejects.toThrow('Bridge non connecté');
  });

  it('does not fail when the port is already closed or when called twice', async () => {
    const { bridge } = connectedChrome();
    bridge.port.disconnect.mockImplementation(() => {
      throw new Error('already closed');
    });
    const client = new BridgeClient();
    await client.connect();
    expect(() => client.disconnect()).not.toThrow();
    expect(() => client.disconnect()).not.toThrow();
    expect(bridge.port.disconnect).toHaveBeenCalledTimes(1);
  });

  it('can be called on a client that never connected', () => {
    expect(() => new BridgeClient().disconnect()).not.toThrow();
  });
});

describe('disconnection by the browser or the bridge', () => {
  it('resolves the pending requests with ok: false and "Bridge déconnecté" (no reqId)', async () => {
    const { bridge } = connectedChrome();
    const client = new BridgeClient();
    await client.connect();

    const pending = client.pair('1');
    bridge.emitDisconnect();

    expect(await pending).toEqual({ reqId: '', ok: false, error: 'Bridge déconnecté' });
    await expect(client.pair('2')).rejects.toThrow('Bridge non connecté');
  });

  it('logs the browser error when there is one', async () => {
    const { bridge, runtime } = connectedChrome();
    runtime.lastError = { message: 'Native host has exited.' };
    const client = new BridgeClient();
    await client.connect();

    bridge.emitDisconnect();

    expect(console.warn).toHaveBeenCalledWith(
      '[sobria bridge] disconnect:',
      'Native host has exited.'
    );
  });
});
