import type { Page } from '@playwright/test';

/** The demo module as the dev server serves it. */
const DEMO_MODULE = '/src/lib/demo/index.ts';

/**
 * Asks the app's own demo module (`src/lib/demo`, served by the dev server) for the answers of
 * some commands, so a test can feed them to the IPC stub: the stub then returns the engine
 * results of the demo fixtures, and no value is made up in the tests.
 * Loads `/` once to get a page on the app's origin.
 */
export async function demoAnswers(
  page: Page,
  commands: [cmd: string, args?: Record<string, unknown>][]
): Promise<Record<string, unknown>[]> {
  await page.goto('/');
  return page.evaluate(
    async ({ wanted, module }) => {
      const { demoHandlers } = await import(/* @vite-ignore */ module);
      return wanted.map(([cmd, args]) => demoHandlers[cmd](args));
    },
    { wanted: commands, module: DEMO_MODULE }
  );
}
