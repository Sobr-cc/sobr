// @vitest-environment node
// SOB-158 — compares the 34 model presets of the extension (`src/lib/presets.ts`,
// a hand copy) with the Rust model registry. The Rust registry is read through
// `crates/sobria-estimator/tests/golden/model_registry.json`: that file is the
// recorded content of `MODEL_REGISTRY`, and the estimator's characterization
// tests fail when the registry changes without the file being regenerated. So
// a change on the Rust side ends here, in a failing test, until the extension
// follows.

import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { MODEL_PRESETS } from '../../src/lib/presets.js';

const here = dirname(fileURLToPath(import.meta.url));
const REGISTRY_GOLDEN = resolve(
  here,
  '../../../crates/sobria-estimator/tests/golden/model_registry.json'
);

type RustPreset = {
  id: string;
  display_name: string;
  provider: string;
  family: string;
  active_params_b: number;
  epsilon_prefill_mj: [number, number, number];
  epsilon_decode_mj: [number, number, number];
  embodied_g_per_req: [number, number, number];
  architecture: { kind: string };
};

const RUST_PRESETS = JSON.parse(readFileSync(REGISTRY_GOLDEN, 'utf8')) as RustPreset[];

/**
 * How a field of the extension preset maps to the Rust one. Only the P50 of
 * the (P5, P50, P95) triplets is copied; for a mixture of experts the extension
 * keeps the ACTIVE parameters (see the header of presets.ts).
 */
const FIELDS: ReadonlyArray<{
  name: string;
  extension: (p: (typeof MODEL_PRESETS)[number]) => unknown;
  rust: (p: RustPreset) => unknown;
}> = [
  { name: 'displayName', extension: (p) => p.displayName, rust: (p) => p.display_name },
  { name: 'vendor', extension: (p) => p.vendor, rust: (p) => p.provider },
  { name: 'family', extension: (p) => p.family, rust: (p) => p.family },
  { name: 'paramsBillion', extension: (p) => p.paramsBillion, rust: (p) => p.active_params_b },
  {
    name: 'epsilonPrefillMjPerToken',
    extension: (p) => p.epsilonPrefillMjPerToken,
    rust: (p) => p.epsilon_prefill_mj[1]
  },
  {
    name: 'epsilonDecodeMjPerToken',
    extension: (p) => p.epsilonDecodeMjPerToken,
    rust: (p) => p.epsilon_decode_mj[1]
  },
  {
    name: 'embodiedGPerRequest',
    extension: (p) => p.embodiedGPerRequest,
    rust: (p) => p.embodied_g_per_req[1]
  },
  {
    name: 'architectureFamily',
    extension: (p) => p.architectureFamily,
    rust: (p) => (p.architecture.kind === 'dense_transformer' ? 'dense' : p.architecture.kind)
  }
];

/**
 * The differences that exist today between the two copies, as
 * `"<preset id>.<field>: extension <x> / rust <y>"`. All numbers match; four
 * labels do not. Tracked by SOB-171, which empties this list. The test fails
 * both when a new difference appears and when one of these is fixed.
 */
const KNOWN_DIFFERENCES: string[] = [
  'o3.displayName: extension "o3" / rust "OpenAI o3"',
  'grok-4.family: extension "grok" / rust "grok-4"',
  'qwen-3-6-plus.displayName: extension "Qwen 3.6 Plus" / rust "Qwen 3.6-Plus"',
  'phi-4-reasoning-vision.displayName: extension "Phi-4 Reasoning Vision" / rust "Phi-4 Reasoning Vision 15B"'
];

describe('extension presets vs Rust model registry', () => {
  it('reads a registry of 34 models on both sides', () => {
    expect(RUST_PRESETS).toHaveLength(34);
    expect(MODEL_PRESETS).toHaveLength(34);
  });

  it('has the same model ids, in the same order', () => {
    expect(MODEL_PRESETS.map((p) => p.id)).toEqual(RUST_PRESETS.map((p) => p.id));
  });

  it('differs from the Rust registry only by the four labels of SOB-171 (all numbers match)', () => {
    const differences: string[] = [];
    for (const rust of RUST_PRESETS) {
      const preset = MODEL_PRESETS.find((p) => p.id === rust.id);
      if (!preset) {
        differences.push(`${rust.id}: missing in the extension`);
        continue;
      }
      for (const field of FIELDS) {
        const ours = field.extension(preset);
        const theirs = field.rust(rust);
        if (!Object.is(ours, theirs)) {
          differences.push(
            `${rust.id}.${field.name}: extension ${JSON.stringify(ours)} / rust ${JSON.stringify(theirs)}`
          );
        }
      }
    }
    expect(differences).toEqual(KNOWN_DIFFERENCES);
  });

  // The ids are the same on both sides (test above), so one check covers both.
  it('has unique ids', () => {
    expect(new Set(MODEL_PRESETS.map((p) => p.id)).size).toBe(MODEL_PRESETS.length);
  });
});
