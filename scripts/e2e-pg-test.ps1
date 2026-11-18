param(
    [string]$DatabaseUrl = "postgres://questforge:questforge@127.0.0.1:5432/questforge",
    [string]$Addr = "127.0.0.1:8091",
    [string]$QuestBin = "C:\Users\elvis\.cargo\target\debug\quest.exe"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $QuestBin)) {
    throw "quest binary not found at $QuestBin. Run `cargo build --bin quest` first."
}

$serverUrl = "http://$Addr"
Write-Host "Starting quest serve with PostgreSQL at $Addr..." -ForegroundColor Cyan
$proc = Start-Process -FilePath $QuestBin -ArgumentList @(
    "serve",
    "--addr", $Addr,
    "--web-root", "web/dist",
    "--database-url", $DatabaseUrl
) -PassThru -NoNewWindow

try {
    $ready = $false
    for ($i = 0; $i -lt 20; $i++) {
        Start-Sleep -Milliseconds 300
        try {
            $health = Invoke-RestMethod -Uri "$serverUrl/health" -TimeoutSec 2
            if ($health.status -eq "ok") {
                $ready = $true
                break
            }
        } catch {
            # wait for bind
        }
    }
    if (-not $ready) {
        throw "quest serve did not become healthy at $serverUrl/health"
    }

    & "$PSScriptRoot\e2e-demo.ps1" -ServerUrl $serverUrl -QuestBin $QuestBin

    $projects = Invoke-RestMethod -Uri "$serverUrl/api/projects"
    $audit = Invoke-RestMethod -Uri "$serverUrl/api/audit"
    Write-Host ("PostgreSQL E2E passed: {0} project(s), {1} audit event(s) persisted." -f $projects.projects.Count, $audit.audit_log.Count) -ForegroundColor Green
} finally {
    if ($proc -and -not $proc.HasExited) {
        Stop-Process -Id $proc.Id -Force
    }
}
