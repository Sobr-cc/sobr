import { get } from 'svelte/store';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('./api', () => ({
  getTeamStatus: vi.fn(),
  setTeamUrl: vi.fn(),
  setTeamMode: vi.fn(),
  setTeamAcceptInvalidCerts: vi.fn()
}));

const INITIAL = {
  enrolled: false,
  url: null,
  user_id: null,
  mode: 'local',
  fingerprint: null,
  enrolled_at: null,
  accept_invalid_certs: false,
  last_seen_at: null,
  estimations_sent: 0,
  loaded: false
};

const ENROLLED = {
  enrolled: true,
  url: 'https://team.example.org',
  user_id: 'u-1',
  mode: 'both' as const,
  fingerprint: 'ab:cd',
  enrolled_at: '2026-05-12T14:32:08Z',
  accept_invalid_certs: true,
  last_seen_at: '2026-05-13T09:00:00Z',
  estimations_sent: 12
};

async function fresh() {
  vi.resetModules();
  const api = vi.mocked(await import('./api'));
  const store = await import('./team-store');
  return { api, ...store };
}

async function freshLoaded() {
  const ctx = await fresh();
  ctx.api.getTeamStatus.mockResolvedValue(ENROLLED);
  await ctx.loadTeam();
  return ctx;
}

type LooseSave = (field: string, value: unknown) => Promise<void>;

beforeEach(() => {
  vi.resetAllMocks();
});

describe('teamStore', () => {
  it('starts local, not enrolled and not loaded', async () => {
    const { teamStore } = await fresh();
    expect(get(teamStore)).toEqual(INITIAL);
  });
});

describe('loadTeam', () => {
  it('replaces the state with the snapshot and marks it loaded', async () => {
    const { api, teamStore, loadTeam } = await fresh();
    api.getTeamStatus.mockResolvedValue(ENROLLED);

    await loadTeam();

    expect(get(teamStore)).toEqual({ ...ENROLLED, loaded: true });
  });

  it('keeps the previous state and rethrows when the IPC fails', async () => {
    const { api, teamStore, loadTeam } = await fresh();
    const error = new Error('ipc down');
    api.getTeamStatus.mockRejectedValue(error);

    await expect(loadTeam()).rejects.toBe(error);

    expect(get(teamStore)).toEqual(INITIAL);
  });
});

describe('saveTeamField', () => {
  it('updates the url locally before the IPC answers, then keeps it', async () => {
    const { api, teamStore, saveTeamField } = await fresh();
    const pending = Promise.withResolvers<undefined>();
    api.setTeamUrl.mockReturnValue(pending.promise);

    const saving = saveTeamField('url', 'https://team.example.org');

    expect(get(teamStore).url).toBe('https://team.example.org');
    expect(api.setTeamUrl.mock.calls).toEqual([['https://team.example.org']]);
    pending.resolve(undefined);
    await saving;
    expect(get(teamStore)).toEqual({ ...INITIAL, url: 'https://team.example.org' });
  });

  it.each(['local', 'team', 'both'] as const)('persists the mode %s', async (mode) => {
    const { api, teamStore, saveTeamField } = await fresh();
    api.setTeamMode.mockResolvedValue();

    await saveTeamField('mode', mode);

    expect(get(teamStore)).toEqual({ ...INITIAL, mode });
    expect(api.setTeamMode.mock.calls).toEqual([[mode]]);
  });

  it('persists accept_invalid_certs', async () => {
    const { api, teamStore, saveTeamField } = await fresh();
    api.setTeamAcceptInvalidCerts.mockResolvedValue();

    await saveTeamField('accept_invalid_certs', true);

    expect(get(teamStore)).toEqual({ ...INITIAL, accept_invalid_certs: true });
    expect(api.setTeamAcceptInvalidCerts.mock.calls).toEqual([[true]]);
  });

  it('touches only the saved field and leaves the others as loaded', async () => {
    const { api, teamStore, loadTeam, saveTeamField } = await fresh();
    api.getTeamStatus.mockResolvedValue(ENROLLED);
    api.setTeamMode.mockResolvedValue();
    await loadTeam();

    await saveTeamField('mode', 'team');

    expect(get(teamStore)).toEqual({ ...ENROLLED, mode: 'team', loaded: true });
  });

  it('restores the previous state and rethrows when the url IPC fails', async () => {
    const { api, teamStore, loadTeam, saveTeamField } = await fresh();
    api.getTeamStatus.mockResolvedValue(ENROLLED);
    await loadTeam();
    const before = get(teamStore);
    const error = new Error('url must start with https://');
    api.setTeamUrl.mockRejectedValue(error);

    await expect(saveTeamField('url', 'http://insecure')).rejects.toBe(error);

    expect(get(teamStore)).toEqual(before);
  });

  it('restores the previous state and rethrows when the mode IPC fails', async () => {
    const { api, teamStore, saveTeamField } = await freshLoaded();
    const before = get(teamStore);
    const error = new Error('storage');
    api.setTeamMode.mockRejectedValue(error);

    await expect(saveTeamField('mode', 'team')).rejects.toBe(error);

    expect(get(teamStore)).toEqual(before);
    expect(before.mode).toBe('both');
  });

  it('restores the previous state and rethrows when the certificate IPC fails', async () => {
    const { api, teamStore, saveTeamField } = await freshLoaded();
    const before = get(teamStore);
    const error = new Error('storage');
    api.setTeamAcceptInvalidCerts.mockRejectedValue(error);

    await expect(saveTeamField('accept_invalid_certs', false)).rejects.toBe(error);

    expect(get(teamStore)).toEqual(before);
    expect(before.accept_invalid_certs).toBe(true);
  });

  it('rolls back to the initial state when nothing was loaded', async () => {
    const { api, teamStore, saveTeamField } = await fresh();
    api.setTeamMode.mockRejectedValue(new Error('storage'));

    await expect(saveTeamField('mode', 'team')).rejects.toThrow('storage');

    expect(get(teamStore)).toEqual(INITIAL);
  });

  it('shows the optimistic value while the IPC is pending, then rolls back', async () => {
    const { api, teamStore, saveTeamField } = await freshLoaded();
    const pending = Promise.withResolvers<undefined>();
    api.setTeamMode.mockReturnValue(pending.promise);

    const saving = saveTeamField('mode', 'local');
    expect(get(teamStore).mode).toBe('local');
    pending.reject(new Error('offline'));

    await expect(saving).rejects.toThrow('offline');
    expect(get(teamStore).mode).toBe('both');
  });

  it('forwards a value outside the mode union to the IPC without patching the store', async () => {
    const { api, teamStore, saveTeamField } = await fresh();
    api.setTeamMode.mockResolvedValue();

    await (saveTeamField as LooseSave)('mode', 'weird');

    expect(api.setTeamMode.mock.calls).toEqual([['weird']]);
    expect(get(teamStore)).toEqual(INITIAL);
  });

  it('forwards a url that is not a string to the IPC without patching the store', async () => {
    const { api, teamStore, saveTeamField } = await fresh();
    api.setTeamUrl.mockResolvedValue();

    await (saveTeamField as LooseSave)('url', 5);

    expect(api.setTeamUrl.mock.calls).toEqual([[5]]);
    expect(get(teamStore)).toEqual(INITIAL);
  });
});
