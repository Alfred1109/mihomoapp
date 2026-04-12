import assert from 'node:assert/strict';
import test from 'node:test';

import { isTauriEnv, runInTauri } from '../frontend/utils/tauri.ts';

function withWindow<T>(value: Window | undefined, fn: () => T): T {
  const hadWindow = 'window' in globalThis;
  const previousWindow = hadWindow ? globalThis.window : undefined;

  if (value === undefined) {
    Reflect.deleteProperty(globalThis, 'window');
  } else {
    Object.defineProperty(globalThis, 'window', {
      configurable: true,
      writable: true,
      value,
    });
  }

  try {
    return fn();
  } finally {
    if (hadWindow) {
      Object.defineProperty(globalThis, 'window', {
        configurable: true,
        writable: true,
        value: previousWindow,
      });
    } else {
      Reflect.deleteProperty(globalThis, 'window');
    }
  }
}

test('isTauriEnv returns false when window is missing', () => {
  const result = withWindow(undefined, () => isTauriEnv());
  assert.equal(result, false);
});

test('isTauriEnv returns false when __TAURI_IPC__ is not a function', () => {
  const result = withWindow({ __TAURI_IPC__: {} } as Window, () => isTauriEnv());
  assert.equal(result, false);
});

test('isTauriEnv returns true when __TAURI_IPC__ is available', () => {
  const result = withWindow(
    { __TAURI_IPC__: (() => undefined) as Window['__TAURI_IPC__'] } as Window,
    () => isTauriEnv()
  );
  assert.equal(result, true);
});

test('runInTauri returns null outside Tauri', async () => {
  const result = await withWindow(undefined, () => runInTauri(async () => 'value'));
  assert.equal(result, null);
});

test('runInTauri executes the callback inside Tauri', async () => {
  const result = await withWindow(
    { __TAURI_IPC__: (() => undefined) as Window['__TAURI_IPC__'] } as Window,
    () => runInTauri(async () => 'value')
  );
  assert.equal(result, 'value');
});
