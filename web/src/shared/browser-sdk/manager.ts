// Browser manager — ported from Rust manager.rs.
// Bun host APIs handle Chrome discovery and launch.

import { exists, runtime, makeDirectory, deferDirectoryCleanup, stopProcess, hostCall } from "./host";
import { joinPath, resolvePath } from "./host";
import { HostProcess, startProcess, runCommand, fetchText } from "./host";
import { Viewport, viewportDefault } from "./types";
import { currentPlatform, OS_WINDOWS, OS_MACOS } from "./platform";
import { sleepMs } from "./cdp_client";

// ---------------------------------------------------------------------------
// Browser type constants
// ---------------------------------------------------------------------------

export let BROWSER_CHROME: string = "chrome";

// ---------------------------------------------------------------------------
// LaunchConfig
// ---------------------------------------------------------------------------

export class LaunchConfig {
  browserType: string;
  headless: boolean;
  viewport: Viewport;
  timeout: number;
  slowMo: number;
  args: string[];
  executablePath: string;

  constructor() {
    this.browserType = BROWSER_CHROME;
    this.headless = true;
    this.viewport = viewportDefault();
    this.timeout = 30000;
    this.slowMo = 0;
    this.args = [];
    this.executablePath = "";
  }
}

export function defaultLaunchConfig(): LaunchConfig {
  return new LaunchConfig();
}

// ---------------------------------------------------------------------------
// BrowserProcess
// ---------------------------------------------------------------------------

export class BrowserProcess {
  process: HostProcess;
  port: number;
  profile: string;

  constructor(process: HostProcess, port: number, profile = "") {
    this.process = process;
    this.port = port;
    this.profile = profile;
  }

  kill(): void {
    let actualPid = this.process.pid;
    try { actualPid = hostCall<number>('browserPid', [this.port, this.profile]); }
    catch { /* The browser may already have exited. */ }
    try {
      stopProcess(this.process, actualPid);
    } catch (_e) {
      // ignore
    }
    if (this.profile) {
      deferDirectoryCleanup(this.profile);
      this.profile = "";
    }
  }
}

// ---------------------------------------------------------------------------
// Chrome discovery
// ---------------------------------------------------------------------------

export function findChromeBinary(): string {
  // Check CHROME_PATH env var first
  let envPath: string = runtime.env["CHROME_PATH"] || "";
  if (envPath !== "") {
    if (!exists(envPath)) throw new Error("CHROME_PATH does not name an existing browser: " + envPath);
    return envPath;
  }

  let platform = currentPlatform();
  let candidates: string[] = [];

  if (platform.os === OS_WINDOWS) {
    candidates = [
      "C:/Program Files/Google/Chrome/Application/chrome.exe",
      "C:/Program Files (x86)/Google/Chrome/Application/chrome.exe",
    ];
    let localAppData: string = runtime.env["LOCALAPPDATA"] || "";
    if (localAppData !== "") {
      candidates.push(joinPath(localAppData, "Google", "Chrome", "Application", "chrome.exe"));
    }
  } else if (platform.os === OS_MACOS) {
    candidates = [
      "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
      "/Applications/Chromium.app/Contents/MacOS/Chromium",
    ];
  } else {
    candidates = [
      "google-chrome-stable",
      "google-chrome",
      "chromium-browser",
      "chromium",
    ];
  }

  let i: number = 0;
  while (i < candidates.length) {
    if (exists(candidates[i])) {
      return candidates[i];
    }
    if (platform.os !== OS_WINDOWS) {
      try {
        let result = runCommand("which", [candidates[i]], { encoding: "utf-8", timeout: 3000 });
        if (result.status === 0 && result.stdout.trim() !== "") {
          return candidates[i];
        }
      } catch (_e) {
        // ignore
      }
    }
    i = i + 1;
  }

  return "";
}

// ---------------------------------------------------------------------------
// Bridge script discovery
// ---------------------------------------------------------------------------

export function findBridgeScript(): string {
  let envPath: string = runtime.env["BROWSER_BRIDGE_PATH"] || "";
  if (envPath !== "" && exists(envPath)) {
    return envPath;
  }

  let searchRoots: string[] = [runtime.cwd()];
  searchRoots.unshift(resolvePath(import.meta.dir));

  let relativePaths: string[] = [
    "scripts/cdp_bridge_native.ts",
    "cdp_bridge_native.ts",
    "../scripts/cdp_bridge_native.ts",
  ];

  let ri: number = 0;
  while (ri < searchRoots.length) {
    let rpi: number = 0;
    while (rpi < relativePaths.length) {
      let candidate: string = joinPath(searchRoots[ri], relativePaths[rpi]);
      if (exists(candidate)) {
        return candidate;
      }
      rpi = rpi + 1;
    }
    let dir: string = searchRoots[ri];
    let level: number = 0;
    while (level < 5) {
      let parent: string = resolvePath(dir, "..");
      if (parent === dir) break;
      dir = parent;
      let rpi2: number = 0;
      while (rpi2 < relativePaths.length) {
        let candidate: string = joinPath(dir, relativePaths[rpi2]);
        if (exists(candidate)) {
          return candidate;
        }
        rpi2 = rpi2 + 1;
      }
      level = level + 1;
    }
    ri = ri + 1;
  }

  return "";
}

// ---------------------------------------------------------------------------
// Persistent bridge and relay discovery
// ---------------------------------------------------------------------------

export function findPersistentBridgeScript(): string {
  return findScriptByName("cdp_persistent_bridge_native.ts");
}

export function findRelayScript(): string {
  return findScriptByName("cdp_relay_native.ts");
}

function findScriptByName(scriptName: string): string {
  let searchRoots: string[] = [runtime.cwd()];
  // Resolve the installed SDK relative to this module.
  searchRoots.unshift(resolvePath(import.meta.dir));

  let relativePaths: string[] = [
    "cdpclient/features/node/" + scriptName,
    "scripts/" + scriptName,
    scriptName,
    "../cdpclient/features/node/" + scriptName,
    "../scripts/" + scriptName,
  ];

  let ri: number = 0;
  while (ri < searchRoots.length) {
    let rpi: number = 0;
    while (rpi < relativePaths.length) {
      let candidate: string = joinPath(searchRoots[ri], relativePaths[rpi]);
      if (exists(candidate)) {
        return candidate;
      }
      rpi = rpi + 1;
    }
    let dir: string = searchRoots[ri];
    let level: number = 0;
    while (level < 5) {
      let parent: string = resolvePath(dir, "..");
      if (parent === dir) break;
      dir = parent;
      let rpi2: number = 0;
      while (rpi2 < relativePaths.length) {
        let candidate: string = joinPath(dir, relativePaths[rpi2]);
        if (exists(candidate)) {
          return candidate;
        }
        rpi2 = rpi2 + 1;
      }
      level = level + 1;
    }
    ri = ri + 1;
  }
  return "";
}

// ---------------------------------------------------------------------------
// Debugger endpoint helpers
// ---------------------------------------------------------------------------

export function getWsUrl(port: number): string {
  let url: string = "http://localhost:" + port + "/json";
  try {
    let body: string = fetchText(url);
    // Extract webSocketDebuggerUrl via string search (avoids JSON.parse
    // which requires dynamic object property access not yet supported in JIT).
    let key: string = "webSocketDebuggerUrl";
    let idx: number = body.indexOf(key);
    if (idx < 0) return "";
    // Find the value after "webSocketDebuggerUrl": "..."
    let start: number = body.indexOf("ws://", idx);
    if (start < 0) return "";
    let end: number = body.indexOf('"', start);
    if (end < 0) return "";
    return body.substring(start, end);
  } catch (_e) {
    return "";
  }
}

export function waitForDebugger(port: number, timeoutMs: number): string {
  let start: number = Date.now();

  while (Date.now() - start < timeoutMs) {
    let ws: string = getWsUrl(port);
    if (ws !== "") return ws;

    sleepMs(200);
  }

  return "";
}

// ---------------------------------------------------------------------------
// Launch browser
// ---------------------------------------------------------------------------

let nextPort: number = 9400 + Math.floor(Math.random() * 500);

export function getNextPort(): number {
  let port: number = nextPort;
  nextPort = nextPort + 1;
  return port;
}

export function launchBrowserProcess(config: LaunchConfig): BrowserProcess {
  let chrome: string = config.executablePath;
  if (chrome === "") {
    chrome = findChromeBinary();
  }
  if (chrome === "") {
    throw new Error("Chrome not found. Install Chrome or set CHROME_PATH.");
  }

  let port: number = getNextPort();
  let profile = joinPath(runtime.env.TEMP || runtime.env.TMPDIR || "/tmp", "mweb-browser-" + crypto.randomUUID());
  makeDirectory(profile);

  // Extract viewport dimensions before building args (avoids nested property access in string concat)
  let vp = config.viewport;
  let vpWidth: number = vp.width;
  let vpHeight: number = vp.height;
  if (vpWidth === 0) vpWidth = 1280;
  if (vpHeight === 0) vpHeight = 720;

  let args: string[] = [
    "--headless=new",
    "--remote-debugging-port=" + port,
    "--user-data-dir=" + profile,
    "--no-first-run",
    "--enable-automation",
    "--no-default-browser-check",
    "--disable-gpu",
    "--disable-extensions",
    "--disable-translate",
    "--disable-background-networking",
    "--mute-audio",
    "--window-size=" + vpWidth + "," + vpHeight,
    "about:blank",
  ];

  if (!config.headless) {
    args[0] = "--no-startup-window";
  }

  let i: number = 0;
  while (i < config.args.length) {
    args.push(config.args[i]);
    i = i + 1;
  }

  let chromeProcess: HostProcess = startProcess(chrome, args, {
    stdio: ["ignore", "ignore", "pipe"],
  });

  return new BrowserProcess(chromeProcess, port, profile);
}
