import type { RemoteStatus } from '~/adapter/remote/remote.client';

export const remoteLabel = (status: RemoteStatus): string => {
  if (status.status === 'reconnecting') return `Reconnecting to ${status.hostName || 'remote host'}...`;
  if (status.status === 'connected') return `Connected to ${status.hostName || 'remote host'}`;
  return status.remembered ? `Disconnected from ${status.hostName || 'remote host'}` : 'Not connected';
};
