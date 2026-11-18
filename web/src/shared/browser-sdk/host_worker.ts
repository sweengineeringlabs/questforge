/// <reference path="./bun-host.d.ts" />
// A worker owns asynchronous file/network operations while the existing SDK
// remains synchronous. No Node modules, compatibility globals, or bridge process.
import { $ } from 'bun';

type Connection = {
  socket: WebSocket;
  nextId: number;
  events: unknown[];
  overflow: boolean;
  pending: Map<number, { resolve(value: string): void; reject(error: Error): void }>;
};
const sockets = new Map<number, Connection>();
type OutputFile = { path: string; writer?: ReturnType<BunHostFile['writer']> };
const files = new Map<number, OutputFile>();
let nextHandle = 1;

function bytesToBase64(bytes: Uint8Array): string {
  let text = '';
  for (let offset = 0; offset < bytes.length; offset += 8192) {
    text += String.fromCharCode(...bytes.subarray(offset, offset + 8192));
  }
  return btoa(text);
}

async function connect(url: string, timeout: number): Promise<number> {
  const handle = nextHandle++;
  const socket = new WebSocket(url);
  const connection: Connection = { socket, nextId: 1, events: [], overflow: false, pending: new Map() };
  socket.addEventListener('message', event => {
    try {
      const message = JSON.parse(String(event.data));
      if (typeof message.id === 'number') {
        connection.pending.get(message.id)?.resolve(String(event.data));
      } else if (connection.events.length < 10000) {
        connection.events.push(message);
      } else {
        connection.overflow = true;
      }
    } catch (error) {
      for (const pending of connection.pending.values()) pending.reject(new Error(`Invalid CDP response: ${error}`));
    }
  });
  socket.addEventListener('close', () => {
    for (const pending of connection.pending.values()) pending.reject(new Error('CDP connection closed'));
  });
  await new Promise<void>((resolve, reject) => {
    const timer = setTimeout(() => { socket.close(); reject(new Error(`WebSocket connect timed out: ${url}`)); }, timeout);
    socket.addEventListener('open', () => { clearTimeout(timer); resolve(); }, { once: true });
    socket.addEventListener('error', () => { clearTimeout(timer); reject(new Error(`WebSocket connection failed: ${url}`)); }, { once: true });
  });
  sockets.set(handle, connection);
  return handle;
}

async function dispatch(op: string, args: any[]): Promise<unknown> {
  switch (op) {
    case 'cwd': return (await $`pwd`.quiet().text()).trim();
    case 'exists': return Bun.file(args[0]).exists();
    case 'read': return bytesToBase64(new Uint8Array(await Bun.file(args[0]).arrayBuffer()));
    case 'write': return Bun.write(args[0], Uint8Array.from(atob(args[1]), char => char.charCodeAt(0)));
    case 'mkdir': await $`mkdir -p ${args[0]}`.quiet(); return null;
    case 'remove': {
      const root = (Bun.env.TEMP || Bun.env.TMPDIR || '/tmp').replace(/\\/g, '/').replace(/\/$/, '') + '/';
      const target = String(args[0]).replace(/\\/g, '/');
      if (!target.startsWith(root) || !/^mweb-browser-[a-f0-9-]+$/.test(target.slice(root.length))) {
        throw new Error('Refusing to remove a directory outside the owned browser profile');
      }
      let failure = '';
      for (let attempt = 0; attempt < 100; attempt++) {
        const result = await $`rm -rf ${target}`.quiet().nothrow();
        if (result.exitCode === 0) return null;
        failure = new TextDecoder().decode(result.stderr);
        await Bun.sleep(100);
      }
      throw new Error(`Could not clean browser profile: ${target}: ${failure}`);
    }
    case 'fetch': {
      const response = await fetch(args[0], { signal: AbortSignal.timeout(args[1]) });
      if (!response.ok) throw new Error(`HTTP ${response.status}: ${args[0]}`);
      return response.text();
    }
    case 'port': {
      try {
        const server = Bun.serve({ hostname: '127.0.0.1', port: args[0], fetch: () => new Response('') });
        server.stop(true);
        return true;
      } catch { return false; }
    }
    case 'open': {
      const handle = nextHandle++;
      await Bun.write(args[0], new Uint8Array());
      files.set(handle, { path: args[0], writer: Bun.file(args[0]).writer() });
      return handle;
    }
    case 'writeAt': {
      const file = files.get(args[0]);
      if (!file) throw new Error('File is closed');
      const bytes = Uint8Array.from(atob(args[1]), char => char.charCodeAt(0));
      if (args[2] === null) {
        if (!file.writer) throw new Error('Cannot append after finalizing an output file');
        file.writer.write(bytes);
      } else {
        await file.writer?.end(); file.writer = undefined;
        const original = new Uint8Array(await Bun.file(file.path).arrayBuffer());
        const output = new Uint8Array(Math.max(original.length, args[2] + bytes.length));
        output.set(original); output.set(bytes, args[2]);
        await Bun.write(file.path, output);
      }
      return bytes.length;
    }
    case 'closeFile': await files.get(args[0])?.writer?.end(); files.delete(args[0]); return null;
    case 'connect': return connect(args[0], args[1]);
    case 'browserPid': {
      const version = await (await fetch(`http://127.0.0.1:${args[0]}/json/version`, { signal: AbortSignal.timeout(3000) })).json();
      const handle = await connect(version.webSocketDebuggerUrl, 3000);
      try {
        const commandLine = JSON.parse(await dispatch('send', [handle, JSON.stringify({ method: 'Browser.getBrowserCommandLine' }), 3000]) as string);
        const expected = `--user-data-dir=${args[1]}`.replace(/\\/g, '/');
        if (!commandLine.result?.arguments?.some((value: string) => value.replace(/\\/g, '/') === expected)) {
          throw new Error('Browser process ownership could not be verified');
        }
        const info = JSON.parse(await dispatch('send', [handle, JSON.stringify({ method: 'SystemInfo.getProcessInfo' }), 3000]) as string);
        const browser = info.result?.processInfo?.find((value: { type: string }) => value.type === 'browser');
        if (!browser) throw new Error('Browser PID unavailable');
        return browser.id;
      } finally {
        sockets.get(handle)?.socket.close(); sockets.delete(handle);
      }
    }
    case 'send': {
      const connection = sockets.get(args[0]);
      if (!connection) throw new Error('CDP socket is closed');
      const command = JSON.parse(args[1]);
      command.id = connection.nextId++;
      return new Promise<string>((resolve, reject) => {
        const timer = setTimeout(() => {
          connection.pending.delete(command.id);
          reject(new Error(`CDP ${command.method} timed out`));
        }, args[2]);
        connection.pending.set(command.id, {
          resolve(value) { clearTimeout(timer); connection.pending.delete(command.id); resolve(value); },
          reject(error) { clearTimeout(timer); connection.pending.delete(command.id); reject(error); },
        });
        try { connection.socket.send(JSON.stringify(command)); }
        catch (error) { connection.pending.get(command.id)?.reject(new Error(String(error))); }
      });
    }
    case 'events': {
      const connection = sockets.get(args[0]);
      if (!connection) throw new Error('CDP socket is closed');
      if (connection.overflow) throw new Error('CDP event queue overflow; consume events more frequently');
      const result = JSON.stringify(connection.events);
      connection.events = [];
      return result;
    }
    case 'close': sockets.get(args[0])?.socket.close(); sockets.delete(args[0]); return null;
    case 'shutdown':
      for (const connection of sockets.values()) connection.socket.close();
      for (const file of files.values()) await file.writer?.end();
      sockets.clear(); files.clear(); return null;
    default: throw new Error(`Unknown host operation: ${op}`);
  }
}

// Install before any await: Bun workers do not buffer messages before a listener.
globalThis.addEventListener('message', async (event: MessageEvent) => {
  const { op, args, shared } = event.data;
  const header = new Int32Array(shared, 0, 4);
  const body = new Uint8Array(shared, 16);
  let response: Uint8Array;
  try { response = new TextEncoder().encode(JSON.stringify({ value: await dispatch(op, args) })); }
  catch (error) { response = new TextEncoder().encode(JSON.stringify({ error: String(error) })); }
  if (response.length > body.length) response = new TextEncoder().encode(JSON.stringify({ error: 'Host response exceeds 32 MiB limit' }));
  body.set(response);
  Atomics.store(header, 1, response.length);
  Atomics.store(header, 0, 1);
  Atomics.notify(header, 0);
});
