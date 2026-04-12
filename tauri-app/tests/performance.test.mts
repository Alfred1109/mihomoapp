import assert from 'node:assert/strict';
import test from 'node:test';
import { setTimeout as sleep } from 'node:timers/promises';

import {
  batchProcess,
  debounce,
  defer,
  lazyLoad,
  proxyCache,
  throttle,
} from '../frontend/utils/performance.ts';

test('debounce only runs the last invocation', async () => {
  const calls: number[] = [];
  const debounced = debounce((value: number) => {
    calls.push(value);
  }, 20);

  debounced(1);
  debounced(2);
  debounced(3);

  await sleep(50);

  assert.deepEqual(calls, [3]);
});

test('throttle limits calls within the interval', async () => {
  const calls: number[] = [];
  const throttled = throttle((value: number) => {
    calls.push(value);
  }, 30);

  throttled(1);
  throttled(2);
  await sleep(40);
  throttled(3);

  assert.deepEqual(calls, [1, 3]);
});

test('proxyCache stores, reads and clears values', () => {
  proxyCache.clear();

  const payload = { proxies: { PROXY: { name: 'PROXY', type: 'Selector', all: [] } } };
  proxyCache.set('proxies', payload);

  assert.deepEqual(proxyCache.get('proxies'), payload);
  assert.equal(proxyCache.has('proxies'), true);

  proxyCache.clear();

  assert.equal(proxyCache.get('proxies'), null);
  assert.equal(proxyCache.has('proxies'), false);
});

test('defer schedules work asynchronously', async () => {
  let value = 0;

  defer(() => {
    value = 1;
  }, 10);

  assert.equal(value, 0);
  await sleep(30);
  assert.equal(value, 1);
});

test('batchProcess preserves order across batches', async () => {
  const result = await batchProcess(
    [1, 2, 3, 4, 5],
    async (item) => item * 2,
    2
  );

  assert.deepEqual(result, [2, 4, 6, 8, 10]);
});

test('lazyLoad resolves loader result after delay', async () => {
  const startedAt = Date.now();
  const result = await lazyLoad(async () => 'ok', 20);

  assert.equal(result, 'ok');
  assert.ok(Date.now() - startedAt >= 15);
});
