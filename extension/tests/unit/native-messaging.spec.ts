// @vitest-environment node
// SOB-158 — characterization of `background/native-messaging.ts`: how the
// extension talks to the native bridge (requests, matching of answers,
// timeouts, disconnections).

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { BridgeClient } from '../../src/background/native-messaging.js';
import {
  createFakePort,
  DISCONNECT,
  fakeEstimate,
  installChrome,
  silenceConsole,
  withPing,
  type BridgeReply
} from './helpers/chrome-stub.js';

const DEV_LOG = expect.stringContaining('[sobria bridge]');

/** A bridge that answers pings, and everything else with `more`. */
function connectedChrome(more?: BridgeReply) {
  const bridge = createFakePort(withPing(more));
  const stub = installChrome({ connectNative: () => bridge.port });
  return { ...stub, bridge };
}

/** A client already connected to a bridge that answers pings and everything else with `more`. */
async function connectedClient(more?: BridgeReply) {
  const stub = connectedChrome(more);
  const client = new BridgeClient();
  await client.connect();
  return { ...stub, client };
}

beforeEach(() => {
  silenceConsole('warn');
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('connect', () => {
  it('opens the port of the native app com.sobria.bridge and pings it', async () => {
    const { runtime, bridge } = connectedChrome();

    expect(await new BridgeClient().connect()).toBe(true);

    expect(runtime.connectNative).toHaveBeenCalledWith('com.sobria.bridge');
    expect(bridge.sent()).toEqual([{ type: 'ping', reqId: expect.any(String) }]);
  });

  it('is false when the browser has no native messaging', async () => {
    installChrome();
    expect(await new BridgeClient().connect()).toBe(false);
  });

  it('is false, and logs a warning, when connectNative throws (invalid host name)', async () => {
    installChrome({
      connectNative: () => {
        throw new Error('Invalid native messaging host name specified.');
      }
    });
    const client = new BridgeClient();
    expect(await client.connect()).toBe(false);
    expect(console.warn).toHaveBeenCalledWith(DEV_LOG, expect.any(Error));
    await expect(client.pair('123456')).rejects.toThrow('Bridge non connecté');
  });

  // Chrome and Firefox open the port, then report a host that is not installed
  // through onDisconnect: this is what a user without the desktop app gets.
  it('is false at once when the browser reports the native host as missing', async () => {
    vi.useFakeTimers(); // no timer is advanced: waiting for the 5 s timeout would hang
    const { runtime } = installAndConnect(createFakePort(() => DISCONNECT));
    runtime.lastError = { message: 'Specified native messaging host not found.' };

    expect(await new BridgeClient().connect()).toBe(false);

    expect(console.warn).toHaveBeenCalledWith(
      DEV_LOG,
      'Specified native messaging host not found.'
    );
  });

  it('is false when the bridge answers the ping without pong: true', async () => {
    installAndConnect(createFakePort(() => ({ ok: true, pong: false })));
    expect(await new BridgeClient().connect()).toBe(false);
  });

  it('is false when the bridge stays silent for 5 seconds', async () => {
    vi.useFakeTimers();
    installAndConnect(createFakePort());
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

/** Installs a chrome stub whose native app is `bridge`, as is. */
function installAndConnect(bridge: ReturnType<typeof createFakePort>) {
  return installChrome({ connectNative: () => bridge.port });
}

describe('requests', () => {
  it('sends pair, estimate and revoke with the documented fields', async () => {
    const { client, bridge } = await connectedClient(() => ({ ok: true }));
    const payload = {
      estimate: fakeEstimate(),
      host: 'claude' as const,
      modelDisplayName: 'Claude Sonnet 4.6',
      ts: '2026-05-16T08:00:00.000Z'
    };

    await client.pair('123456');
    await client.sendEstimate('s3cret', payload);
    await client.revoke('s3cret');

    const [, pair, estimate, revoke] = bridge.sent();
    expect(pair).toEqual({ type: 'pair', reqId: expect.any(String), code: '123456' });
    expect(estimate).toEqual({
      type: 'estimate',
      reqId: expect.any(String),
      secret: 's3cret',
      payload
    });
    expect(revoke).toEqual({ type: 'revoke', reqId: expect.any(String), secret: 's3cret' });
  });

  it('gives each request an id of its own', async () => {
    const { client, bridge } = await connectedClient(() => ({ ok: true }));
    await client.pair('1');
    await client.revoke('s');
    const ids = bridge.sent().map((request) => request.reqId);
    expect(ids).toHaveLength(3);
    expect(new Set(ids).size).toBe(3);
    expect(ids.every((id) => id.length > 0)).toBe(true);
  });

  it('resolves with the answer of the bridge, unchanged (errors included)', async () => {
    const { client, bridge } = await connectedClient((request) =>
      request.type === 'pair'
        ? { ok: false, error: 'Code invalide ou expiré.' }
        : { ok: true, secret: 's', pairingId: 'p', fingerprint: 'f' }
    );

    expect(await client.pair('000000')).toMatchObject({
      ok: false,
      error: 'Code invalide ou expiré.'
    });
    expect(await client.revoke('s')).toMatchObject({ ok: true, secret: 's', pairingId: 'p' });
    expect(bridge.port.postMessage).toHaveBeenCalledTimes(3);
  });

  it('matches answers to requests by reqId, whatever the order', async () => {
    const { client, bridge } = await connectedClient();

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

  it('rejects with the error of postMessage', async () => {
    const { client, bridge } = await connectedClient();
    bridge.port.postMessage.mockImplementationOnce(() => {
      throw new Error('port closed');
    });
    await expect(client.pair('1')).rejects.toThrow('port closed');
  });
});

describe('answers that are ignored, and the timeout', () => {
  it('ignores non-objects, answers without a string reqId and unknown reqIds, then times out after 5000 ms', async () => {
    vi.useFakeTimers();
    const { client, bridge } = await connectedClient();

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
    await vi.advanceTimersByTimeAsync(4999);
    let settled = false;
    void pending.then(() => {
      settled = true;
    });
    await vi.advanceTimersByTimeAsync(0);
    expect(settled).toBe(false);

    await vi.advanceTimersByTimeAsync(1);
    expect(await pending).toEqual(new Error('Bridge timeout (5000 ms)'));
  });
});

describe('disconnect', () => {
  it('closes the port and refuses new requests', async () => {
    const { client, bridge } = await connectedClient();

    client.disconnect();

    expect(bridge.port.disconnect).toHaveBeenCalledTimes(1);
    await expect(client.pair('1')).rejects.toThrow('Bridge non connecté');
  });

  it('does not fail when the port is already closed or when called twice', async () => {
    const { client, bridge } = await connectedClient();
    bridge.port.disconnect.mockImplementation(() => {
      throw new Error('already closed');
    });
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
    const { client, bridge } = await connectedClient();

    const pending = client.pair('1');
    bridge.emitDisconnect();

    expect(await pending).toEqual({ reqId: '', ok: false, error: 'Bridge déconnecté' });
    await expect(client.pair('2')).rejects.toThrow('Bridge non connecté');
  });

  it('logs the browser error when there is one', async () => {
    const { bridge, runtime } = await connectedClient();
    runtime.lastError = { message: 'Native host has exited.' };

    bridge.emitDisconnect();

    expect(console.warn).toHaveBeenCalledWith(DEV_LOG, 'Native host has exited.');
  });
});
