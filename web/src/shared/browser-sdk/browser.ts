// Browser — top-level browser automation API. Ported from Rust browser.rs.

import { fetchText, sleep } from "./host";
import { CdpClient } from "./cdp_client";
import { Page } from "./page";
import {
  LaunchConfig, BrowserProcess, defaultLaunchConfig,
  launchBrowserProcess, waitForDebugger,
  getWsUrl, findPersistentBridgeScript, findRelayScript,
} from "./manager";
import { findAvailablePort } from "./server";
import { Viewport, viewportDefault } from "./types";

// ---------------------------------------------------------------------------
// Process cleanup — kill all tracked browsers on exit to prevent orphaned Chrome
// ---------------------------------------------------------------------------

let activeBrowsers: Browser[] = [];

export function closeAllBrowsers(): void {
  for (const browser of [...activeBrowsers]) browser.close();
}

// ---------------------------------------------------------------------------
// Browser class
// ---------------------------------------------------------------------------

export class Browser {
  cdp: CdpClient;
  process: BrowserProcess;
  config: LaunchConfig;
  pages: Page[];

  constructor(cdp: CdpClient, browserProcess: BrowserProcess, config: LaunchConfig) {
    this.cdp = cdp;
    this.process = browserProcess;
    this.config = config;
    this.pages = [];
  }

  // -----------------------------------------------------------------------
  // Launch
  // -----------------------------------------------------------------------

  static launch(config: LaunchConfig): Browser {
    let bp: BrowserProcess = launchBrowserProcess(config);
    let wsUrl: string = waitForDebugger(bp.port, config.timeout);
    if (wsUrl === "") {
      bp.kill();
      throw new Error("Chrome debugger did not respond within " + config.timeout + "ms on port " + bp.port);
    }

    // The Bun worker owns a persistent WebSocket to Chrome.
    let cdp: CdpClient;
    try { cdp = CdpClient.connectDirect(wsUrl, "", config.timeout); }
    catch (error) { bp.kill(); throw error; }
    let browser: Browser = new Browser(cdp, bp, config);
    activeBrowsers.push(browser);
    return browser;
  }

  static launchDefault(): Browser {
    return Browser.launch(defaultLaunchConfig());
  }

  // -----------------------------------------------------------------------
  // Page management
  // -----------------------------------------------------------------------

  newPage(): Page {
    // Use a persistent connection to each page's direct WebSocket endpoint.
    // The initial Chrome launch already has a page at about:blank — reuse it
    // for the first page, or create a new target for subsequent pages.
    let pageCdp: CdpClient;
    let targetId: string = "";

    if (this.pages.length === 0) {
      // Reuse the initial page (already connected)
      pageCdp = this.cdp;
      targetId = "initial";
    } else if (this.cdp.isNative()) {
      // Native: create a new target and attach via session on the existing WS connection.
      // Sessions are multiplexed — no new port or bridge process needed.
      let newTargetId: string = this.cdp.targetCreateTarget("about:blank");
      if (newTargetId === "") {
        throw new Error("Failed to create new target");
      }
      let sessionId: string = this.cdp.targetAttachToTarget(newTargetId);
      if (sessionId === "") {
        throw new Error("Failed to attach to new target (targetId=" + newTargetId + ")");
      }
      pageCdp = this.cdp.withSession(sessionId);
      targetId = newTargetId;
    } else {
      // Fallback: open a new tab, find its WS URL, create a new connection.
      this.cdp.evaluate("window.open('about:blank')");
      this.sleepMs(500);
      let newWs: string = this.findNewPageWsUrl();
      if (newWs === "") {
        throw new Error("Failed to get WS URL for new page");
      }
      let pageStart: number = 19200 + Math.floor(Math.random() * 800);
      let pagePort: number = findAvailablePort(pageStart, 200);
      if (pagePort <= 0) {
        throw new Error("No available port found for new page bridge (searched " + pageStart + "-" + (pageStart + 199) + ").");
      }
      pageCdp = CdpClient.createPersistent(
        newWs, this.cdp.bridgeScript, this.cdp.relayScript, "", pagePort
      );
      targetId = "page-" + this.pages.length;
    }

    let vp: Viewport = this.config.viewport;
    let page: Page = new Page(pageCdp, targetId, "", vp, this.config.timeout, this.config.slowMo);
    this.pages.push(page);
    return page;
  }

  private findNewPageWsUrl(): string {
    let url: string = "http://localhost:" + this.process.port + "/json";
    try {
      let targets = JSON.parse(fetchText(url));
      // Return the last page target (newest)
      let i: number = targets.length - 1;
      while (i >= 0) {
        if (targets[i].type === "page" && targets[i].webSocketDebuggerUrl) {
          return targets[i].webSocketDebuggerUrl;
        }
        i = i - 1;
      }
    } catch (_e) { /* ignore */ }
    return "";
  }

  private sleepMs(ms: number): void {
    sleep(ms);
  }

  // -----------------------------------------------------------------------
  // Info
  // -----------------------------------------------------------------------

  version(): string {
    let result: string = this.cdp.sendJson("Browser.getVersion", "");
    if (result === "") return "";
    return this.cdp.extractStringField(result, "product");
  }

  userAgent(): string {
    let result: string = this.cdp.sendJson("Browser.getVersion", "");
    if (result === "") return "";
    return this.cdp.extractStringField(result, "userAgent");
  }

  // -----------------------------------------------------------------------
  // Context management
  // -----------------------------------------------------------------------

  newContext(): BrowserContext {
    let result: string = this.cdp.sendJson("Target.createBrowserContext", "");
    let contextId: string = "";
    if (result !== "") {
      contextId = this.cdp.extractStringField(result, "browserContextId");
    }
    return new BrowserContext(this, contextId);
  }

  disposeContext(contextId: string): void {
    this.cdp.sendJson("Target.disposeBrowserContext", '{"browserContextId":"' + contextId + '"}');
  }

  // -----------------------------------------------------------------------
  // Close
  // -----------------------------------------------------------------------

  close(): void {
    let i: number = 0;
    while (i < this.pages.length) {
      this.pages[i].close();
      i = i + 1;
    }
    this.pages = [];
    this.cdp.closeBridge();
    this.process.kill();

    // Remove from active list so exit handler doesn't double-kill
    let idx: number = activeBrowsers.indexOf(this);
    if (idx >= 0) {
      activeBrowsers.splice(idx, 1);
    }
  }
}

// ---------------------------------------------------------------------------
// BrowserContext
// ---------------------------------------------------------------------------

export class BrowserContext {
  browser: Browser;
  contextId: string;

  constructor(browser: Browser, contextId: string) {
    this.browser = browser;
    this.contextId = contextId;
  }

  newPage(): Page {
    // Delegate to browser's newPage — context isolation is limited
    // with the bridge approach since sessions don't persist
    return this.browser.newPage();
  }

  cookies(): string {
    return this.browser.cdp.networkGetCookies([]);
  }

  clearCookies(): void {
    this.browser.cdp.networkClearBrowserCookies();
  }

  close(): void {
    if (this.contextId !== "") {
      this.browser.disposeContext(this.contextId);
    }
  }
}

// Standalone entrypoint shares the same lifecycle tracking as the class API.
export function launchBrowser(config: LaunchConfig): Browser {
  return Browser.launch(config);
}
