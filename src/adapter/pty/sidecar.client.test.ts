import { expect, test } from 'bun:test';
import { initSidecar, isRemoteReconnect, SIDECAR_HTTP, SIDECAR_WS, LOCAL_SIDECAR_HTTP } from './sidecar.client';

test('reloads only when an active remote endpoint is replaced', () => {
  expect(isRemoteReconnect('local', 'remote', 'local')).toBe(false);
  expect(isRemoteReconnect('remote', 'local', 'local')).toBe(false);
  expect(isRemoteReconnect('remote', 'remote', 'local')).toBe(false);
  expect(isRemoteReconnect('remote', 'remote2', 'local')).toBe(true);
  expect(isRemoteReconnect('', 'remote', 'local')).toBe(false);
});

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
