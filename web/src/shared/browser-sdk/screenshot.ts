// Screenshot comparison — ported from Rust screenshot.rs.
// Pixel diffing math is WASM-safe; file I/O parts are Bun host.

import { readBytes, writeBytes, exists, makeDirectory } from "./host";
import { joinPath } from "./host";
import { COMPARE_MATCH, COMPARE_MISMATCH, COMPARE_NEW_BASELINE, CompareResult } from "./types";

// ---------------------------------------------------------------------------
// ScreenshotDiffer
// ---------------------------------------------------------------------------

export class ScreenshotDiffer {
  baselineDir: string;
  diffDir: string;
  threshold: number;
  antiAliasing: boolean;

  constructor(baselineDir: string, diffDir: string) {
    this.baselineDir = baselineDir;
    this.diffDir = diffDir;
    this.threshold = 0.1;
    this.antiAliasing = true;
  }

  setThreshold(threshold: number): ScreenshotDiffer {
    this.threshold = threshold;
    return this;
  }

  setAntiAliasing(enabled: boolean): ScreenshotDiffer {
    this.antiAliasing = enabled;
    return this;
  }

  compare(name: string, actualData: Uint8Array): CompareResult {
    let baselinePath: string = joinPath(this.baselineDir, name + ".png");

    if (!exists(baselinePath)) {
      return new CompareResult(COMPARE_NEW_BASELINE, 0, new Uint8Array(0));
    }

    let baselineData: Uint8Array = new Uint8Array(readBytes(baselinePath));
    let result = comparePixels(baselineData, actualData, this.threshold);
    return result;
  }

  updateBaseline(name: string, screenshotData: Uint8Array): void {
    if (!exists(this.baselineDir)) {
      makeDirectory(this.baselineDir, { recursive: true });
    }
    let path: string = joinPath(this.baselineDir, name + ".png");
    writeBytes(path, screenshotData);
  }

  saveDiff(name: string, actualData: Uint8Array): void {
    if (!exists(this.diffDir)) {
      makeDirectory(this.diffDir, { recursive: true });
    }
    let actualPath: string = joinPath(this.diffDir, name + "-actual.png");
    writeBytes(actualPath, actualData);
  }
}

// ---------------------------------------------------------------------------
// Pixel comparison (WASM-safe math)
// ---------------------------------------------------------------------------

export function pixelsDiffer(
  r1: number, g1: number, b1: number, a1: number,
  r2: number, g2: number, b2: number, a2: number
): boolean {
  let threshold: number = 35;
  let dr: number = r1 - r2; if (dr < 0) dr = -dr;
  let dg: number = g1 - g2; if (dg < 0) dg = -dg;
  let db: number = b1 - b2; if (db < 0) db = -db;
  let da: number = a1 - a2; if (da < 0) da = -da;
  return dr > threshold || dg > threshold || db > threshold || da > threshold;
}

function comparePixels(baseline: Uint8Array, actual: Uint8Array, threshold: number): CompareResult {
  // Simple byte-level comparison for raw PNG data.
  // For a production implementation you'd decode the PNGs to pixel arrays.
  // This simplified version compares file contents directly.
  if (baseline.length === actual.length) {
    let diffBytes: number = 0;
    let totalBytes: number = baseline.length;
    let i: number = 0;
    while (i < totalBytes) {
      if (baseline[i] !== actual[i]) {
        diffBytes = diffBytes + 1;
      }
      i = i + 1;
    }
    let diffPct: number = (diffBytes / totalBytes) * 100;
    if (diffPct <= threshold) {
      return new CompareResult(COMPARE_MATCH, diffPct, new Uint8Array(0));
    }
    return new CompareResult(COMPARE_MISMATCH, diffPct, new Uint8Array(0));
  }

  // Different sizes — definitely a mismatch
  let sizeDiff: number = baseline.length - actual.length;
  if (sizeDiff < 0) sizeDiff = -sizeDiff;
  let maxSize: number = baseline.length > actual.length ? baseline.length : actual.length;
  let pct: number = (sizeDiff / maxSize) * 100;
  return new CompareResult(COMPARE_MISMATCH, pct, new Uint8Array(0));
}
