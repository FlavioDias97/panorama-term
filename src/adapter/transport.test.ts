import { describe, expect, test } from 'bun:test';
import { createTransport } from './transport';

describe('transport', () => {
  test('local calls invoke directly, remote routes host commands and keeps local commands local', async () => {
    const calls: unknown[][] = [];
    const direct = Promise.resolve('result');
    const invoke = ((...args: unknown[]) => {
      calls.push(args);
      return direct;
    }) as typeof import('@tauri-apps/api/core').invoke;
    let remote = false;
    const call = createTransport(invoke, () => remote);
    expect(call('git_status', { path: '/repo' })).toBe(direct);
    remote = true;
    await call('git_status', { path: '/repo' });
    for (const command of ['plugins_scan', 'log_ui_error', 'open_url', 'has_clipboard_image', 'updater']) await call(command);
    await call('store_read', { name: 'config.json' });
    await call('store_read', { name: 'ui-state.json' });
    await call('store_read', { name: 'workspaces.json' });
    expect(calls[0]).toEqual(['git_status', { path: '/repo' }]);
    expect(calls[1]).toEqual(['remote_invoke', { command: 'git_status', args: { path: '/repo' } }]);
    expect(calls.slice(2, 9).map((c) => c[0])).toEqual([
      'plugins_scan',
      'log_ui_error',
      'open_url',
      'has_clipboard_image',
      'updater',
      'store_read',
      'store_read'
    ]);
    expect(calls[9]).toEqual(['remote_invoke', { command: 'store_read', args: { name: 'workspaces.json' } }]);
  });
  test('converts remote bytes and splits mixed store writes', async () => {
    const calls: unknown[][] = [];
    const invoke = ((...args: unknown[]) => {
      calls.push(args);
      return Promise.resolve([0, 255]);
    }) as typeof import('@tauri-apps/api/core').invoke;
    const call = createTransport(invoke, () => true);
    expect([...new Uint8Array(await call<ArrayBuffer>('read_file_bytes', { path: '/file' }))]).toEqual([0, 255]);
    await call('store_write_many', {
      entries: [
        { name: 'config.json', value: 1 },
        { name: 'workspaces/a', value: 2 }
      ]
    });
    expect(calls[1]).toEqual(['store_write_many', { entries: [{ name: 'config.json', value: 1 }] }]);
    expect(calls[2]).toEqual([
      'remote_invoke',
      { command: 'store_write_many', args: { entries: [{ name: 'workspaces/a', value: 2 }] } }
    ]);
  });
});
