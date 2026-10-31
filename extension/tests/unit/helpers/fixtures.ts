// Recorded pages of the three sites (tests/fixtures/*.html), loaded into the
// document. The older specs keep their own copy of these two helpers.

import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));

/** Content of tests/fixtures/<name>. */
export function loadFixture(name: string): string {
  return readFileSync(resolve(here, '..', '..', 'fixtures', name), 'utf8');
}

/** Replaces the document with a recorded page. */
export function setFixture(html: string): void {
  document.documentElement.innerHTML = html.replace(/^<!doctype html>/i, '').trim();
}
