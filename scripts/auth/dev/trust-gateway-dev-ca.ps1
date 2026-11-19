$ErrorActionPreference = 'Stop'
$projectDirectory = (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
$certificatePath = Join-Path $projectDirectory 'secrets/gateway-dev/ca.crt'
$certificate = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new($certificatePath)

try {
    if ($certificate.Subject -ne 'CN=Sonimbus Gateway Development CA' -or
        $certificate.Issuer -ne $certificate.Subject -or
        $certificate.HasPrivateKey -or
        $certificate.NotAfter -le (Get-Date) -or
        $certificate.NotBefore -gt (Get-Date)) {
        throw 'The gateway development CA is invalid. Provision it inside the VM first.'
    }
    $constraints = $certificate.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.19' }
    if (-not $constraints -or -not $constraints.CertificateAuthority) {
        throw 'The exported certificate is not a development CA.'
    }
    $storePath = "Cert:\CurrentUser\Root\$($certificate.Thumbprint)"
    if (-not (Test-Path -LiteralPath $storePath)) {
        Import-Certificate -FilePath $certificatePath -CertStoreLocation 'Cert:\CurrentUser\Root' | Out-Null
    }
    Write-Host "Gateway CA trusted for the current Windows user: $($certificate.Thumbprint)"
}
finally {
    $certificate.Dispose()
}
