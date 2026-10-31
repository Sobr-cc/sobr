// @vitest-environment node
// SOB-158 — characterization of `lib/team-client.ts` (REST client of the team
// server): requests sent, token refresh on 401, error mapping and what is
// stored after enrollment.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  enroll,
  logout,
  ping,
  pushEstimation,
  TeamApiError,
  type EstimatePayload
} from '../../src/lib/team-client.js';
import {
  getTeamAccessToken,
  getTeamRefreshToken,
  getTeamState,
  setTeamMode,
  setTeamTokens,
  setTeamUrl
} from '../../src/content/shared/team-storage.js';
import {
  fakeEstimate,
  installChrome,
  jsonResponse,
  request,
  stubFetch
} from './helpers/chrome-stub.js';

type Reply = { status?: number; body?: unknown; raw?: string } | Error;

/** Replaces `fetch` with a queue of answers, consumed in order. */
function queueFetch(...replies: Reply[]) {
  const queue = [...replies];
  return stubFetch(() => {
    const reply = queue.shift();
    if (!reply) throw new Error('unexpected fetch call');
    if (reply instanceof Error) throw reply;
    return reply.raw === undefined
      ? jsonResponse(reply.body ?? {}, reply.status)
      : new Response(reply.raw, { status: reply.status ?? 200 });
  });
}

const PAYLOAD: EstimatePayload = {
  estimate: fakeEstimate(),
  host: 'chatgpt',
  modelDisplayName: 'GPT-4o',
  ts: '2026-05-16T08:00:00.000Z'
};

beforeEach(async () => {
  installChrome();
  await setTeamUrl('https://team.example');
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe('server URL', () => {
  it('fails before any request when no server URL is configured', async () => {
    installChrome();
    const fetchMock = queueFetch();
    await expect(ping()).rejects.toThrow('URL serveur équipe non configurée');
    await expect(pushEstimation(PAYLOAD)).rejects.toThrow('URL serveur équipe non configurée');
    expect(fetchMock).not.toHaveBeenCalled();
  });
});

describe('ping', () => {
  it('sends GET /health to the configured server and returns the body', async () => {
    const fetchMock = queueFetch({ body: { ok: true, version: '0.9.0' } });
    expect(await ping()).toEqual({ ok: true, version: '0.9.0' });
    expect(request(fetchMock, 0)).toMatchObject({
      url: 'https://team.example/health',
      method: 'GET'
    });
  });

  it('does not send the access token', async () => {
    await setTeamTokens({ access: 'jwt', refresh: 'r' });
    const fetchMock = queueFetch({ body: { ok: true, version: '0.9.0' } });
    await ping();
    expect(request(fetchMock, 0).headers.authorization).toBeUndefined();
  });

  it('maps a failed answer without JSON body to http_error', async () => {
    queueFetch({ status: 502, raw: '<html>bad gateway</html>' });
    await expect(ping()).rejects.toMatchObject({
      status: 502,
      code: 'http_error',
      message: 'HTTP 502'
    });
  });

  it('lets a network error through untouched (not a TeamApiError)', async () => {
    const failure = new TypeError('Failed to fetch');
    queueFetch(failure);
    const error = await ping().catch((e: unknown) => e);
    expect(error).toBe(failure);
    expect(error).not.toBeInstanceOf(TeamApiError);
  });
});

describe('error mapping', () => {
  it('uses the error and code fields of the JSON body', async () => {
    queueFetch({ status: 400, body: { error: 'payload invalide', code: 'bad_request' } });
    const error = await pushEstimation(PAYLOAD).catch((e: unknown) => e);
    expect(error).toBeInstanceOf(TeamApiError);
    expect(error).toBeInstanceOf(Error);
    expect(error).toMatchObject({ status: 400, code: 'bad_request', message: 'payload invalide' });
  });

  it('keeps the defaults for the fields that are missing or not strings', async () => {
    queueFetch({ status: 500, body: { error: 12, code: null } });
    await expect(pushEstimation(PAYLOAD)).rejects.toMatchObject({
      status: 500,
      code: 'http_error',
      message: 'HTTP 500'
    });
  });
});

describe('enroll', () => {
  it('posts the credentials and stores the tokens and the enrollment', async () => {
    vi.useFakeTimers({ toFake: ['Date'] });
    vi.setSystemTime(new Date('2026-05-16T08:30:00.000Z'));
    const answer = {
      user_id: '01HZ0000000000000000000001',
      access_token: 'jwt-1',
      refresh_token: 'ulid.uuid-1',
      access_expires_at: '2026-05-17T08:30:00Z',
      refresh_expires_at: '2026-08-14T08:30:00Z'
    };
    const fetchMock = queueFetch({ body: answer });

    const result = await enroll({
      code: '123456789012',
      password: 'a-password',
      fingerprint: 'ext-win-21d796c0',
      displayName: 'Ada'
    });

    expect(result).toEqual(answer);
    expect(request(fetchMock, 0)).toMatchObject({
      url: 'https://team.example/api/v1/enroll',
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: {
        code: '123456789012',
        password: 'a-password',
        fingerprint: 'ext-win-21d796c0',
        display_name: 'Ada'
      }
    });
    expect(request(fetchMock, 0).headers.authorization).toBeUndefined();
    expect(await getTeamState()).toEqual({
      enrolled: true,
      url: 'https://team.example',
      userId: '01HZ0000000000000000000001',
      mode: 'local',
      fingerprint: 'ext-win-21d796c0',
      enrolledAt: '2026-05-16T08:30:00.000Z'
    });
    expect(await getTeamAccessToken()).toBe('jwt-1');
    expect(await getTeamRefreshToken()).toBe('ulid.uuid-1');
  });

  it('omits display_name when none is given', async () => {
    const fetchMock = queueFetch({
      body: { user_id: 'u', access_token: 'a', refresh_token: 'r' }
    });
    await enroll({ code: '1', password: 'p', fingerprint: 'f' });
    expect(request(fetchMock, 0).body).toEqual({ code: '1', password: 'p', fingerprint: 'f' });
  });

  it('stores nothing when the server refuses', async () => {
    queueFetch({
      status: 409,
      body: { error: 'empreinte déjà enrôlée', code: 'fingerprint_already_enrolled' }
    });
    await expect(enroll({ code: '1', password: 'p', fingerprint: 'f' })).rejects.toMatchObject({
      status: 409,
      code: 'fingerprint_already_enrolled'
    });
    expect((await getTeamState()).enrolled).toBe(false);
    expect(await getTeamRefreshToken()).toBeNull();
  });
});

describe('pushEstimation', () => {
  it('posts the payload as JSON with the bearer token', async () => {
    await setTeamTokens({ access: 'jwt-1', refresh: 'ulid.uuid-1' });
    const fetchMock = queueFetch({ body: { id: '01HZ-est', ack: true } });

    expect(await pushEstimation(PAYLOAD)).toEqual({ id: '01HZ-est', ack: true });
    expect(request(fetchMock, 0)).toMatchObject({
      url: 'https://team.example/api/v1/estimations',
      method: 'POST',
      headers: { 'content-type': 'application/json', authorization: 'Bearer jwt-1' },
      body: PAYLOAD
    });
  });

  it('sends no Authorization header without a token', async () => {
    const fetchMock = queueFetch({ status: 401, body: { error: 'jeton manquant' } });
    await expect(pushEstimation(PAYLOAD)).rejects.toMatchObject({ status: 401 });
    expect(request(fetchMock, 0).headers.authorization).toBeUndefined();
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('includes the project label when the payload has one', async () => {
    const fetchMock = queueFetch({ body: { id: 'x', ack: true } });
    await pushEstimation({ ...PAYLOAD, project: 'Alpha' });
    expect((request(fetchMock, 0).body as EstimatePayload).project).toBe('Alpha');
  });
});

describe('refresh on 401', () => {
  beforeEach(async () => {
    await setTeamMode('team');
    await setTeamTokens({ access: 'old-jwt', refresh: 'old-ulid.old-uuid' });
  });

  it('refreshes the tokens, stores the new pair and retries once', async () => {
    const fetchMock = queueFetch(
      { status: 401, body: { error: 'jeton expiré', code: 'invalid_token' } },
      { body: { access_token: 'new-jwt', refresh_token: 'new-ulid.new-uuid' } },
      { body: { id: 'est-1', ack: true } }
    );

    expect(await pushEstimation(PAYLOAD)).toEqual({ id: 'est-1', ack: true });

    expect(fetchMock).toHaveBeenCalledTimes(3);
    expect(request(fetchMock, 0).headers.authorization).toBe('Bearer old-jwt');
    expect(request(fetchMock, 1)).toMatchObject({
      url: 'https://team.example/api/v1/refresh',
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: { refresh_token: 'old-ulid.old-uuid' }
    });
    expect(request(fetchMock, 1).headers.authorization).toBeUndefined();
    expect(request(fetchMock, 2).headers.authorization).toBe('Bearer new-jwt');
    expect(await getTeamAccessToken()).toBe('new-jwt');
    expect(await getTeamRefreshToken()).toBe('new-ulid.new-uuid');
  });

  it('refreshes only once: a second 401 is returned as an error', async () => {
    const fetchMock = queueFetch(
      { status: 401, body: { error: 'jeton expiré', code: 'invalid_token' } },
      { body: { access_token: 'new-jwt', refresh_token: 'new-ulid.new-uuid' } },
      { status: 401, body: { error: 'toujours invalide', code: 'invalid_token' } }
    );
    await expect(pushEstimation(PAYLOAD)).rejects.toMatchObject({
      status: 401,
      message: 'toujours invalide'
    });
    expect(fetchMock).toHaveBeenCalledTimes(3);
  });

  it('logs the user out when the refresh is refused, and returns the original 401', async () => {
    const fetchMock = queueFetch(
      { status: 401, body: { error: 'jeton expiré', code: 'invalid_token' } },
      { status: 401, body: { error: 'refresh invalide' } }
    );
    await expect(pushEstimation(PAYLOAD)).rejects.toMatchObject({
      status: 401,
      message: 'jeton expiré'
    });
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(await getTeamState()).toMatchObject({
      enrolled: false,
      url: 'https://team.example',
      mode: 'team'
    });
    expect(await getTeamRefreshToken()).toBeNull();
  });

  it('logs the user out when the refresh request itself fails', async () => {
    queueFetch({ status: 401, body: { error: 'jeton expiré' } }, new TypeError('offline'));
    await expect(pushEstimation(PAYLOAD)).rejects.toMatchObject({ status: 401 });
    expect(await getTeamAccessToken()).toBeNull();
    expect(await getTeamRefreshToken()).toBeNull();
  });

  it('does not try to refresh without a refresh token', async () => {
    await setTeamTokens({ access: 'old-jwt', refresh: '' });
    const fetchMock = queueFetch({ status: 401, body: { error: 'jeton expiré' } });
    await expect(pushEstimation(PAYLOAD)).rejects.toMatchObject({ status: 401 });
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('does not try to refresh a 401 sent without a token', async () => {
    await setTeamTokens({ access: '', refresh: 'r' });
    const fetchMock = queueFetch({ status: 401, body: { error: 'jeton manquant' } });
    await expect(pushEstimation(PAYLOAD)).rejects.toMatchObject({ status: 401 });
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('does not refresh on other failures', async () => {
    const fetchMock = queueFetch({ status: 403, body: { error: 'interdit', code: 'forbidden' } });
    await expect(pushEstimation(PAYLOAD)).rejects.toMatchObject({ status: 403, code: 'forbidden' });
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
});

describe('logout', () => {
  it('removes the session and keeps the URL and the mode', async () => {
    await setTeamMode('both');
    await setTeamTokens({ access: 'a', refresh: 'r' });
    await logout();
    expect(await getTeamState()).toMatchObject({
      enrolled: false,
      url: 'https://team.example',
      mode: 'both'
    });
  });
});
