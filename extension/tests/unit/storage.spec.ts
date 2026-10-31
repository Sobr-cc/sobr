// SOB-158 — characterization of `content/shared/storage.ts`: the keys written
// to chrome.storage.local, the defaults, the fallbacks for malformed values and
// the accumulation rules. A renamed key loses the data users already have, so
// the key names are pinned on purpose.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  appendToDailyTotal,
  appendToSessionLog,
  clearPairingState,
  getBadgeVisible,
  getLastEstimate,
  getMethod,
  getPairingState,
  getSessionLog,
  getSitesEnabled,
  getTodayTotal,
  purgeAll,
  setBadgeVisible,
  setLastEstimate,
  setMethod,
  setPairingState,
  setSitesEnabled,
  todayKey,
  type SessionLogEntry
} from '../../src/content/shared/storage.js';
import type { LastEstimate } from '../../src/lib/messages.js';
import type { Estimate } from '../../src/lib/types.js';
import { installChrome } from './helpers/chrome-stub.js';

let local: ReturnType<typeof installChrome>['local'];

beforeEach(() => {
  ({ local } = installChrome());
  vi.useFakeTimers({ toFake: ['Date'] });
  vi.setSystemTime(new Date(2026, 4, 16, 10, 0, 0));
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

const ESTIMATE = { method: 'afnor_sobria', modelId: 'gpt-4o', gco2eq: 1 } as unknown as Estimate;

describe('defaults on an empty storage', () => {
  it('returns the documented default of every getter', async () => {
    expect(await getMethod()).toBe('afnor_sobria');
    expect(await getLastEstimate()).toBeNull();
    expect(await getTodayTotal()).toEqual({
      date: '2026-05-16',
      count: 0,
      gco2eq: 0,
      waterMl: 0,
      energyWh: 0
    });
    expect(await getSitesEnabled()).toEqual({ chatgpt: true, claude: true, leChat: true });
    expect(await getBadgeVisible()).toBe(true);
    expect(await getSessionLog()).toEqual([]);
    expect(await getPairingState()).toBeNull();
  });
});

describe('malformed stored values', () => {
  it('falls back to the default instead of throwing', async () => {
    local.data.set('method', 'something-else');
    local.data.set('last_estimate', 'not an object');
    local.data.set('badge_visible', 'yes');
    local.data.set('sites_enabled', 'not an object');
    local.data.set('session_log', { not: 'an array' });
    local.data.set('pairing_state', 'not an object');
    local.data.set('daily_2026-05-16', 42);

    expect(await getMethod()).toBe('afnor_sobria');
    expect(await getLastEstimate()).toBeNull();
    expect(await getBadgeVisible()).toBe(true);
    expect(await getSitesEnabled()).toEqual({ chatgpt: true, claude: true, leChat: true });
    expect(await getSessionLog()).toEqual([]);
    expect(await getPairingState()).toBeNull();
    expect((await getTodayTotal()).count).toBe(0);
  });

  it('merges a partial sites_enabled object over the defaults', async () => {
    local.data.set('sites_enabled', { claude: false });
    expect(await getSitesEnabled()).toEqual({ chatgpt: true, claude: false, leChat: true });
  });

  it('does not validate the shape of a stored object (returned as is)', async () => {
    local.data.set('pairing_state', { whatever: 1 });
    expect(await getPairingState()).toEqual({ whatever: 1 });
  });
});

describe('storage keys', () => {
  it('writes each setting under its historical key', async () => {
    await setMethod('ecologits');
    await setBadgeVisible(false);
    await setSitesEnabled({ chatgpt: false, claude: true, leChat: true });
    await setPairingState({ secret: 's', pairingId: 'p', fingerprint: 'f', pairedAt: 't' });
    await appendToSessionLog({ ts: 't', gco2eq: 1, waterMl: 2, energyWh: 3 });
    await appendToDailyTotal({ gco2eq: 1, waterMl: 1, energyWh: 1 });
    await setLastEstimate({
      estimate: ESTIMATE,
      host: 'claude',
      modelDisplayName: 'Claude',
      ts: '2026-05-16T08:00:00.000Z'
    });

    expect([...local.data.keys()].sort()).toEqual([
      'badge_visible',
      'daily_2026-05-16',
      'last_estimate',
      'method',
      'pairing_state',
      'session_log',
      'sites_enabled'
    ]);
  });
});

describe('method, badge, sites, last estimate', () => {
  it('round-trips the values', async () => {
    await setMethod('ecologits');
    expect(await getMethod()).toBe('ecologits');

    await setBadgeVisible(false);
    expect(await getBadgeVisible()).toBe(false);

    await setSitesEnabled({ chatgpt: false, claude: false, leChat: true });
    expect(await getSitesEnabled()).toEqual({ chatgpt: false, claude: false, leChat: true });

    const last: LastEstimate = {
      estimate: ESTIMATE,
      host: 'le-chat',
      modelDisplayName: 'Mistral Large 3',
      ts: '2026-05-16T08:00:00.000Z'
    };
    await setLastEstimate(last);
    expect(await getLastEstimate()).toEqual(last);
  });
});

describe('todayKey', () => {
  it('formats the local date as YYYY-MM-DD with zero padding', () => {
    expect(todayKey(new Date(2026, 0, 5))).toBe('2026-01-05');
    expect(todayKey(new Date(2026, 11, 31))).toBe('2026-12-31');
  });

  it('uses the current local date by default', () => {
    expect(todayKey()).toBe('2026-05-16');
  });
});

describe('daily total', () => {
  it('adds each measurement to the total of the local day', async () => {
    const first = await appendToDailyTotal({ gco2eq: 0.5, waterMl: 2, energyWh: 0.25 });
    expect(first).toEqual({
      date: '2026-05-16',
      count: 1,
      gco2eq: 0.5,
      waterMl: 2,
      energyWh: 0.25
    });

    const second = await appendToDailyTotal({ gco2eq: 1, waterMl: 4, energyWh: 0.5 });
    expect(second).toEqual({
      date: '2026-05-16',
      count: 2,
      gco2eq: 1.5,
      waterMl: 6,
      energyWh: 0.75
    });
    expect(await getTodayTotal()).toEqual(second);
  });

  it('counts a repeated measurement twice (no deduplication here)', async () => {
    await appendToDailyTotal({ gco2eq: 1, waterMl: 1, energyWh: 1 });
    await appendToDailyTotal({ gco2eq: 1, waterMl: 1, energyWh: 1 });
    expect((await getTodayTotal()).count).toBe(2);
  });

  it('starts a new total when the local date changes and keeps the old one', async () => {
    await appendToDailyTotal({ gco2eq: 1, waterMl: 1, energyWh: 1 });
    vi.setSystemTime(new Date(2026, 4, 17, 0, 5, 0));

    expect((await getTodayTotal()).count).toBe(0);
    await appendToDailyTotal({ gco2eq: 2, waterMl: 2, energyWh: 2 });

    expect(local.data.has('daily_2026-05-16')).toBe(true);
    expect(local.data.has('daily_2026-05-17')).toBe(true);
    expect((await getTodayTotal()).gco2eq).toBe(2);
  });
});

describe('session log', () => {
  const entry = (n: number): SessionLogEntry => ({
    ts: `t${n}`,
    gco2eq: n,
    waterMl: n,
    energyWh: n
  });

  it('appends entries in order and returns the new log', async () => {
    expect(await appendToSessionLog(entry(1))).toEqual([entry(1)]);
    expect(await appendToSessionLog(entry(2))).toEqual([entry(1), entry(2)]);
    expect(await getSessionLog()).toEqual([entry(1), entry(2)]);
  });

  it('keeps only the last 50 entries', async () => {
    for (let n = 1; n <= 55; n++) await appendToSessionLog(entry(n));
    const log = await getSessionLog();
    expect(log).toHaveLength(50);
    expect(log[0]).toEqual(entry(6));
    expect(log[49]).toEqual(entry(55));
  });
});

describe('pairing state', () => {
  it('stores, reads and clears the pairing', async () => {
    const state = { secret: 's3cret', pairingId: 'p-1', fingerprint: 'fp', pairedAt: 'now' };
    await setPairingState(state);
    expect(await getPairingState()).toEqual(state);

    await clearPairingState();
    expect(await getPairingState()).toBeNull();
  });
});

describe('purgeAll', () => {
  it('clears every key, pairing and team settings included', async () => {
    await setMethod('ecologits');
    await setPairingState({ secret: 's', pairingId: 'p', fingerprint: 'f', pairedAt: 't' });
    local.data.set('team_url', 'https://team.example');
    local.data.set('sobria_projects_list', ['Alpha']);

    await purgeAll();

    expect(local.data.size).toBe(0);
    expect(await getMethod()).toBe('afnor_sobria');
  });
});
