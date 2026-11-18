/// <reference path="./bun-host.d.ts" />
// Bun/Web Platform host boundary. Imported only by automation, never by app code.
if (typeof Bun === 'undefined') throw new Error('The automation SDK requires Bun. Run host tools with bun.');
let worker: (Worker & { unref(): void }) | undefined;
const children = new Set<HostSubprocess>();
const cleanupDirectories = new Set<string>();
export function deferDirectoryCleanup(path: string): void { cleanupDirectories.add(path); }
export function hostCall<T>(op: string, args: unknown[] = [], timeout = 30000): T {
  if (!worker) {
    worker = new Worker(new URL('./host_worker.ts', import.meta.url).href) as Worker & { unref(): void };
    worker.unref();
  }
  const shared = new SharedArrayBuffer(16 + 32 * 1024 * 1024);
  const header = new Int32Array(shared, 0, 4);
  worker.postMessage({ op, args, shared });
  if (Atomics.wait(header, 0, 0, timeout + 1000) === 'timed-out') {
    worker.terminate(); worker = undefined;
    throw new Error(`Host operation ${op} timed out`);
  }
  const response = JSON.parse(new TextDecoder().decode(new Uint8Array(shared, 16, Atomics.load(header, 1))));
  if (response.error) throw new Error(response.error);
  return response.value;
}
export const runtime = {
  env: Bun.env,
  executable: Bun.argv[0],
  platform: navigator.platform.startsWith('Win') ? 'win32' : navigator.platform.startsWith('Mac') ? 'darwin' : 'linux',
  arch: (Bun.env.PROCESSOR_ARCHITECTURE || navigator.userAgent).toLowerCase().includes('arm') ? 'arm64' : 'x64',
  cwd: (): string => hostCall('cwd'),
};
export type HostProcess = HostSubprocess;
export async function nativeCompiler(root = '.'): Promise<string> {
  const platform = runtime.platform === 'win32' ? 'win32-x64'
    : runtime.platform === 'darwin' ? (runCommand('uname', ['-m']).stdout.trim() === 'arm64' ? 'darwin-arm64' : 'darwin-x64')
    : 'linux-x64';
  const binary = runtime.platform === 'win32' ? 'tsc.exe' : 'tsc';
  const path = joinPath(root, `node_modules/@typescript/typescript-${platform}/lib/${binary}`);
  if (!await Bun.file(path).exists()) throw new Error('Native TypeScript is missing. Run bun install with optional dependencies enabled.');
  return path;
}
export function runCommand(command: string, args: string[], options: { encoding?: string; timeout?: number; stdio?: unknown; cwd?: string } = {}) {
  const result = Bun.spawnSync([command, ...args], { cwd: options.cwd, timeout: options.timeout, stdout: 'pipe', stderr: 'pipe', windowsHide: true });
  return { status: result.exitCode, stdout: new TextDecoder().decode(result.stdout), stderr: new TextDecoder().decode(result.stderr) };
}
export function startProcess(command: string, args: string[], _options: unknown = {}): HostProcess {
  const child = Bun.spawn([command, ...args], { stdin: 'ignore', stdout: 'ignore', stderr: 'ignore', windowsHide: true });
  children.add(child);
  void child.exited.finally(() => children.delete(child));
  return child;
}
export function stopProcess(child: HostProcess, actualPid = child.pid): void {
  if (runtime.platform === 'win32') {
    const result = runCommand(`${Bun.env.SystemRoot || 'C:/Windows'}/System32/taskkill.exe`, ['/PID', String(actualPid), '/T', '/F'], { timeout: 10000 });
    if (result.status !== 0 && actualPid !== child.pid) throw new Error(`Browser cleanup failed: ${result.stderr}`);
  }
  child.kill();
}
export function sleep(ms: number): void { Bun.sleepSync(ms); }
export function exists(path: string): boolean { return hostCall('exists', [path]); }
export function readBytes(path: string): Uint8Array { return decodeBase64(hostCall('read', [path])); }
export function writeBytes(path: string, data: string | Uint8Array): void {
  hostCall('write', [path, encodeBase64(typeof data === 'string' ? new TextEncoder().encode(data) : data)]);
}
export function makeDirectory(path: string, _options?: unknown): void { hostCall('mkdir', [path]); }
export function removeDirectory(path: string): void { hostCall('remove', [path]); }
export function fetchText(url: string, timeout = 5000): string { return hostCall('fetch', [url, timeout], timeout); }
export function portAvailable(port: number): boolean { return hostCall('port', [port]); }
export function joinPath(...parts: string[]): string {
  return parts.join('/').replace(/\\/g, '/').replace(/\/{2,}/g, '/');
}
export function resolvePath(...parts: string[]): string {
  const joined = joinPath(...parts);
  const absolute = /^(?:[A-Za-z]:\/|\/)/.test(joined) ? joined : joinPath(runtime.cwd(), joined);
  const prefix = absolute.startsWith('/') ? '/' : '';
  const result: string[] = [];
  for (const part of absolute.split('/')) {
    if (!part || part === '.') continue;
    if (part === '..') { if (result.length && !result[result.length - 1].endsWith(':')) result.pop(); }
    else result.push(part);
  }
  return prefix + result.join('/');
}
export function encodeBase64(bytes: Uint8Array): string {
  let text = '';
  for (let index = 0; index < bytes.length; index += 8192) text += String.fromCharCode(...bytes.subarray(index, index + 8192));
  return btoa(text);
}
export function decodeBase64(value: string): Uint8Array { return Uint8Array.from(atob(value), char => char.charCodeAt(0)); }
export function openOutput(path: string, _mode?: string): number { return hostCall('open', [path]); }
export function writeOutput(handle: number, bytes: Uint8Array, offset: number, length: number, position: number | null): number {
  return hostCall('writeAt', [handle, encodeBase64(bytes.subarray(offset, offset + length)), position]);
}
export function closeOutput(handle: number): void { hostCall('closeFile', [handle]); }
export async function shutdownHost(): Promise<void> {
  for (const child of children) stopProcess(child);
  await Promise.allSettled([...children].map(child => child.exited));
  // Allow Bun's process exit handlers to release Windows handles before cleanup.
  await Bun.sleep(100);
  for (const path of cleanupDirectories) {
    if (runtime.platform === 'win32') {
      // Installed Chrome can reparent its crash reporter outside the initial
      // process tree. Match only our unique profile, never all browser processes.
      const script = `$ErrorActionPreference='Stop'; $target=$env:MWEB_BROWSER_PROFILE_CLEANUP.Replace('\\','/');
        $root=$env:TEMP.Replace('\\','/').TrimEnd('/')+'/';
        if (!$target.StartsWith($root) -or $target.Substring($root.Length) -notmatch '^mweb-browser-[a-f0-9-]+$') { throw 'Invalid owned browser profile' };
        Get-CimInstance Win32_Process -Filter "Name='chrome.exe' OR Name='chrome-headless-shell.exe'" | Where-Object { $_.CommandLine -and $_.CommandLine.Replace('\\','/').Contains($target) } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue };`;
      const result = Bun.spawnSync([`${Bun.env.SystemRoot || 'C:/Windows'}/System32/WindowsPowerShell/v1.0/powershell.exe`, '-NoProfile', '-NonInteractive', '-Command', script], {
        env: { ...Bun.env, MWEB_BROWSER_PROFILE_CLEANUP: path }, stdout: 'pipe', stderr: 'pipe', windowsHide: true, timeout: 45000,
      });
      if (!result.success) throw new Error(`Browser cleanup failed (exit ${result.exitCode}): ${new TextDecoder().decode(result.stderr)}`);
    }
    removeDirectory(path); cleanupDirectories.delete(path);
  }
  if (worker) { hostCall('shutdown'); worker.terminate(); worker = undefined; }
}
