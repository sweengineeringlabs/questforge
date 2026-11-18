/// <reference path="../src/shared/browser-sdk/bun-host.d.ts" />
// Project tooling runs under Bun. Browser source is checked separately.
import { closeAllBrowsers } from '../src/shared/browser-sdk/browser';
import { shutdownHost, nativeCompiler } from '../src/shared/browser-sdk/host';
const publicDirectory: string = "public";

async function compiler(): Promise<string> {
  return nativeCompiler();
}

function runCompiler(executable: string, config: string, extra: string[] = []): string {
  const result = Bun.spawnSync([executable, '--project', config, '--noEmit', ...extra], { stdout: 'pipe', stderr: 'pipe' });
  const stdout = new TextDecoder().decode(result.stdout);
  const stderr = new TextDecoder().decode(result.stderr);
  if (!result.success) throw new Error(`${stdout}${stderr}`);
  return stdout;
}

async function typecheck(): Promise<void> {
  const executable = await compiler();
  const configs: string[] = [];
  for (const file of ['machineweb.toml', 'justweb.toml']) if (await Bun.file(file).exists()) configs.push(file);
  if (configs.length > 1) throw new Error('Both canonical and legacy project configs exist; resolve the ambiguity first.');
  if (configs.length) {
    const config = Bun.TOML.parse(await Bun.file(configs[0]).text()) as { toolchain?: { typescript?: string; bun?: string } };
    const pin = config.toolchain?.typescript;
    if (pin) {
      const version = Bun.spawnSync([executable, '--version'], { stdout: 'pipe', stderr: 'pipe' });
      if (!version.success || new TextDecoder().decode(version.stdout).trim() !== `Version ${pin}`) {
        throw new Error(`TypeScript pin ${pin} does not match the installed native compiler.`);
      }
    }
    if (config.toolchain?.bun && config.toolchain.bun !== Bun.version) throw new Error(`Bun pin ${config.toolchain.bun} does not match ${Bun.version}.`);
  }
  const browserFiles = runCompiler(executable, 'tsconfig.json', ['--listFilesOnly']).split(/\r?\n/);
  for (const file of browserFiles) {
    const normalized = file.replace(/\\/g, '/');
    if (/\/(?:browser-sdk|tooling)\//.test(normalized)
        || /\/node_modules\/(?:@types\/node|bun-types|@types\/bun)\//.test(normalized)) {
      throw new Error(`Browser source imports host-only code or declarations: ${file}`);
    }
  }
  runCompiler(executable, 'tsconfig.json');
  runCompiler(executable, 'tsconfig.tools.json');
  console.log('Browser and host TypeScript checks passed.');
}

async function copyPublicFiles(): Promise<void> {
  // Glob scanning is a Bun API; optional public roots may not exist yet.
  const glob = new Bun.Glob('**/*');
  try {
    for await (const file of glob.scan({ cwd: publicDirectory, onlyFiles: true })) {
      await Bun.write(`dist/${file}`, Bun.file(`${publicDirectory}/${file}`));
    }
  } catch (error) {
    if ((error as { code?: string }).code !== 'ENOENT') throw error;
  }
}

async function build(): Promise<void> {
  await typecheck();
  await copyPublicFiles();
  const result = await Bun.build({ entrypoints: ['./index.html'], outdir: './dist', target: 'browser', minify: true });
  if (!result.success) throw new Error(result.logs.map(String).join('\n'));
  console.log('Built dist/index.html');
}

async function dev(): Promise<void> {
  const { default: homepage } = await import('../index.html');
  const routes: Record<string, unknown> = { '/': homepage, '/*': homepage };
  try {
    for await (const file of new Bun.Glob('**/*').scan({ cwd: publicDirectory, onlyFiles: true })) {
      routes[`/${file.replace(/\\/g, '/')}`] = () => new Response(Bun.file(`${publicDirectory}/${file}`));
    }
  } catch (error) {
    if ((error as { code?: string }).code !== 'ENOENT') throw error;
  }
  const server = Bun.serve({ hostname: '127.0.0.1', port: Number(Bun.env.PORT || 8090), routes, development: { hmr: Bun.env.MWEB_DEV_HMR !== '0', console: true } });
  console.log(`Dev: http://127.0.0.1:${server.port}`);
}

async function preview(): Promise<void> {
  if (!await Bun.file('dist/index.html').exists()) throw new Error('Run bun run build before preview.');
  const server = Bun.serve({
    hostname: '127.0.0.1', port: Number(Bun.env.PORT || 8090),
    async fetch(request) {
      const path = decodeURIComponent(new URL(request.url).pathname);
      if (path.includes('..') || path.includes('\\') || path.includes('\0')) return new Response('Invalid path', { status: 400 });
      const file = Bun.file(`dist${path === '/' ? '/index.html' : path}`);
      if (await file.exists()) return new Response(file);
      if (path.split('/').pop()?.includes('.')) return new Response('Not found', { status: 404 });
      return new Response(Bun.file('dist/index.html'));
    },
  });
  console.log(`Preview: http://127.0.0.1:${server.port}`);
}

async function test(): Promise<void> {
  try {
    // Native component integration tests inspect class exports, not DOM behavior.
    // Install their declared minimal harness before ESM dependencies are loaded.
    if (typeof (globalThis as { HTMLElement?: unknown }).HTMLElement === 'undefined') {
      (globalThis as { HTMLElement?: unknown }).HTMLElement = class {};
    }
    const requested = Bun.argv.slice(3);
    const tests = requested.length ? requested : [...new Bun.Glob('**/*_int_test.gen.ts').scanSync({ cwd: '.', onlyFiles: true })];
    if (!tests.length) throw new Error('No generated integration tests found. Run mweb generate app first.');
    for (const file of tests) {
      if (file.replace(/\\/g, '/').split('/').includes('node_modules')) continue;
      await import(new URL(`../${file}`, import.meta.url).href);
    }
  } finally {
    try { closeAllBrowsers(); }
    finally { await shutdownHost(); }
  }
}

switch (Bun.argv[2]) {
  case 'dev': await dev(); break;
  case 'typecheck': await typecheck(); break;
  case 'build': await build(); break;
  case 'preview': await preview(); break;
  case 'test': await test(); break;
  default: throw new Error('Expected dev, typecheck, build, preview, or test.');
}
