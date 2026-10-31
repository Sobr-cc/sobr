import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

import { invoke } from '@tauri-apps/api/core';

import { estimatePrompt, metaInfo, SobriaIpcError, verifyAudit } from './api';

const mockedInvoke = vi.mocked(invoke);

async function rejection(err: unknown): Promise<SobriaIpcError> {
  mockedInvoke.mockRejectedValue(err);
  const caught: unknown = await metaInfo().then(
    () => undefined,
    (e: unknown) => e
  );
  expect(caught).toBeInstanceOf(SobriaIpcError);
  return caught as SobriaIpcError;
}

describe('IPC error normalization (Tauri context)', () => {
  beforeEach(() => {
    vi.stubGlobal('window', { __TAURI_INTERNALS__: {} });
    mockedInvoke.mockReset();
  });

  it('returns the invoke result untouched on success', async () => {
    mockedInvoke.mockResolvedValue({ app_version: '1.0.0' });

    await expect(metaInfo()).resolves.toEqual({ app_version: '1.0.0' });
    expect(mockedInvoke.mock.calls).toEqual([['meta_info', undefined]]);
  });

  it('passes a SobriaIpcError through as the same instance', async () => {
    const original = new SobriaIpcError('not_found', 'no such entry', { id: 7 });

    const e = await rejection(original);

    expect(e).toBe(original);
  });

  it('rebuilds the serialized IpcError sent by Tauri', async () => {
    const e = await rejection({ code: 'unknown_model', message: 'unknown model x', details: [1] });

    expect(e).toBeInstanceOf(Error);
    expect(e.name).toBe('SobriaIpcError');
    expect(e.code).toBe('unknown_model');
    expect(e.message).toBe('unknown model x');
    expect(e.details).toEqual([1]);
  });

  it('leaves details undefined when the payload has none', async () => {
    const e = await rejection({ code: 'io_error', message: 'disk full' });

    expect(e.code).toBe('io_error');
    expect(e.details).toBeUndefined();
  });

  it('does not validate the code against the known IpcErrorCode values', async () => {
    const e = await rejection({ code: 'brand_new_code', message: 'm' });

    expect(e.code).toBe('brand_new_code');
  });

  it('keeps the code of an Error that carries a string code', async () => {
    const e = await rejection(Object.assign(new Error('missing file'), { code: 'ENOENT' }));

    expect(e.code).toBe('ENOENT');
    expect(e.message).toBe('missing file');
  });

  it('maps a plain Error to the internal code with its message', async () => {
    const e = await rejection(new Error('boom'));

    expect(e.code).toBe('internal');
    expect(e.message).toBe('boom');
    expect(e.details).toBeUndefined();
  });

  it('maps a string to the internal code with the string as message', async () => {
    const e = await rejection('plain text failure');

    expect(e.code).toBe('internal');
    expect(e.message).toBe('plain text failure');
  });

  it.each([
    ['null', null],
    ['undefined', undefined],
    ['a number', 42],
    ['an object without code and message', { reason: 'x' }],
    ['an object whose code is not a string', { code: 500, message: 'server' }],
    ['an object whose message is not a string', { code: 'http_error', message: 500 }],
    ['an object with a code but no message', { code: 'http_error' }]
  ])('maps %s to the generic internal error', async (_label, err) => {
    const e = await rejection(err);

    expect(e.code).toBe('internal');
    expect(e.message).toBe('Erreur IPC inconnue');
    expect(e.details).toBeUndefined();
  });
});

describe('IPC error normalization (web demo, no Tauri)', () => {
  beforeEach(() => {
    vi.unstubAllGlobals();
    vi.stubGlobal('window', {});
    mockedInvoke.mockReset();
  });

  it('turns a raw {code, message} thrown by a demo handler into a SobriaIpcError', async () => {
    const caught: unknown = await estimatePrompt({
      model_id: 'no-such-model',
      tokens_in: 100,
      tokens_out_estimated: 100
    } as Parameters<typeof estimatePrompt>[0]).then(
      () => undefined,
      (e: unknown) => e
    );

    expect(caught).toBeInstanceOf(SobriaIpcError);
    expect((caught as SobriaIpcError).code).toBe('unknown_model');
    expect(mockedInvoke).not.toHaveBeenCalled();
  });

  it('rejects a command the demo does not cover with tauri_unavailable', async () => {
    const caught: unknown = await verifyAudit().then(
      () => undefined,
      (e: unknown) => e
    );

    expect(caught).toBeInstanceOf(SobriaIpcError);
    expect((caught as SobriaIpcError).code).toBe('tauri_unavailable');
    expect((caught as SobriaIpcError).message).toContain("l'application de bureau Sobr.ia");
    expect(mockedInvoke).not.toHaveBeenCalled();
  });
});
