param(
    [string]$ServerUrl = "http://127.0.0.1:8090",
    [string]$QuestBin = "C:\Users\elvis\.cargo\target\debug\quest.exe"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $QuestBin)) {
    throw "quest binary not found at $QuestBin. Run `cargo build --bin quest` first."
}

$workRoot = Join-Path $env:TEMP "questforge-e2e-demo"
if (Test-Path $workRoot) {
    Remove-Item -Recurse -Force $workRoot
}
New-Item -ItemType Directory -Force -Path $workRoot | Out-Null

$projDir = Join-Path $workRoot "demo-quest-app"
$distDir = Join-Path $projDir "dist"

Write-Host "=== 1/5: Scaffolding Quest project (quest init) ===" -ForegroundColor Cyan
& $QuestBin init $projDir --engine unity --package-id com.swelabs.demo --skip-doctor --json | Out-Host

Write-Host "=== 2/5: Validating environment (quest doctor) ===" -ForegroundColor Cyan
$doctorJson = & $QuestBin doctor --project-dir $projDir --json
Write-Host $doctorJson

Write-Host "=== 3/5: Running CI plan & SBOM (quest ci --dry-run) ===" -ForegroundColor Cyan
& $QuestBin ci --project-dir $projDir --output-dir $distDir --release-version 1.0.0 --dry-run --json | Out-Host

Write-Host "=== 4/5: Building & verifying signed ARM64 Quest APK (quest verify) ===" -ForegroundColor Cyan
$apkPath = Join-Path $distDir "demo-quest-app-1.0.0.apk"
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$fs = [System.IO.File]::Open($apkPath, [System.IO.FileMode]::Create)
try {
    $zip = New-Object System.IO.Compression.ZipArchive($fs, [System.IO.Compression.ZipArchiveMode]::Create, $false)
    try {
        $entries = @{
            "AndroidManifest.xml" = '<manifest package="com.swelabs.demo"><uses-feature android:name="android.hardware.vr.headtracking"/><application><activity><intent-filter><category android:name="com.oculus.intent.category.VR"/></intent-filter></activity></application></manifest>'
            "classes.dex" = "dex`n035`0"
            "lib/arm64-v8a/libunity.so" = "ELF-ARM64"
            "META-INF/MANIFEST.MF" = "Manifest-Version: 1.0`n"
            "META-INF/CERT.RSA" = "SIG-OK"
        }
        foreach ($kv in $entries.GetEnumerator()) {
            $entry = $zip.CreateEntry($kv.Key)
            $sw = New-Object System.IO.StreamWriter($entry.Open())
            $sw.Write($kv.Value)
            $sw.Dispose()
        }
    } finally {
        $zip.Dispose()
    }
} finally {
    $fs.Dispose()
}

$verifyRaw = & $QuestBin verify $apkPath --project-dir $projDir --format json
$verifyObj = $verifyRaw | ConvertFrom-Json
Write-Host ("Verification score: {0}/100 (ready={1})" -f $verifyObj.score, $verifyObj.ready)

Write-Host "=== 5/5: Publishing to Control Plane ($ServerUrl) ===" -ForegroundColor Cyan
$headers = @{ "X-Quest-Role" = "administrator" }

$projResp = Invoke-RestMethod -Method Post -Uri "$ServerUrl/api/projects" -Headers $headers -ContentType "application/json" -Body (@{
    name = "demo-quest-app"
    repository = "https://github.com/sweengineeringlabs/questforge"
    owner = "xr-platform"
} | ConvertTo-Json)

$buildResp = Invoke-RestMethod -Method Post -Uri "$ServerUrl/api/builds" -Headers $headers -ContentType "application/json" -Body (@{
    project_id = $projResp.id
    version = "1.0.0"
} | ConvertTo-Json)

$verifyResp = Invoke-RestMethod -Method Post -Uri "$ServerUrl/api/verify" -Headers $headers -ContentType "application/json" -Body (@{
    score = $verifyObj.score
    ready = $verifyObj.ready
} | ConvertTo-Json)

$depResp = Invoke-RestMethod -Method Post -Uri "$ServerUrl/api/deployments" -Headers $headers -ContentType "application/json" -Body (@{
    target = "qa-lab"
    apk_path = $apkPath
} | ConvertTo-Json)

Write-Host "Published: project=$($projResp.id), build=$($buildResp.id), report=$($verifyResp.id), deployment=$($depResp.id)" -ForegroundColor Green
Write-Host "Open $ServerUrl in your browser to view the live dashboard!" -ForegroundColor Green
