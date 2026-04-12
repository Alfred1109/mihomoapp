export function isTauriEnv(): boolean {
  return typeof window !== 'undefined' && typeof window.__TAURI_IPC__ === 'function';
}

export function runInTauri<T>(fn: () => Promise<T>): Promise<T | null> {
  if (!isTauriEnv()) {
    return Promise.resolve(null);
  }
  return fn();
}
