$ErrorActionPreference='Stop'
$dir=Join-Path $env:TEMP 'termian-ssh-live-test-177'
New-Item -ItemType Directory -Force -Path $dir | Out-Null
$exe=Join-Path $dir 'termian.exe'
Copy-Item 'src-tauri\target\release\termian.exe' $exe -Force
$data=Join-Path $dir 'termian-data'
New-Item -ItemType Directory -Force -Path $data | Out-Null
$hostJson='[{"id":"live-test-177","name":"SSH 192.168.1.177 test","address":"192.168.1.177","port":2001,"username":"probe","group":"","tags":"","keyPath":""}]'
[IO.File]::WriteAllText((Join-Path $data 'hosts.json'), $hostJson, [Text.UTF8Encoding]::new($false))
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-port=9317'
$env:WEBVIEW2_USER_DATA_FOLDER=Join-Path $dir 'webview-data'
$p=Start-Process -FilePath $exe -WorkingDirectory $dir -PassThru
Start-Sleep -Seconds 4
$p.Refresh()
Write-Host ('LIVE_TAURI_PROCESS_RUNNING='+(-not $p.HasExited))
Write-Host ('PID='+$p.Id)
try {
  $doc=Invoke-RestMethod -Uri 'http://127.0.0.1:9317/json' -TimeoutSec 4
  Write-Host ('WEBVIEW_PAGES='+$doc.Count)
  Write-Host ('WEBVIEW_URL='+$doc[0].url)
} catch { Write-Host ('CDP_ERROR='+$_.Exception.Message) }
