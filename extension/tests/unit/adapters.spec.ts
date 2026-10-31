// SOB-158 — characterization of the three site adapters (ChatGPT, Claude,
// Le Chat). Their alias tables and selectors are private constants, so the
// real content scripts are imported with the prompt detector mocked: the
// detector receives the adapter's own `extractModelId` and `onSubmit`, which
// are then called against small DOM fixtures. This pins what the real tables
// resolve (quirks included), the selectors, and what a submission does.

import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { injectBadge, injectUnsupportedBadge } from '../../src/content/shared/badge-injector.js';
import type { PromptDetectorConfig } from '../../src/content/shared/prompt-detector.js';
import { estimate } from '../../src/lib/empreinte/index.js';
import { eventually, installChrome } from './helpers/chrome-stub.js';

// Importing the real scripts is slow on a loaded machine (coverage, parallel builds).
vi.setConfig({ testTimeout: 30_000, hookTimeout: 30_000 });

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
  logName: string;
  /** Puts a model label where the adapter reads it. */
  mountLabel: (label: string) => void;
  /** What the real alias table and the shared resolver return for a label. */
  labelCases: ReadonlyArray<readonly [label: string, presetId: string | null]>;
  /** Sample composer DOM that the real selectors must find. */
  composer: { textarea: HTMLElement; sendButton: HTMLElement };
  supported: { label: string; presetId: string; displayName: string };
};

const ADAPTERS: Adapter[] = [
  {
    name: 'ChatGPT',
    url: 'https://chatgpt.com/',
    load: () => import('../../src/content/chatgpt.js'),
    host: 'chatgpt',
    logName: 'ChatGPT',
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
      // A space instead of a dot is not an alias (quirk): not recognized
      ['GPT 5.5 Thinking', null],
      // The shared resolver also knows the models of the other vendors (quirk)
      ['Claude Sonnet 4.6', 'claude-sonnet-4-6'],
      ['Mistral Large 3', 'mistral-large-3']
    ],
    composer: {
      textarea: el('div', { contenteditable: 'true', id: 'prompt-textarea' }),
      sendButton: el('button', { 'data-testid': 'send-button' })
    },
    supported: { label: 'ChatGPT 4o', presetId: 'gpt-4o', displayName: 'GPT-4o' }
  },
  {
    name: 'Claude',
    url: 'https://claude.ai/',
    load: () => import('../../src/content/claude.js'),
    host: 'claude',
    logName: 'Claude',
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
      // A newer minor version falls back on the generic "opus 4" entry (quirk)
      ['Opus 4.5', 'claude-opus-4'],
      ['Opus 4.6', 'claude-opus-4'],
      ['Claude', null],
      ['Sonnet 5', null],
      ['Haiku 3', null],
      // The label must mention Claude, Opus, Sonnet or Haiku to be considered
      ['GPT-4o', null]
    ],
    composer: {
      textarea: el('div', { contenteditable: 'true', role: 'textbox' }),
      sendButton: el('button', { 'aria-label': 'Send Message' })
    },
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
    logName: 'Le Chat',
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
      // An unknown version falls back on the generic entry (quirk)
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
    composer: {
      textarea: el('textarea', { name: 'message' }),
      sendButton: el('button', { type: 'submit' })
    },
    supported: {
      label: 'Mistral Large 3',
      presetId: 'mistral-large-3',
      displayName: 'Mistral Large 3'
    }
  }
];

let stub: ReturnType<typeof installChrome>;

/** Detector config captured for each adapter, by name. */
const detectors = new Map<string, PromptDetectorConfig>();

describe.each(ADAPTERS)('$name adapter', (adapter) => {
  let detector: PromptDetectorConfig;

  beforeAll(async () => {
    vi.spyOn(console, 'info').mockImplementation(() => undefined);
    stub = installChrome();
    detector = await loadAdapter(adapter.url, adapter.load);
    detectors.set(adapter.name, detector);
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
    it.each(adapter.labelCases)('label "%s" resolves to %s', (label, presetId) => {
      adapter.mountLabel(label);
      expect(detector.extractModelId()).toBe(presetId);
    });

    it('resolves to null when the page shows no model label', () => {
      expect(detector.extractModelId()).toBeNull();
    });
  });

  describe('selectors', () => {
    it('finds the composer and the send button of a typical page', () => {
      document.body.append(adapter.composer.textarea, adapter.composer.sendButton);
      expect(document.querySelector(detector.selectorTextarea)).toBe(adapter.composer.textarea);
      expect(document.querySelector(detector.selectorSendButton)).toBe(adapter.composer.sendButton);
    });
  });

  describe('submission of a prompt', () => {
    const PROMPT = 'Bonjour le monde'; // 16 characters: 5 tokens in, 200 tokens out (minimum)

    function addActionsRow(): HTMLElement {
      const row = el('div', { id: 'actions' });
      row.append(el('button', { 'aria-label': 'Copy' }));
      document.body.append(row);
      return row;
    }

    it('estimates, stores locally, shows the badge and tells the service worker', async () => {
      adapter.mountLabel(adapter.supported.label);
      const row = addActionsRow();
      const expected = estimate({
        method: 'afnor_sobria',
        modelId: adapter.supported.presetId,
        tokensIn: 5,
        tokensOut: 200
      });

      await detector.onSubmit({ prompt: PROMPT, modelId: adapter.supported.presetId });

      expect(stub.local.data.get('last_estimate')).toEqual({
        estimate: expected,
        host: adapter.host,
        modelDisplayName: adapter.supported.displayName,
        ts: expect.any(String)
      });
      const daily = [...stub.local.data].find(([key]) => key.startsWith('daily_'))?.[1];
      expect(daily).toMatchObject({
        count: 1,
        gco2eq: expected.gco2eq,
        waterMl: expected.waterMl,
        energyWh: expected.energyWh
      });
      expect(stub.local.data.get('session_log')).toHaveLength(1);
      expect(injectBadge).toHaveBeenCalledWith(row, expected, {
        session: expect.objectContaining({ count: 1 })
      });
      expect(injectUnsupportedBadge).not.toHaveBeenCalled();
      expect(stub.runtime.sendMessage).toHaveBeenCalledWith({
        type: 'estimation_submitted',
        estimate: expected,
        host: adapter.host,
        modelDisplayName: adapter.supported.displayName
      });
    });

    it('uses the method chosen by the user', async () => {
      stub.local.data.set('method', 'ecologits');
      await detector.onSubmit({ prompt: PROMPT, modelId: adapter.supported.presetId });
      const sent = stub.runtime.sendMessage.mock.calls[0]?.[0] as { estimate: { method: string } };
      expect(sent.estimate.method).toBe('ecologits');
    });

    it('still stores and reports, but shows no badge, when the badge is turned off', async () => {
      stub.local.data.set('badge_visible', false);
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
      stub.runtime.sendMessage = vi.fn(() => Promise.reject(new Error('no receiver')));
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
      const error = vi.spyOn(console, 'error').mockImplementation(() => undefined);

      await expect(
        detector.onSubmit({ prompt: PROMPT, modelId: 'not-a-model' })
      ).resolves.toBeUndefined();

      expect(error).toHaveBeenCalledWith(
        `[sobria] estimation ${adapter.logName} échouée :`,
        expect.any(Error)
      );
      expect(stub.local.data.size).toBe(0);
      expect(stub.runtime.sendMessage).not.toHaveBeenCalled();
    });
  });
});

describe('ChatGPT model from the URL', () => {
  // The ChatGPT module is imported once (above); its detector is reused.
  let detector: PromptDetectorConfig;

  beforeAll(() => {
    const captured = detectors.get('ChatGPT');
    if (!captured) throw new Error('the ChatGPT detector was not captured');
    detector = captured;
  });

  beforeEach(() => {
    document.body.replaceChildren();
  });

  it('falls back on the ?model= parameter when the page shows no label', () => {
    setUrl('https://chatgpt.com/?model=gpt-4o-mini');
    expect(detector.extractModelId()).toBe('gpt-4o-mini');
    setUrl('https://chatgpt.com/?model=o3-pro');
    expect(detector.extractModelId()).toBe('o3');
  });

  it('prefers the label of the page over the parameter', () => {
    setUrl('https://chatgpt.com/?model=gpt-4o-mini');
    document.body.replaceChildren(
      el('button', { 'data-testid': 'model-switcher-dropdown-button' }, 'GPT-5.5 Pro')
    );
    expect(detector.extractModelId()).toBe('gpt-5-5-pro');
  });

  it('falls back on the parameter when the label is not recognized', () => {
    setUrl('https://chatgpt.com/?model=gpt-4o');
    document.body.replaceChildren(
      el('button', { 'data-testid': 'model-switcher-dropdown-button' }, 'Auto')
    );
    expect(detector.extractModelId()).toBe('gpt-4o');
  });

  it('is null for an unknown parameter', () => {
    setUrl('https://chatgpt.com/?model=something-new');
    expect(detector.extractModelId()).toBeNull();
  });

  it('reads the model switcher before the other buttons that mention GPT', () => {
    setUrl('https://chatgpt.com/');
    document.body.replaceChildren(
      el('button', { 'aria-label': 'GPT-4o mini' }),
      el('button', { 'data-testid': 'model-switcher-dropdown-button' }, 'GPT-5.5')
    );
    expect(detector.extractModelId()).toBe('gpt-5-5');
  });
});

describe('when an adapter starts', () => {
  beforeEach(() => {
    vi.spyOn(console, 'info').mockImplementation(() => undefined);
    stub = installChrome();
  });

  afterAll(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  async function startsOn(url: string, adapter: Adapter, sites?: object): Promise<boolean> {
    if (sites) stub.local.data.set('sites_enabled', sites);
    const before = captured.detectors.length;
    setUrl(url);
    vi.resetModules();
    await adapter.load();
    await new Promise((resolve) => setTimeout(resolve, 25));
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
    const all = { chatgpt: true, claude: true, leChat: true };
    expect(await startsOn(chatgpt.url, chatgpt, { ...all, chatgpt: false })).toBe(false);
    expect(await startsOn(claude.url, claude, { ...all, claude: false })).toBe(false);
    expect(await startsOn(leChat.url, leChat, { ...all, leChat: false })).toBe(false);
  });

  it('only turns off the site that was switched off', async () => {
    expect(
      await startsOn(claude.url, claude, { chatgpt: false, claude: true, leChat: false })
    ).toBe(true);
  });
});
