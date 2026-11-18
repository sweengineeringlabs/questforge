// AUTO-GENERATED — do not edit. Regenerate with: mweb generate api dashboard
// Source: dashboard.yaml

import { apiFetch, type HttpResult } from "../../shared/api-runtime";
import { AuditLogEntry, BuildRecord, DeploymentRecord, ProjectRecord, VerificationRecord } from "./dashboard_types.gen";

// ── API functions ──

export async function listProjects(): Promise<HttpResult<ProjectRecord[]>> {
  const r = await apiFetch<ProjectRecord[]>("GET", "/api/projects");
  if (!r.ok) {
    console.error("[listProjects] " + r.error.code + ": " + r.error.message);
    return r;
  }
  return r;
}

export async function createProject(body: unknown): Promise<HttpResult<ProjectRecord>> {
  const r = await apiFetch<ProjectRecord>("POST", "/api/projects", body);
  if (!r.ok) {
    console.error("[createProject] " + r.error.code + ": " + r.error.message);
    return r;
  }
  return r;
}

export async function listBuilds(): Promise<HttpResult<BuildRecord[]>> {
  const r = await apiFetch<BuildRecord[]>("GET", "/api/builds");
  if (!r.ok) {
    console.error("[listBuilds] " + r.error.code + ": " + r.error.message);
    return r;
  }
  return r;
}

export async function createBuild(body: unknown): Promise<HttpResult<BuildRecord>> {
  const r = await apiFetch<BuildRecord>("POST", "/api/builds", body);
  if (!r.ok) {
    console.error("[createBuild] " + r.error.code + ": " + r.error.message);
    return r;
  }
  return r;
}

export async function listReports(): Promise<HttpResult<VerificationRecord[]>> {
  const r = await apiFetch<VerificationRecord[]>("GET", "/api/reports");
  if (!r.ok) {
    console.error("[listReports] " + r.error.code + ": " + r.error.message);
    return r;
  }
  return r;
}

export async function listDeployments(): Promise<HttpResult<DeploymentRecord[]>> {
  const r = await apiFetch<DeploymentRecord[]>("GET", "/api/deployments");
  if (!r.ok) {
    console.error("[listDeployments] " + r.error.code + ": " + r.error.message);
    return r;
  }
  return r;
}

export async function createDeployment(body: unknown): Promise<HttpResult<DeploymentRecord>> {
  const r = await apiFetch<DeploymentRecord>("POST", "/api/deployments", body);
  if (!r.ok) {
    console.error("[createDeployment] " + r.error.code + ": " + r.error.message);
    return r;
  }
  return r;
}

export async function listAudit(): Promise<HttpResult<AuditLogEntry[]>> {
  const r = await apiFetch<AuditLogEntry[]>("GET", "/api/audit");
  if (!r.ok) {
    console.error("[listAudit] " + r.error.code + ": " + r.error.message);
    return r;
  }
  return r;
}

// Dev mode: import mock data
import "./dashboard_api_mock.gen";
