// Server utilities — ported from Rust server.rs.
// Port checks use Bun.serve through the host worker.

import { portAvailable } from "./host";
import { runCommand, sleep } from "./host";

// ---------------------------------------------------------------------------
// Port checking
// ---------------------------------------------------------------------------

export function isPortAvailable(port: number): boolean {
  return !isServerRunning(port);
}

export function isServerRunning(port: number): boolean {
  return !portAvailable(port);
}

// ---------------------------------------------------------------------------
// Port finding
// ---------------------------------------------------------------------------

export function findAvailablePort(startPort: number, maxAttempts: number): number {
  let offset: number = 0;
  while (offset < maxAttempts) {
    let port: number = startPort + offset;
    if (isPortAvailable(port)) return port;
    offset = offset + 1;
  }
  return 0;
}

// ---------------------------------------------------------------------------
// URL port extraction
// ---------------------------------------------------------------------------

export function extractPortFromUrl(url: string): number {
  let isHttps: boolean = url.indexOf("https://") === 0;
  let stripped: string = url;
  if (stripped.indexOf("http://") === 0) stripped = stripped.substring(7);
  else if (stripped.indexOf("https://") === 0) stripped = stripped.substring(8);

  let slashIdx: number = stripped.indexOf("/");
  let hostPort: string = slashIdx >= 0 ? stripped.substring(0, slashIdx) : stripped;

  let colonIdx: number = hostPort.indexOf(":");
  if (colonIdx >= 0) {
    let portStr: string = hostPort.substring(colonIdx + 1);
    let port: number = parseInt(portStr, 10);
    if (!isNaN(port)) return port;
  }

  return isHttps ? 443 : 80;
}
