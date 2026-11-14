import { describe, expect, test } from 'bun:test';
import { remoteLabel } from './remote';

describe('remote status labels', () => {
  test('describes each connection state and handles missing names', () => {
    expect(remoteLabel({ status: 'connected', hostName: 'Laptop', remembered: true })).toBe('Connected to Laptop');
    expect(remoteLabel({ status: 'reconnecting', hostName: null, remembered: true })).toBe('Reconnecting to remote host...');
    expect(remoteLabel({ status: 'disconnected', hostName: 'Laptop', remembered: true })).toBe('Disconnected from Laptop');
    expect(remoteLabel({ status: 'disconnected', hostName: null, remembered: false })).toBe('Not connected');
  });
});
