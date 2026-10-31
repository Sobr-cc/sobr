// SOB-158 — characterization of the three site adapters (ChatGPT, Claude,
// Le Chat). Their alias tables and selectors are private constants, so the
// real content scripts are imported with the prompt detector mocked: the
// detector receives the adapter's own `extractModelId` and `onSubmit`, which
// are then called against small DOM fixtures. This pins what the real tables
// resolve (quirks included), the selectors, and what a submission does.

import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { injectBadge, injectUnsupportedBadge } from '../../src/content/shared/badge-injector.js';
import type { PromptDetectorConfig } from '../../src/content/shared/prompt-detector.js';
import {
  setBadgeVisible,
  setMethod,
  setSitesEnabled,
  todayKey
} from '../../src/content/shared/storage.js';
import { estimate } from '../../src/lib/empreinte/index.js';
import {
  allowSlowImports,
  eventually,
  flush,
  installChrome,
  silenceConsole,
  type ChromeStub
} from './helpers/chrome-stub.js';
import { loadFixture, setFixture } from './helpers/fixtures.js';

allowSlowImports();

const captured = vi.hoisted(() => ({ detectors: [] as unknown[] }));

vi.mock('../../src/content/shared/prompt-detector.js', () => ({
  observePromptSubmission: vi.fn((config: unknown) => {
    captured.detectors.push(config);
  })
}));
vi.mock('../../src/content/shared/composer-indicator.js', () => ({
  injectComposerIndicator: vi.fn(() => () => undefined)
}));
vi.mock('../../src/content/shared/badge-injector.js', () => ({
  injectBadge: vi.fn(),
  injectUnsupportedBadge: vi.fn()
}));
vi.mock('../../src/content/shared/wait-for-bubble.js', () => ({
  waitForNewMatching: vi.fn(async () => undefined)
}));

function setUrl(url: string): void {
  (window as unknown as { happyDOM: { setURL(url: string): void } }).happyDOM.setURL(url);
}

function el(tag: string, attributes: Record<string, string> = {}, text = ''): HTMLElement {
  const node = document.createElement(tag);
  for (const [name, value] of Object.entries(attributes)) node.setAttribute(name, value);
  node.textContent = text;
  return node;
}

/** Lets the bootstrap of a content script (async) reach `observePromptSubmission`. */
async function loadAdapter(
  url: string,
  load: () => Promise<unknown>
): Promise<PromptDetectorConfig> {
  const before = captured.detectors.length;
  setUrl(url);
  await load();
  await eventually(() => expect(captured.detectors.length).toBeGreaterThan(before));
  return captured.detectors.at(-1) as PromptDetectorConfig;
}

type Adapter = {
  name: string;
  url: string;
  load: () => Promise<unknown>;
  host: 'chatgpt' | 'claude' | 'le-chat';
  /** Puts a model label where the adapter reads it. */
  mountLabel: (label: string) => void;
  /** What the real alias table and the shared resolver return for a label. */
  labelCases: ReadonlyArray<readonly [label: string, presetId: string | null]>;
  /**
   * Labels with extra text ("... (new)"): a label equal to a preset id or name is
   * resolved before the alias table is read, so these are the cases that reach
   * each entry of the table, in its order (the first key found in the text wins).
   */
  aliasCases: ReadonlyArray<readonly [label: string, presetId: string | null]>;
  /**
   * Page of the site recorded in tests/fixtures: the real selectors must find
   * its composer and send button, and the model it shows must be recognized.
   */
  fixture: { file: string; presetId: string };
  /** A send button that carries only its main attribute (the recorded page has several). */
  mainSendButton: () => HTMLElement;
  supported: { label: string; presetId: string; displayName: string };
};

const ADAPTERS: Adapter[] = [
  {
    name: 'ChatGPT',
    url: 'https://chatgpt.com/',
    load: () => import('../../src/content/chatgpt.js'),
    host: 'chatgpt',
    mountLabel: (label) =>
      document.body.replaceChildren(
        el('button', { 'data-testid': 'model-switcher-dropdown-button' }, label)
      ),
    labelCases: [
      ['ChatGPT 4o', 'gpt-4o'],
      ['ChatGPT 4o mini', 'gpt-4o-mini'],
      ['GPT-4o', 'gpt-4o'],
      ['GPT-4o mini', 'gpt-4o-mini'],
      ['gpt-4o-mini', 'gpt-4o-mini'],
      ['GPT-4', 'gpt-4o'],
      ['GPT-4 Turbo', 'gpt-4o'],
      ['GPT-5', 'gpt-5-5'],
      ['GPT-5.5', 'gpt-5-5'],
      ['GPT-5.5 Thinking', 'gpt-5-5-thinking'],
      ['GPT-5.5 Pro', 'gpt-5-5-pro'],
      ['GPT-5 Thinking', 'gpt-5-5-thinking'],
      ['GPT-5 Pro', 'gpt-5-5-pro'],
      ['ChatGPT 5', 'gpt-5-5'],
      ['ChatGPT 5.5', 'gpt-5-5'],
      ['ChatGPT 5.5 Thinking', 'gpt-5-5-thinking'],
      ['ChatGPT 5.5 Pro', 'gpt-5-5-pro'],
      ['gpt-5-5', 'gpt-5-5'],
      ['gpt-5-5-thinking', 'gpt-5-5-thinking'],
      ['o3', 'o3'],
      ['o3-mini', 'o3'],
      ['o3-pro', 'o3'],
      // o1 models are billed as GPT-4o models (quirk of the alias table)
      ['o1-preview', 'gpt-4o'],
      ['o1-mini', 'gpt-4o-mini'],
      ['o1', null],
      ['ChatGPT', null],
      ['Auto', null],
      ['GPT-6', null],
      // A space instead of a dot is not an alias (quirk, SOB-176): not recognized
      ['GPT 5.5 Thinking', null],
      // The shared resolver also knows the models of the other vendors (quirk)
      ['Claude Sonnet 4.6', 'claude-sonnet-4-6'],
      ['Mistral Large 3', 'mistral-large-3']
    ],
    aliasCases: [
      ['GPT-5.5 Thinking (new)', 'gpt-5-5-thinking'],
      ['GPT-5.5 Pro (new)', 'gpt-5-5-pro'],
      ['ChatGPT 5.5 Thinking (new)', 'gpt-5-5-thinking'],
      ['ChatGPT 5.5 Pro (new)', 'gpt-5-5-pro'],
      ['ChatGPT 5.5 (new)', 'gpt-5-5'],
      ['ChatGPT 5 (new)', 'gpt-5-5'],
      ['GPT-5 Thinking (new)', 'gpt-5-5-thinking'],
      ['GPT-5 Pro (new)', 'gpt-5-5-pro'],
      ['GPT-5.5 (new)', 'gpt-5-5'],
      ['GPT-5 (new)', 'gpt-5-5'],
      ['ChatGPT-5 (new)', 'gpt-5-5'],
      ['o3-pro (new)', 'o3'],
      ['o3-mini (new)', 'o3'],
      ['o3 (new)', 'o3'],
      ['ChatGPT 4o mini (new)', 'gpt-4o-mini'],
      ['ChatGPT 4o (new)', 'gpt-4o'],
      ['GPT-4o mini (new)', 'gpt-4o-mini'],
      ['GPT-4o-mini (new)', 'gpt-4o-mini'],
      ['GPT-4o (new)', 'gpt-4o'],
      ['GPT-4 Turbo (new)', 'gpt-4o'],
      ['GPT-4 (new)', 'gpt-4o'],
      ['o1-preview (new)', 'gpt-4o'],
      ['o1-mini (new)', 'gpt-4o-mini'],
      // the specific entry comes first: "5.5 pro" is not read as the generic "5.5"
      ['GPT-5.5 Pro Extended', 'gpt-5-5-pro']
    ],
    fixture: { file: 'chatgpt-2026-05.html', presetId: 'gpt-4o' },
    mainSendButton: () => el('button', { 'data-testid': 'send-button' }),
    supported: { label: 'ChatGPT 4o', presetId: 'gpt-4o', displayName: 'GPT-4o' }
  },
  {
    name: 'Claude',
    url: 'https://claude.ai/',
    load: () => import('../../src/content/claude.js'),
    host: 'claude',
    mountLabel: (label) => {
      const nav = el('nav');
      nav.append(el('button', {}, label));
      document.body.replaceChildren(nav);
    },
    labelCases: [
      ['Claude Opus 4.8', 'claude-opus-4-8'],
      ['Opus 4.8', 'claude-opus-4-8'],
      ['Opus 4.7', 'claude-opus-4-7'],
      ['Sonnet 4.6', 'claude-sonnet-4-6'],
      ['Claude Sonnet 4.6', 'claude-sonnet-4-6'],
      ['claude-sonnet-4-6', 'claude-sonnet-4-6'],
      ['Haiku 4.5', 'claude-haiku-4-5'],
      ['Claude Haiku 4.5', 'claude-haiku-4-5'],
      ['Opus 4', 'claude-opus-4'],
      ['Claude Opus 4', 'claude-opus-4'],
      ['Sonnet 4', 'claude-sonnet-4'],
      ['Claude 3.7 Sonnet', 'claude-3-7-sonnet'],
      ['3.7 Sonnet', 'claude-3-7-sonnet'],
      ['Claude 3.5 Sonnet', 'claude-3-5-sonnet'],
      // Other Claude 3 models are all counted as Claude 3.5 Sonnet (quirk)
      ['Claude 3.5 Haiku', 'claude-3-5-sonnet'],
      ['Claude 3 Opus', 'claude-3-5-sonnet'],
      ['Claude 3 Sonnet', 'claude-3-5-sonnet'],
      ['Claude 3 Haiku', 'claude-3-5-sonnet'],
      // A newer minor version falls back on the generic "opus 4" entry (quirk, SOB-176)
      ['Opus 4.5', 'claude-opus-4'],
      ['Opus 4.6', 'claude-opus-4'],
      ['Claude', null],
      ['Sonnet 5', null],
      ['Haiku 3', null],
      // The label must mention Claude, Opus, Sonnet or Haiku to be considered
      ['GPT-4o', null]
    ],
    aliasCases: [
      ['Opus 4.8 (new)', 'claude-opus-4-8'],
      ['Opus 4.7 (new)', 'claude-opus-4-7'],
      ['Sonnet 4.6 (new)', 'claude-sonnet-4-6'],
      ['Haiku 4.5 (new)', 'claude-haiku-4-5'],
      ['Opus 4 (new)', 'claude-opus-4'],
      ['Sonnet 4 (new)', 'claude-sonnet-4'],
      ['3.7 Sonnet (new)', 'claude-3-7-sonnet'],
      ['Claude 3.5 Sonnet (new)', 'claude-3-5-sonnet'],
      ['Claude 3.5 Haiku (new)', 'claude-3-5-sonnet'],
      ['Claude 3 Opus (new)', 'claude-3-5-sonnet'],
      ['Claude 3 Sonnet (new)', 'claude-3-5-sonnet'],
      ['Claude 3 Haiku (new)', 'claude-3-5-sonnet'],
      // the specific entry comes first: "opus 4.8" is not read as the generic "opus 4"
      ['Claude Opus 4.8 Extended', 'claude-opus-4-8']
    ],
    fixture: { file: 'claude-2026-05.html', presetId: 'claude-3-5-sonnet' },
    mainSendButton: () => el('button', { 'aria-label': 'Send Message' }),
    supported: {
      label: 'Claude Sonnet 4.6',
      presetId: 'claude-sonnet-4-6',
      displayName: 'Claude Sonnet 4.6'
    }
  },
  {
    name: 'Le Chat',
    url: 'https://chat.mistral.ai/',
    load: () => import('../../src/content/le-chat.js'),
    host: 'le-chat',
    mountLabel: (label) => document.body.replaceChildren(el('div', { 'data-model': label })),
    labelCases: [
      ['Mistral Large 3', 'mistral-large-3'],
      ['Mistral Large 2', 'mistral-large-2'],
      ['Mistral Large', 'mistral-large-3'],
      ['Mistral Medium 3.5', 'mistral-medium-3-5'],
      ['Mistral Medium 3', 'mistral-medium-3'],
      ['Mistral Medium', 'mistral-medium-3-5'],
      ['Mistral Small 4', 'mistral-small-4'],
      ['Mistral Small', 'mistral-small-4'],
      // An unknown version falls back on the generic entry (quirk, SOB-176)
      ['Mistral Small 3', 'mistral-small-4'],
      ['mistral-large-3', 'mistral-large-3'],
      ['mistral-large-2', 'mistral-large-2'],
      ['Le Chat', null],
      ['Codestral', null],
      ['Magistral', null],
      ['Mistral 7B', null],
      // Any label with a hyphen is considered, even for another vendor (quirk)
      ['gpt-4o', 'gpt-4o']
    ],
    aliasCases: [
      ['Mistral Medium 3.5 (new)', 'mistral-medium-3-5'],
      ['Mistral Medium 3 (new)', 'mistral-medium-3'],
      ['Mistral Large 3 (new)', 'mistral-large-3'],
      ['Mistral Small 4 (new)', 'mistral-small-4'],
      ['Mistral Medium (new)', 'mistral-medium-3-5'],
      ['Mistral Large (new)', 'mistral-large-3'],
      ['Mistral Small (new)', 'mistral-small-4'],
      // The generic "mistral large" entry comes before "mistral large 2": a label with
      // extra text never reaches the Large 2 entry (quirk, SOB-176)
      ['Mistral Large 2 (new)', 'mistral-large-3']
    ],
    fixture: { file: 'le-chat-2026-05.html', presetId: 'mistral-large-2' },
    mainSendButton: () => el('button', { type: 'submit' }),
    supported: {
      label: 'Mistral Large 3',
      presetId: 'mistral-large-3',
      displayName: 'Mistral Large 3'
    }
  }
];

let stub: ChromeStub;

const NOW = new Date('2026-05-16T12:00:00.000Z');
const NOW_ISO = NOW.toISOString();

describe.each(ADAPTERS)('$name adapter', (adapter) => {
  let detector: PromptDetectorConfig;

  beforeAll(async () => {
    silenceConsole('info');
    stub = installChrome();
    detector = await loadAdapter(adapter.url, adapter.load);
  });

  afterAll(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  beforeEach(() => {
    stub.local.data.clear();
    vi.clearAllMocks();
    document.body.replaceChildren();
    setUrl(adapter.url);
  });

  describe('model detection with the real alias table', () => {
    it.each([...adapter.labelCases, ...adapter.aliasCases])(
      'label "%s" resolves to %s',
      (label, presetId) => {
        adapter.mountLabel(label);
        expect(detector.extractModelId()).toBe(presetId);
      }
    );

    it('resolves to null when the page shows no model label', () => {
      expect(detector.extractModelId()).toBeNull();
    });
  });

  describe('recorded page', () => {
    beforeEach(() => {
      setFixture(loadFixture(adapter.fixture.file));
    });

    it('is found by the real selectors: composer and send button', () => {
      expect(document.querySelector(detector.selectorTextarea)).not.toBeNull();
      expect(document.querySelector(detector.selectorSendButton)).not.toBeNull();
    });

    it('shows a model that the adapter recognizes', () => {
      expect(detector.extractModelId()).toBe(adapter.fixture.presetId);
    });
  });

  it('finds a send button that carries only its main attribute', () => {
    const button = adapter.mainSendButton();
    document.body.append(button);
    expect(document.querySelector(detector.selectorSendButton)).toBe(button);
  });

  describe.runIf(adapter.host === 'chatgpt')('model from the URL', () => {
    it('falls back on the ?model= parameter when the page shows no label', () => {
      setUrl('https://chatgpt.com/?model=gpt-4o-mini');
      expect(detector.extractModelId()).toBe('gpt-4o-mini');
      setUrl('https://chatgpt.com/?model=o3-pro');
      expect(detector.extractModelId()).toBe('o3');
    });

    it('prefers the label of the page over the parameter', () => {
      setUrl('https://chatgpt.com/?model=gpt-4o-mini');
      adapter.mountLabel('GPT-5.5 Pro');
      expect(detector.extractModelId()).toBe('gpt-5-5-pro');
    });

    it('falls back on the parameter when the label is not recognized', () => {
      setUrl('https://chatgpt.com/?model=gpt-4o');
      adapter.mountLabel('Auto');
      expect(detector.extractModelId()).toBe('gpt-4o');
    });

    it('is null for an unknown parameter', () => {
      setUrl('https://chatgpt.com/?model=something-new');
      expect(detector.extractModelId()).toBeNull();
    });

    it('reads the model switcher before the other buttons that mention GPT', () => {
      document.body.replaceChildren(
        el('button', { 'aria-label': 'GPT-4o mini' }),
        el('button', { 'data-testid': 'model-switcher-dropdown-button' }, 'GPT-5.5')
      );
      expect(detector.extractModelId()).toBe('gpt-5-5');
    });
  });

  describe('submission of a prompt', () => {
    const PROMPT = 'Bonjour le monde'; // 16 characters: 5 tokens in, 200 tokens out (minimum)
    const EXPECTED = estimate({
      method: 'afnor_sobria',
      modelId: adapter.supported.presetId,
      tokensIn: 5,
      tokensOut: 200
    });
    const SUM = {
      gco2eq: EXPECTED.gco2eq,
      waterMl: EXPECTED.waterMl,
      energyWh: EXPECTED.energyWh
    };

    beforeEach(() => {
      vi.useFakeTimers({ toFake: ['Date'] });
      vi.setSystemTime(NOW);
    });

    afterEach(() => {
      vi.useRealTimers();
    });

    function addActionsRow(): HTMLElement {
      const row = el('div', { id: 'actions' });
      row.append(el('button', { 'aria-label': 'Copy' }));
      document.body.append(row);
      return row;
    }

    it('estimates, stores locally, shows the badge and tells the service worker', async () => {
      adapter.mountLabel(adapter.supported.label);
      const row = addActionsRow();

      await detector.onSubmit({ prompt: PROMPT, modelId: adapter.supported.presetId });

      const daily = { date: todayKey(NOW), count: 1, ...SUM };
      expect(Object.fromEntries(stub.local.data)).toEqual({
        last_estimate: {
          estimate: EXPECTED,
          host: adapter.host,
          modelDisplayName: adapter.supported.displayName,
          ts: NOW_ISO
        },
        [`daily_${todayKey(NOW)}`]: daily,
        session_log: [{ ts: NOW_ISO, ...SUM }]
      });
      expect(injectBadge).toHaveBeenCalledWith(row, EXPECTED, { session: daily });
      expect(injectUnsupportedBadge).not.toHaveBeenCalled();
      expect(stub.runtime.sendMessage).toHaveBeenCalledWith({
        type: 'estimation_submitted',
        estimate: EXPECTED,
        host: adapter.host,
        modelDisplayName: adapter.supported.displayName
      });
    });

    it('gives the badge the running total of the day', async () => {
      addActionsRow();

      await detector.onSubmit({ prompt: PROMPT, modelId: adapter.supported.presetId });
      await detector.onSubmit({ prompt: PROMPT, modelId: adapter.supported.presetId });

      const [, second] = vi.mocked(injectBadge).mock.calls;
      expect(second?.[2]).toEqual({
        session: {
          date: todayKey(NOW),
          count: 2,
          gco2eq: SUM.gco2eq * 2,
          waterMl: SUM.waterMl * 2,
          energyWh: SUM.energyWh * 2
        }
      });
      expect(stub.local.data.get('session_log')).toHaveLength(2);
    });

    it('uses the method chosen by the user', async () => {
      await setMethod('ecologits');
      await detector.onSubmit({ prompt: PROMPT, modelId: adapter.supported.presetId });
      const sent = stub.runtime.sendMessage.mock.calls[0]?.[0] as { estimate: { method: string } };
      expect(sent.estimate.method).toBe('ecologits');
    });

    it('still stores and reports, but shows no badge, when the badge is turned off', async () => {
      await setBadgeVisible(false);
      addActionsRow();

      await detector.onSubmit({ prompt: PROMPT, modelId: adapter.supported.presetId });

      expect(injectBadge).not.toHaveBeenCalled();
      expect(stub.local.data.has('last_estimate')).toBe(true);
      expect(stub.runtime.sendMessage).toHaveBeenCalledTimes(1);
    });

    it('does not fail when the page has no row of actions to attach the badge to', async () => {
      await detector.onSubmit({ prompt: PROMPT, modelId: adapter.supported.presetId });
      expect(injectBadge).not.toHaveBeenCalled();
      expect(stub.runtime.sendMessage).toHaveBeenCalledTimes(1);
    });

    it('does not fail when the service worker is not reachable', async () => {
      stub.runtime.sendMessage.mockRejectedValueOnce(new Error('no receiver'));
      await expect(
        detector.onSubmit({ prompt: PROMPT, modelId: adapter.supported.presetId })
      ).resolves.toBeUndefined();
      expect(stub.local.data.has('last_estimate')).toBe(true);
    });

    it('shows the "unsupported model" badge and records nothing when the model is unknown', async () => {
      adapter.mountLabel(adapter.supported.label);
      const row = addActionsRow();

      await detector.onSubmit({ prompt: PROMPT, modelId: null });

      expect(injectUnsupportedBadge).toHaveBeenCalledWith(row, {
        modelLabel: adapter.supported.label
      });
      expect(injectBadge).not.toHaveBeenCalled();
      expect(stub.local.data.size).toBe(0);
      expect(stub.runtime.sendMessage).not.toHaveBeenCalled();
    });

    it('shows the unsupported badge without a label when none can be read', async () => {
      const row = addActionsRow();
      await detector.onSubmit({ prompt: PROMPT, modelId: null });
      expect(injectUnsupportedBadge).toHaveBeenCalledWith(row, {});
    });

    it('logs an error and records nothing for a model id the engine does not know', async () => {
      const [error] = silenceConsole('error');

      await expect(
        detector.onSubmit({ prompt: PROMPT, modelId: 'not-a-model' })
      ).resolves.toBeUndefined();

      expect(error).toHaveBeenCalledWith(expect.stringContaining('[sobria]'), expect.any(Error));
      expect(stub.local.data.size).toBe(0);
      expect(stub.runtime.sendMessage).not.toHaveBeenCalled();
    });
  });
});

describe('when an adapter starts', () => {
  beforeEach(() => {
    silenceConsole('info');
    stub = installChrome();
  });

  afterAll(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  const ALL_SITES = { chatgpt: true, claude: true, leChat: true };

  async function startsOn(url: string, adapter: Adapter, sites = ALL_SITES): Promise<boolean> {
    await setSitesEnabled(sites);
    const before = captured.detectors.length;
    setUrl(url);
    vi.resetModules();
    await adapter.load();
    await flush();
    return captured.detectors.length > before;
  }

  const [chatgpt, claude, leChat] = ADAPTERS as [Adapter, Adapter, Adapter];

  it('watches prompts on its own site', async () => {
    expect(await startsOn(chatgpt.url, chatgpt)).toBe(true);
    expect(await startsOn(claude.url, claude)).toBe(true);
    expect(await startsOn(leChat.url, leChat)).toBe(true);
  });

  it('ChatGPT also runs on the legacy chat.openai.com', async () => {
    expect(await startsOn('https://chat.openai.com/', chatgpt)).toBe(true);
  });

  it('does nothing on any other site', async () => {
    expect(await startsOn('https://example.com/', chatgpt)).toBe(false);
    expect(await startsOn('https://claude.ai.evil.example/', claude)).toBe(false);
    expect(await startsOn('https://chatgpt.com/', claude)).toBe(false);
    expect(await startsOn('https://chat.mistral.ai.evil.example/', leChat)).toBe(false);
  });

  it('does nothing when the site is turned off in the options', async () => {
    expect(await startsOn(chatgpt.url, chatgpt, { ...ALL_SITES, chatgpt: false })).toBe(false);
    expect(await startsOn(claude.url, claude, { ...ALL_SITES, claude: false })).toBe(false);
    expect(await startsOn(leChat.url, leChat, { ...ALL_SITES, leChat: false })).toBe(false);
  });

  it('only turns off the site that was switched off', async () => {
    expect(
      await startsOn(claude.url, claude, { chatgpt: false, claude: true, leChat: false })
    ).toBe(true);
  });
});
