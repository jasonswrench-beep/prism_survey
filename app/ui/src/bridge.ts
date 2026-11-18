// The one door to the engine: every command goes through `call` (app/src-tauri/src/main.rs, crates/prismsurvey-api).
// Inside the desktop window it is a Tauri command. In a plain browser (`npm run dev` beside `prismsurvey serve`, for
// trying the screens) it is the development bridge, and the file pickers become typed paths.

import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';

export interface ApiError {
  code: string;
  message: string;
}

export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return invoke<T>('call', { cmd, args: args ?? null });
  const r = await fetch('/rpc', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ cmd, args: args ?? null }) });
  const body = (await r.json()) as { ok: boolean; result?: T; error?: ApiError };
  if (!body.ok) throw body.error ?? { code: 'bridge', message: 'the development bridge did not answer' };
  return body.result as T;
}

export function errorText(e: unknown): string {
  if (e && typeof e === 'object' && 'message' in e) return String((e as ApiError).message);
  return String(e);
}

export async function pickFile(title: string, extensions: string[], label: string): Promise<string | null> {
  if (!inTauri) return window.prompt(`${title} (${extensions.join(', ')}): type the full path`);
  const r = await open({ title, multiple: false, directory: false, filters: [{ name: label, extensions }] });
  return typeof r === 'string' ? r : null;
}

export async function pickFolder(title: string): Promise<string | null> {
  if (!inTauri) return window.prompt(`${title}: type the full path`);
  const r = await open({ title, multiple: false, directory: true });
  return typeof r === 'string' ? r : null;
}

export async function pickSave(title: string, defaultPath: string, extensions: string[], label: string): Promise<string | null> {
  if (!inTauri) return window.prompt(`${title}: type the full path`, defaultPath);
  const r = await save({ title, defaultPath, filters: [{ name: label, extensions }] });
  return r ?? null;
}
