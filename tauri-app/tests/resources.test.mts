import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { promises as fs } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { gzipSync } from 'node:zlib';
import { mihomoAsset, prepareResources, verifySha256 } from '../prepare-resources.mjs';

const version = 'v1.19.20';
const fixture = (reported = 'Mihomo Meta v1.19.20 linux amd64 with_gvisor') =>
  gzipSync(Buffer.from(`#!/bin/sh\nprintf '%s\\n' '${reported}'\n`));
const digest = (data: Buffer) => createHash('sha256').update(data).digest('hex');

async function isolatedResources(run: (dir: string) => Promise<void>) {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'mihomo-resources-test-'));
  try {
    await fs.writeFile(path.join(dir, 'mihomo'), 'existing-core');
    await run(dir);
  } finally {
    await fs.rm(dir, { recursive: true, force: true });
  }
}

function fakeRelease(archive: Buffer, sha256 = digest(archive)) {
  return async (url: string) => url.startsWith('https://api.github.com/')
    ? Buffer.from(JSON.stringify({ assets: [{ name: mihomoAsset('linux', 'x64', version), digest: `sha256:${sha256}` }] }))
    : archive;
}

test('resource preparation rejects unsupported architecture and malformed versions', () => {
  assert.throws(() => mihomoAsset('win32', 'arm64', version), /架构/);
  assert.throws(() => mihomoAsset('linux', 'ia32', version), /架构/);
  assert.throws(() => mihomoAsset('linux', 'x64', '../bad'), /版本/);
});

test('resource checksums require an expected digest and reject corrupt data', () => {
  assert.throws(() => verifySha256(Buffer.from('corrupt'), '0'.repeat(64), 'archive'), /校验失败/);
  assert.throws(() => verifySha256(Buffer.from('corrupt'), undefined, 'archive'), /可信/);
});

test('successful preparation verifies then replaces the previous core', { skip: process.platform === 'win32' }, async () => {
  await isolatedResources(async (dir) => {
    const archive = fixture();
    await prepareResources({ platform: 'linux', arch: 'x64', mihomoVersion: version, resourcesDir: dir, fetchData: fakeRelease(archive) });
    assert.match(await fs.readFile(path.join(dir, 'mihomo'), 'utf8'), /Mihomo Meta/);
    assert.deepEqual(await fs.readdir(dir), ['mihomo']);
  });
});

test('checksum failure preserves previous core and cleans staging files', async () => {
  await isolatedResources(async (dir) => {
    await assert.rejects(prepareResources({ platform: 'linux', arch: 'x64', mihomoVersion: version, resourcesDir: dir, fetchData: fakeRelease(fixture(), '0'.repeat(64)) }), /校验失败/);
    assert.equal(await fs.readFile(path.join(dir, 'mihomo'), 'utf8'), 'existing-core');
    assert.deepEqual(await fs.readdir(dir), ['mihomo']);
  });
});

test('wrong reported version or architecture preserves previous core', { skip: process.platform === 'win32' }, async () => {
  for (const reported of ['Mihomo Meta v1.18.10 linux amd64', 'Mihomo Meta v1.19.20 linux arm64']) {
    await isolatedResources(async (dir) => {
      await assert.rejects(prepareResources({ platform: 'linux', arch: 'x64', mihomoVersion: version, resourcesDir: dir, fetchData: fakeRelease(fixture(reported)) }), /不匹配/);
      assert.equal(await fs.readFile(path.join(dir, 'mihomo'), 'utf8'), 'existing-core');
      assert.deepEqual(await fs.readdir(dir), ['mihomo']);
    });
  }
});

test('missing official checksum or Windows WinSW checksum fails without changing resources', async () => {
  await isolatedResources(async (dir) => {
    await assert.rejects(prepareResources({ platform: 'linux', arch: 'x64', mihomoVersion: version, resourcesDir: dir, fetchData: async () => Buffer.from('{"assets":[]}') }), /官方发布未提供/);
    await assert.rejects(prepareResources({ platform: 'win32', arch: 'x64', resourcesDir: dir }), /WinswSha256/);
    assert.deepEqual(await fs.readdir(dir), ['mihomo']);
  });
});

test('explicit WinSW version requires its trusted checksum even when an old binary exists', async () => {
  await isolatedResources(async (dir) => {
    await fs.writeFile(path.join(dir, 'winsw.exe'), 'existing-winsw');
    await assert.rejects(prepareResources({
      platform: 'win32', arch: 'x64', resourcesDir: dir, winswVersion: 'v2.12.0',
      fetchData: async () => { throw new Error('must not download before preflight validation'); },
    }), /显式选择 WinSW 版本.*WinswSha256/);
    assert.equal(await fs.readFile(path.join(dir, 'winsw.exe'), 'utf8'), 'existing-winsw');
    assert.equal(await fs.readFile(path.join(dir, 'mihomo'), 'utf8'), 'existing-core');
    assert.deepEqual((await fs.readdir(dir)).sort(), ['mihomo', 'winsw.exe']);
  });
});
