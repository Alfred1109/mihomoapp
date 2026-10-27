import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { promises as fs } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { gunzipSync } from 'node:zlib';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const versions = JSON.parse(await fs.readFile(path.join(scriptDir, 'resource-versions.json'), 'utf8'));

export async function download(url) {
  const response = await fetch(url, { signal: AbortSignal.timeout(120_000) });
  if (!response.ok) throw new Error(`下载失败: HTTP ${response.status} (${url})`);
  return Buffer.from(await response.arrayBuffer());
}

export function verifySha256(data, expected, name) {
  if (!/^[a-f0-9]{64}$/i.test(expected ?? '')) throw new Error(`${name} 缺少可信 SHA256`);
  if (createHash('sha256').update(data).digest('hex') !== expected.toLowerCase()) {
    throw new Error(`${name} SHA256 校验失败`);
  }
}

export function mihomoAsset(platform, arch, version) {
  if (!/^v\d+\.\d+\.\d+$/.test(version)) throw new Error('Mihomo 版本必须为 vX.Y.Z');
  const targetArch = { x64: 'amd64', arm64: 'arm64' }[arch];
  if (!targetArch || !['linux', 'darwin', 'win32'].includes(platform) || (platform === 'win32' && arch !== 'x64')) {
    throw new Error(`资源不支持当前平台/架构: ${platform}/${arch}`);
  }
  return platform === 'win32'
    ? `mihomo-windows-amd64-compatible-${version}.zip`
    : `mihomo-${platform}-${targetArch}-${version}.gz`;
}

async function trustedMihomoDigest(asset, version, fetchData) {
  if (version === versions.mihomo) return versions.sha256[asset];
  // A deliberately selected release may be used only if GitHub publishes its digest.
  const release = JSON.parse((await fetchData(`https://api.github.com/repos/MetaCubeX/mihomo/releases/tags/${version}`)).toString());
  const digest = release.assets?.find((entry) => entry.name === asset)?.digest;
  if (!/^sha256:[a-f0-9]{64}$/i.test(digest ?? '')) throw new Error(`${asset} 官方发布未提供 SHA256，拒绝覆盖资源`);
  return digest.slice(7);
}

async function fileExists(file) {
  try { await fs.access(file); return true; } catch (error) { if (error.code === 'ENOENT') return false; throw error; }
}

// Stage every requested artifact before replacing any resource. Restore prior files if commit fails.
async function commitResources(staged, resourcesDir, stagingDir) {
  const committed = [];
  try {
    for (const name of staged) {
      const target = path.join(resourcesDir, name);
      const backup = path.join(stagingDir, `${name}.previous`);
      const existed = await fileExists(target);
      if (existed) await fs.copyFile(target, backup);
      await fs.rename(path.join(stagingDir, name), target);
      committed.push({ target, backup, existed });
    }
  } catch (error) {
    for (const entry of committed.reverse()) {
      if (entry.existed) await fs.copyFile(entry.backup, entry.target);
      else await fs.unlink(entry.target);
    }
    throw error;
  }
}

export async function prepareResources({
  platform = process.platform, arch = process.arch,
  mihomoVersion = versions.mihomo, winswVersion,
  winswSha256, resourcesDir = path.join(scriptDir, 'backend', 'resources'),
  fetchData = download,
} = {}) {
  const asset = mihomoAsset(platform, arch, mihomoVersion);
  if (winswVersion && !winswSha256) throw new Error('显式选择 WinSW 版本需要预先确认的 -WinswSha256');
  winswVersion ??= versions.winsw;
  if (!/^v\d+\.\d+\.\d+$/.test(winswVersion)) throw new Error('WinSW 版本必须为 vX.Y.Z');
  const prepareWinsw = platform === 'win32' && Boolean(winswSha256);
  if (platform === 'win32' && !prepareWinsw && !await fileExists(path.join(resourcesDir, 'winsw.exe'))) {
    throw new Error('WinSW 官方发布未提供 SHA256；首次下载请提供预先确认的 -WinswSha256');
  }
  if (winswSha256 && !/^[a-f0-9]{64}$/i.test(winswSha256)) throw new Error('WinSW SHA256 格式错误');
  await fs.mkdir(resourcesDir, { recursive: true });
  const stagingDir = await fs.mkdtemp(path.join(resourcesDir, '.prepare-'));
  try {
    const digest = await trustedMihomoDigest(asset, mihomoVersion, fetchData);
    const archive = await fetchData(`https://github.com/MetaCubeX/mihomo/releases/download/${mihomoVersion}/${asset}`);
    verifySha256(archive, digest, asset);
    const binaryName = platform === 'win32' ? 'mihomo.exe' : 'mihomo';
    const binary = path.join(stagingDir, binaryName);
    if (platform === 'win32') {
      const zip = path.join(stagingDir, 'mihomo.zip');
      await fs.writeFile(zip, archive);
      const quote = (value) => `'${value.replaceAll("'", "''")}'`;
      execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command',
        `$ErrorActionPreference='Stop'; Expand-Archive -LiteralPath ${quote(zip)} -DestinationPath ${quote(path.join(stagingDir, 'unpacked'))}`]);
      const files = (await fs.readdir(path.join(stagingDir, 'unpacked'))).filter((name) => name.toLowerCase().endsWith('.exe'));
      if (files.length !== 1) throw new Error('Mihomo ZIP 必须包含一个可执行文件');
      await fs.rename(path.join(stagingDir, 'unpacked', files[0]), binary);
    } else {
      await fs.writeFile(binary, gunzipSync(archive), { mode: 0o755 });
    }
    const reported = execFileSync(binary, ['-v'], { timeout: 15_000, encoding: 'utf8' });
    const versionMatch = reported.match(/\bMihomo Meta (v\d+\.\d+\.\d+) (linux|darwin|windows) (amd64|arm64)\b/);
    const targetOS = platform === 'win32' ? 'windows' : platform;
    const targetArch = arch === 'x64' ? 'amd64' : 'arm64';
    if (!versionMatch || versionMatch[1] !== mihomoVersion || versionMatch[2] !== targetOS || versionMatch[3] !== targetArch) {
      throw new Error('Mihomo 可执行文件报告的版本/平台/架构与请求不匹配');
    }
    const staged = [binaryName];
    if (prepareWinsw) {
      const winsw = await fetchData(`https://github.com/winsw/winsw/releases/download/${winswVersion}/WinSW-x64.exe`);
      verifySha256(winsw, winswSha256, 'WinSW-x64.exe');
      await fs.writeFile(path.join(stagingDir, 'winsw.exe'), winsw);
      staged.push('winsw.exe');
    }
    await commitResources(staged, resourcesDir, stagingDir);
    console.log(`✓ Mihomo ${mihomoVersion} (${platform}/${arch}) 已校验并准备完成`);
    if (platform === 'win32' && !prepareWinsw) console.log('保留已有 WinSW；本次未验证其来源或完整性。更新请提供 -WinswSha256。');
  } finally {
    await fs.rm(stagingDir, { recursive: true, force: true });
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  const options = {};
  const names = { '--mihomo-version': 'mihomoVersion', '--winsw-version': 'winswVersion', '--winsw-sha256': 'winswSha256' };
  try {
    while (args.length) {
      const key = args.shift();
      if (!names[key] || !args.length) throw new Error(`未知或缺少参数: ${key}`);
      options[names[key]] = args.shift();
    }
    await prepareResources(options);
  } catch (error) {
    console.error(`✗ 资源准备失败: ${error.message}`);
    process.exitCode = 1;
  }
}
