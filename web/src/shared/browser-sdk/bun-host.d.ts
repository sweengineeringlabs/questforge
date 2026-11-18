// The first-party host contract uses Bun and Web APIs only. Deliberately does
// not pull @types/bun (and its transitive Node globals) into browser projects.
interface ImportMeta { readonly dir: string; readonly path: string; }
interface HostSubprocess {
  readonly pid: number;
  readonly exited: Promise<number>;
  kill(signal?: number): void;
}
interface BunHostFile extends Blob {
  slice(begin?: number, end?: number, contentType?: string): BunHostFile;
  exists(): Promise<boolean>;
  delete(): Promise<void>;
  writer(): { write(bytes: Uint8Array): number; flush(): number | Promise<number>; end(): number | Promise<number> };
}
interface BunShellResult { stdout: Uint8Array; stderr: Uint8Array; exitCode: number; }
interface BunShellCommand extends Promise<BunShellResult> {
  quiet(): BunShellCommand;
  nothrow(): BunShellCommand;
  text(): Promise<string>;
}
interface BunRuntime {
  readonly argv: string[];
  readonly main: string;
  readonly version: string;
  readonly env: Record<string, string | undefined>;
  TOML: { parse(contents: string): unknown };
  Glob: new (pattern: string) => {
    scan(options: { cwd: string; onlyFiles: boolean }): AsyncIterable<string>;
    scanSync(options: { cwd: string; onlyFiles: boolean }): Iterable<string>;
  };
  file(path: string | URL): BunHostFile;
  write(path: string | BunHostFile, data: string | Uint8Array | Blob): Promise<number>;
  which(command: string): string | null;
  sleep(ms: number): Promise<void>;
  sleepSync(ms: number): void;
  spawn(command: string[], options?: { cwd?: string; stdin?: string; stdout?: string; stderr?: string; windowsHide?: boolean }): HostSubprocess;
  spawnSync(command: string[], options?: { cwd?: string; timeout?: number; stdin?: string; stdout?: string; stderr?: string; windowsHide?: boolean; env?: Record<string, string | undefined> }): { exitCode: number; stdout: Uint8Array; stderr: Uint8Array; success: boolean };
  build(options: { entrypoints: string[]; outdir: string; target?: string; minify?: boolean }): Promise<{ success: boolean; logs: unknown[]; outputs: Blob[] }>;
  serve(options: { port?: number; hostname?: string; development?: boolean | { hmr: boolean; console: boolean }; routes?: Record<string, unknown>; fetch?: (request: Request) => Response | Promise<Response> }): { port: number; stop(closeActiveConnections?: boolean): void };
}
declare var Bun: BunRuntime;
declare module "bun" {
  export function $(strings: TemplateStringsArray, ...values: unknown[]): BunShellCommand;
}
