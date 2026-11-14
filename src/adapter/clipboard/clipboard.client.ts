import { invoke } from '@tauri-apps/api/core';
import { invoke as hostInvoke, isRemote } from '~/adapter/transport';

export const writeClipboard = (text: string): void => {
  navigator.clipboard?.writeText(text).catch(() => {});
};

export const readClipboard = (): Promise<string> => {
  if (!navigator.clipboard?.readText) return Promise.resolve('');
  return navigator.clipboard.readText().catch(() => '');
};

export const hasClipboardImage = async (): Promise<boolean> => {
  if (navigator.clipboard?.read) {
    try {
      const items = await navigator.clipboard.read();
      if (items.some((item) => item.types.some((t) => t.startsWith('image/')))) return true;
    } catch {
      void 0;
    }
  }
  return invoke<boolean>('has_clipboard_image').catch(() => false);
};

const MIME_EXT: Record<string, string> = {
  'image/png': 'png',
  'image/jpeg': 'jpg',
  'image/gif': 'gif',
  'image/webp': 'webp',
  'image/bmp': 'bmp',
  'image/avif': 'avif'
};

export const writeTempImage = async (blob: Blob, token: string): Promise<string> => {
  if (blob.size > 5 * 1024 * 1024) throw new Error('image too large (>5MB)');
  const ext = MIME_EXT[blob.type];
  if (!ext) throw new Error(`unsupported image type: ${blob.type}`);
  const buf = await blob.arrayBuffer();
  const name = `paste_${token}.${ext}`;
  if (!isRemote()) return invoke<string>('write_temp_image', buf, { headers: { 'x-image-name': name } });
  const bytes = new Uint8Array(buf);
  let path = '';
  for (let offset = 0; offset < bytes.length || offset === 0; offset += 32768) {
    path = await hostInvoke<string>('upload_image_chunk', {
      name,
      bytes: Array.from(bytes.subarray(offset, offset + 32768)),
      first: offset === 0,
      last: offset + 32768 >= bytes.length
    });
  }
  return path;
};

export const readTempImage = async (path: string): Promise<string> => {
  const buf = await hostInvoke<ArrayBuffer>('read_temp_image', { path });
  return URL.createObjectURL(new Blob([buf]));
};
