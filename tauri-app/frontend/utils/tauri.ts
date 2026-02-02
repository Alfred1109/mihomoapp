declare global {
  interface Window {
    __TAURI_IPC__?: unknown;
  }
}

export function isTauriEnv(): boolean {
  return typeof window !== 'undefined' && window.__TAURI_IPC__ !== undefined;
}

export function runInTauri<T>(fn: () => Promise<T>): Promise<T | null> {
  if (!isTauriEnv()) {
    return Promise.resolve(null);
  }
  return fn();
}
