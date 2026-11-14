import { expect, test } from 'bun:test';
import { initSidecar, SIDECAR_HTTP, SIDECAR_WS } from './sidecar.client';

test('initializes both sidecar endpoints from the native port', async () => {
  await initSidecar(async () => 19877);
  expect(SIDECAR_HTTP).toBe('http://127.0.0.1:19877');
  expect(SIDECAR_WS).toBe('ws://127.0.0.1:19877');
});
