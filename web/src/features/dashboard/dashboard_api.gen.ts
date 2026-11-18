// AUTO-GENERATED — do not edit. Regenerate with: mweb generate api dashboard
// Source: dashboard.yaml

import { apiFetch, type HttpResult } from "../../shared/api-runtime";
import { DashboardData } from "./dashboard_types.gen";

// ── API functions ──

export async function listDashboard(): Promise<HttpResult<DashboardData[]>> {
  const r = await apiFetch<DashboardData[]>("GET", "/dashboard");
  if (!r.ok) {
    console.error("[listDashboard] " + r.error.code + ": " + r.error.message);
    return r;
  }
  return r;
}

export async function getDashboard(): Promise<HttpResult<DashboardData>> {
  const r = await apiFetch<DashboardData>("GET", "/dashboard/{{id}}");
  if (!r.ok) {
    console.error("[getDashboard] " + r.error.code + ": " + r.error.message);
    return r;
  }
  return r;
}

export async function createDashboard(body: unknown): Promise<HttpResult<DashboardData>> {
  const r = await apiFetch<DashboardData>("POST", "/dashboard", body);
  if (!r.ok) {
    console.error("[createDashboard] " + r.error.code + ": " + r.error.message);
    return r;
  }
  return r;
}

// Dev mode: import mock data
import "./dashboard_api_mock.gen";
