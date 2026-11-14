import { expect, test } from 'bun:test';
import { initSidecar, SIDECAR_HTTP, SIDECAR_WS, LOCAL_SIDECAR_HTTP } from './sidecar.client';

test('initializes and updates sidecar endpoints', async () => {
  let update: (endpoint: string) => void = () => {};
  await initSidecar(
    async () => '127.0.0.1:19877/token',
    async () => '127.0.0.1:19977',
    async (callback) => {
      update = callback;
    }
  );
  expect(SIDECAR_HTTP).toBe('http://127.0.0.1:19877/token');
  expect(LOCAL_SIDECAR_HTTP).toBe('http://127.0.0.1:19977');
  expect(SIDECAR_WS).toBe('ws://127.0.0.1:19877/token');
  update('127.0.0.1:19977');
  expect(SIDECAR_HTTP).toBe('http://127.0.0.1:19977');
  expect(SIDECAR_WS).toBe('ws://127.0.0.1:19977');
});

test('keeps a connection change received during bootstrap', async () => {
  let update: (endpoint: string) => void = () => {};
  await initSidecar(
    async () => {
      update('127.0.0.1:1234/token');
      return '127.0.0.1:19977';
    },
    async () => '127.0.0.1:19977',
    async (callback) => {
      update = callback;
    }
  );
  expect(SIDECAR_HTTP).toBe('http://127.0.0.1:1234/token');
  expect(SIDECAR_WS).toBe('ws://127.0.0.1:1234/token');
});
