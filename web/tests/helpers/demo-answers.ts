import type { Page } from '@playwright/test';

/** Answers of the demo handlers (src/lib/demo, via the dev server), to feed the IPC stub. */
const DEMO_MODULE = '/src/lib/demo/index.ts';

const cache = new Map<string, Promise<Record<string, unknown>[]>>();

async function fetchAnswers(
  page: Page,
  commands: [cmd: string, args?: Record<string, unknown>][]
): Promise<Record<string, unknown>[]> {
  await page.goto('/');
  return page.evaluate(
    async ({ wanted, module }) => {
      const { demoHandlers } = await import(module);
      return Promise.all(wanted.map(([cmd, args]) => demoHandlers[cmd](args)));
    },
    { wanted: commands, module: DEMO_MODULE }
  );
}

/** The answers are pure (seed 42): each distinct request is computed once per worker. */
export function demoAnswers(
  page: Page,
  commands: [cmd: string, args?: Record<string, unknown>][]
): Promise<Record<string, unknown>[]> {
  const key = JSON.stringify(commands);
  let answers = cache.get(key);
  if (!answers) {
    answers = fetchAnswers(page, commands);
    cache.set(key, answers);
  }
  return answers;
}
