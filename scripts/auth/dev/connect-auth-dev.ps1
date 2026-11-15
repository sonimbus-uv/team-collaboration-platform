$ErrorActionPreference = 'Stop'
$projectDirectory = (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path

Push-Location $projectDirectory
try {
    Write-Host 'Auth gateway: https://localhost:8443; keep this terminal open. Ctrl+C closes the tunnels.'
    & vagrant ssh -- -N -T -o ExitOnForwardFailure=yes -o ServerAliveInterval=30 -o ServerAliveCountMax=3 `
        -L 127.0.0.1:8443:127.0.0.1:8443 `
        -L 127.0.0.1:18080:127.0.0.1:8080 `
        -L 127.0.0.1:18025:127.0.0.1:8025 `
        -L 127.0.0.1:18443:127.0.0.1:3443
    if ($LASTEXITCODE -ne 0) {
        throw 'Development tunnel stopped with an error. Check Vagrant and local port availability.'
    }
}
finally {
    Pop-Location
}
