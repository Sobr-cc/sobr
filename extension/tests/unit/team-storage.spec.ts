// @vitest-environment node
// SOB-158 — characterization of `content/shared/team-storage.ts` (Team mode
// persistence): keys, defaults, what a logout keeps, and the browser
// fingerprint sent to the server at enrollment.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  browserFingerprint,
  clearTeamSession,
  getTeamAccessToken,
  getTeamRefreshToken,
  getTeamState,
  recordEnrollment,
  requireTeamUrl,
  setTeamMode,
  setTeamTokens,
  setTeamUrl
} from '../../src/content/shared/team-storage.js';
import { installChrome, type ChromeStub } from './helpers/chrome-stub.js';

let local: ChromeStub['local'];

beforeEach(() => {
  ({ local } = installChrome());
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('getTeamState', () => {
  it('returns the local-only defaults on an empty storage', async () => {
    expect(await getTeamState()).toEqual({
      enrolled: false,
      url: null,
      userId: null,
      mode: 'local',
      fingerprint: null,
      enrolledAt: null
    });
  });

  it('is enrolled as soon as a non-empty access token is stored', async () => {
    await setTeamTokens({ access: 'jwt', refresh: 'ulid.uuid' });
    expect((await getTeamState()).enrolled).toBe(true);

    local.data.set('team_access', '');
    expect((await getTeamState()).enrolled).toBe(false);

    local.data.set('team_access', 42);
    expect((await getTeamState()).enrolled).toBe(false);
  });

  it('does not need the refresh token to be enrolled', async () => {
    local.data.set('team_access', 'jwt');
    expect((await getTeamState()).enrolled).toBe(true);
  });

  it('reads the mode, and falls back to local for an unknown value', async () => {
    for (const mode of ['local', 'team', 'both'] as const) {
      await setTeamMode(mode);
      expect((await getTeamState()).mode).toBe(mode);
    }
    local.data.set('team_mode', 'everything');
    expect((await getTeamState()).mode).toBe('local');
  });

  it('ignores stored values that are not strings', async () => {
    local.data.set('team_url', 42);
    local.data.set('team_user_id', {});
    local.data.set('team_fingerprint', null);
    local.data.set('team_enrolled_at', false);
    const state = await getTeamState();
    expect(state.url).toBeNull();
    expect(state.userId).toBeNull();
    expect(state.fingerprint).toBeNull();
    expect(state.enrolledAt).toBeNull();
  });
});

describe('stored format', () => {
  it('writes each Team mode setting under its historical key', async () => {
    await setTeamUrl('https://team.example');
    await setTeamMode('both');
    await setTeamTokens({ access: 'a', refresh: 'r' });
    await recordEnrollment({ userId: 'u', fingerprint: 'f', enrolledAt: 't' });

    expect(Object.fromEntries(local.data)).toEqual({
      team_url: 'https://team.example',
      team_mode: 'both',
      team_access: 'a',
      team_refresh: 'r',
      team_user_id: 'u',
      team_fingerprint: 'f',
      team_enrolled_at: 't'
    });
  });

  it('reads what a previous version of the extension stored', async () => {
    for (const [key, value] of Object.entries({
      team_url: 'https://team.example',
      team_mode: 'team',
      team_access: 'jwt',
      team_refresh: 'ulid.uuid',
      team_user_id: 'u',
      team_fingerprint: 'f',
      team_enrolled_at: 't'
    })) {
      local.data.set(key, value);
    }
    expect(await getTeamState()).toEqual({
      enrolled: true,
      url: 'https://team.example',
      userId: 'u',
      mode: 'team',
      fingerprint: 'f',
      enrolledAt: 't'
    });
    expect(await getTeamRefreshToken()).toBe('ulid.uuid');
  });
});

describe('setTeamUrl', () => {
  it('trims spaces and removes trailing slashes', async () => {
    await setTeamUrl('  https://team.example/// ');
    expect(local.data.get('team_url')).toBe('https://team.example');
  });

  it('keeps a path and does not touch inner slashes', async () => {
    await setTeamUrl('https://team.example:8443/sobria/');
    expect(local.data.get('team_url')).toBe('https://team.example:8443/sobria');
  });

  it('stores an empty string for a blank URL (not rejected here)', async () => {
    await setTeamUrl('   ');
    expect(local.data.get('team_url')).toBe('');
  });
});

describe('tokens', () => {
  it('returns null while nothing is stored, and the stored values afterwards', async () => {
    expect(await getTeamAccessToken()).toBeNull();
    expect(await getTeamRefreshToken()).toBeNull();

    await setTeamTokens({ access: 'jwt-1', refresh: 'ulid.uuid-1' });
    expect(await getTeamAccessToken()).toBe('jwt-1');
    expect(await getTeamRefreshToken()).toBe('ulid.uuid-1');
  });

  it('returns null for a stored value that is not a string', async () => {
    local.data.set('team_access', 1);
    local.data.set('team_refresh', ['x']);
    expect(await getTeamAccessToken()).toBeNull();
    expect(await getTeamRefreshToken()).toBeNull();
  });
});

describe('clearTeamSession', () => {
  it('removes the tokens and the enrollment but keeps the URL and the mode', async () => {
    await setTeamUrl('https://team.example');
    await setTeamMode('team');
    await setTeamTokens({ access: 'a', refresh: 'r' });
    await recordEnrollment({ userId: 'u', fingerprint: 'f', enrolledAt: 't' });

    await clearTeamSession();

    expect(await getTeamState()).toEqual({
      enrolled: false,
      url: 'https://team.example',
      userId: null,
      mode: 'team',
      fingerprint: null,
      enrolledAt: null
    });
    expect(await getTeamRefreshToken()).toBeNull();
  });
});

describe('requireTeamUrl', () => {
  it('throws when no URL is configured, or when it is empty', async () => {
    await expect(requireTeamUrl()).rejects.toThrow('URL serveur équipe non configurée');
    local.data.set('team_url', '');
    await expect(requireTeamUrl()).rejects.toThrow('URL serveur équipe non configurée');
  });

  it('returns the stored URL', async () => {
    await setTeamUrl('https://team.example/');
    expect(await requireTeamUrl()).toBe('https://team.example');
  });
});

describe('browserFingerprint', () => {
  function stubBrowser(userAgent: string, platform: string, timeZone = 'Europe/Paris'): void {
    vi.stubGlobal('navigator', { userAgent, platform });
    vi.spyOn(Intl, 'DateTimeFormat').mockImplementation(
      () => ({ resolvedOptions: () => ({ timeZone }) }) as unknown as Intl.DateTimeFormat
    );
  }

  // The hash imitates FNV-1a but is not exact: `h ^= code` yields a signed 32-bit
  // number and `h * 0x01000193` exceeds 2^53, so the product is rounded. The
  // values below are what the code returns (checked against an independent
  // emulation of that arithmetic). Changing the hash would change the
  // fingerprint of every enrolled browser.
  it('pins the exact fingerprint, which is not a standard FNV-1a (precision loss)', () => {
    stubBrowser('Mozilla/5.0 (Windows NT 10.0) Chrome/130', 'Win32');
    expect(browserFingerprint()).toBe('ext-win-21d796c0');
    stubBrowser('UA', 'Linux x86_64', 'UTC');
    expect(browserFingerprint()).toBe('ext-linuxx-6af2ea9c');
  });

  it('changes with the user agent, the platform and the time zone', () => {
    stubBrowser('UA', 'Linux x86_64', 'UTC');
    const base = browserFingerprint();

    stubBrowser('UA-2', 'Linux x86_64', 'UTC');
    expect(browserFingerprint()).not.toBe(base);
    stubBrowser('UA', 'Linux armv8l', 'UTC');
    expect(browserFingerprint()).not.toBe(base);
    stubBrowser('UA', 'Linux x86_64', 'Europe/Paris');
    expect(browserFingerprint()).not.toBe(base);
  });

  it.each([
    ['Linux x86_64', 'linuxx'],
    ['MacIntel', 'macintel'],
    ['Win32', 'win'],
    ['VeryLongPlatformName', 'verylong'],
    // no platform at all, then a platform made of digits only
    ['', 'unknown'],
    ['1234', 'ext']
  ])(
    'platform "%s" gives the slug "%s" (letters only, lower case, 8 at most)',
    (platform, slug) => {
      stubBrowser('UA', platform);
      expect(browserFingerprint()).toMatch(new RegExp(`^ext-${slug}-[0-9a-f]{8}$`));
    }
  );
});
