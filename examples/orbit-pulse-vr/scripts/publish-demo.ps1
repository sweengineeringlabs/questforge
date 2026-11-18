param(
    [string]$ServerUrl = "http://127.0.0.1:8090"
)

$ErrorActionPreference = "Stop"
$ProjectDir = Split-Path -Parent $PSScriptRoot
$ApkPath = Join-Path $ProjectDir "dist\orbit-pulse-vr-1.0.0.apk"

Write-Host "Checking QuestForge Control Plane at $ServerUrl..."
try {
    $null = Invoke-RestMethod -Method Get -Uri "$ServerUrl/health"
} catch {
    Write-Error "Control Plane is not reachable at $ServerUrl. Start it first with: quest serve --addr 127.0.0.1:8090 --web-root web/dist"
}

if (-not (Test-Path $ApkPath)) {
    Write-Host "Building orbit-pulse-vr APK..."
    quest ci --project-dir $ProjectDir --output-dir (Join-Path $ProjectDir "dist") --release-version 1.0.0
}

$verifyJson = quest verify $ApkPath --project-dir $ProjectDir --format json | ConvertFrom-Json
$headers = @{ "X-Quest-Role" = "admin" }

$project = Invoke-RestMethod -Method Post -Uri "$ServerUrl/api/projects" -Headers $headers -ContentType "application/json" -Body (@{
    name       = "orbit-pulse-vr"
    repository = "examples/orbit-pulse-vr"
    owner      = "swelabs"
} | ConvertTo-Json)

$build = Invoke-RestMethod -Method Post -Uri "$ServerUrl/api/builds" -Headers $headers -ContentType "application/json" -Body (@{
    project_id = $project.id
    version    = "1.0.0"
} | ConvertTo-Json)

$report = Invoke-RestMethod -Method Post -Uri "$ServerUrl/api/verify" -Headers $headers -ContentType "application/json" -Body (@{
    apk_path = "dist/orbit-pulse-vr-1.0.0.apk"
    score    = [int]$verifyJson.score
    ready    = [bool]$verifyJson.ready
} | ConvertTo-Json)

Write-Host "Published to $ServerUrl"
Write-Host "  Project: $($project.id) ($($project.name))"
Write-Host "  Build:   $($build.id) ($($build.version))"
Write-Host "  Report:  $($report.id) (score: $($report.score)/100, ready: $($report.ready))"
Write-Host ""
Write-Host "Open $ServerUrl/ in your browser to view orbit-pulse-vr."
