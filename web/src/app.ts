// QuestForge Control Plane Dashboard entrypoint
import { DashboardBase } from './features/dashboard/dashboard_component.gen';

const root = document.getElementById('app');
if (root) {
  root.innerHTML = `
    <style>
      :root {
        --spacing-sm: 8px;
        --spacing-md: 16px;
        --spacing-lg: 24px;
        --radius-sm: 6px;
        --radius-md: 10px;
        --font-family: system-ui, -apple-system, sans-serif;
        --color-primary: #0f172a;
        --color-on-primary: #f8fafc;
        --color-secondary: #1e293b;
        --color-on-secondary: #e2e8f0;
        --color-border: #334155;
        --color-focus: #38bdf8;
      }
      body { margin: 0; background: #020617; color: #f8fafc; font-family: var(--font-family); }
      .qf-shell { max-width: 960px; margin: 32px auto; padding: 0 16px; }
      .qf-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 12px; margin-top: 16px; }
      .qf-card { background: #0f172a; border: 1px solid #1e293b; border-radius: 8px; padding: 14px; }
      .qf-card h3 { margin: 0 0 8px; font-size: 0.95rem; color: #38bdf8; }
      .qf-row { display: flex; gap: 8px; align-items: center; margin-top: 12px; flex-wrap: wrap; }
      .qf-btn { background: #2563eb; color: #fff; border: none; border-radius: 6px; padding: 8px 14px; cursor: pointer; font-weight: 600; }
      .qf-input { background: #020617; color: #f8fafc; border: 1px solid #334155; border-radius: 6px; padding: 8px 10px; min-width: 220px; }
      pre { margin: 0; white-space: pre-wrap; font-size: 0.8rem; color: #cbd5e1; }
    </style>
    <div class="qf-shell">
      <${DashboardBase.tagName}>
        <div slot="header">
          <h1 style="margin:0;">QuestForge Control Plane</h1>
          <p style="margin:4px 0 0;color:#94a3b8;">Meta Quest / Horizon OS DevOps &amp; Fleet Dashboard</p>
        </div>
        <div class="qf-row">
          <span data-part="label">New Project:</span>
          <input data-part="input" class="qf-input" id="qf-proj-name" placeholder="quest-sample" value="quest-sample">
          <button data-part="button" class="qf-btn" id="qf-create-btn">Create Project</button>
          <a data-part="link" href="/api.html" style="color:#38bdf8;margin-left:auto;">OpenAPI Docs</a>
        </div>
        <div class="qf-grid">
          <div class="qf-card"><h3>Projects</h3><pre id="qf-projects">Loading...</pre></div>
          <div class="qf-card"><h3>Builds</h3><pre id="qf-builds">Loading...</pre></div>
          <div class="qf-card"><h3>Deployments</h3><pre id="qf-deployments">Loading...</pre></div>
          <div class="qf-card"><h3>Audit Log</h3><pre id="qf-audit">Loading...</pre></div>
        </div>
        <div slot="footer" style="margin-top:16px;color:#64748b;font-size:0.85rem;">
          QuestForge Control Plane • Powered by machineweb (mweb)
        </div>
      </${DashboardBase.tagName}>
    </div>
  `;

  async function refresh(): Promise<void> {
    const endpoints: Array<[string, string]> = [
      ['qf-projects', '/api/projects'],
      ['qf-builds', '/api/builds'],
      ['qf-deployments', '/api/deployments'],
      ['qf-audit', '/api/audit'],
    ];
    for (const [id, url] of endpoints) {
      const el = document.getElementById(id);
      if (!el) continue;
      try {
        const res = await fetch(url);
        el.textContent = JSON.stringify(await res.json(), null, 2);
      } catch (err) {
        el.textContent = String(err);
      }
    }
  }

  const createBtn = document.getElementById('qf-create-btn');
  if (createBtn) {
    createBtn.addEventListener('click', async () => {
      const input = document.getElementById('qf-proj-name') as HTMLInputElement | null;
      const name = input?.value.trim() || 'quest-sample';
      await fetch('/api/projects', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'X-Quest-Role': 'developer' },
        body: JSON.stringify({ name, repository: 'https://github.com/sweengineeringlabs/questforge', owner: 'xr-team' }),
      });
      await refresh();
    });
  }

  void refresh();
}
