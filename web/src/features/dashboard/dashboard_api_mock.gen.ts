// AUTO-GENERATED — do not edit. Regenerate with: mweb generate api dashboard
// Source: dashboard.yaml

import { registerMock } from "../../shared/api-runtime";

registerMock("GET", "/api/audit", function(): any {
  return [
  {
    "method": "POST",
    "path": "/api/deployments",
    "role": "operations_engineer",
    "seq": 1,
    "status_code": 201
  }
];
});

registerMock("GET", "/api/builds", function(): any {
  return [
  {
    "id": "build-1",
    "project_id": "proj-1",
    "status": "succeeded",
    "version": "1.0.0"
  }
];
});

registerMock("GET", "/api/deployments", function(): any {
  return [
  {
    "apk_path": "dist/quest-sample-1.0.0.apk",
    "status": "deployed",
    "target": "qa-lab"
  }
];
});

registerMock("GET", "/api/projects", function(): any {
  return [
  {
    "id": "proj-1",
    "name": "quest-sample",
    "owner": "xr-platform",
    "repository": "https://github.com/sweengineeringlabs/questforge"
  }
];
});

registerMock("GET", "/api/reports", function(): any {
  return [
  {
    "id": "rep-1",
    "ready": true,
    "score": 100
  }
];
});

