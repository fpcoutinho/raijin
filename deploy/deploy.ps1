#Requires -Version 7
<#
.SYNOPSIS
    Compila o raijin para ARM64 e atualiza o código da função Lambda.
.DESCRIPTION
    Só código. Variável de ambiente, timeout e memória são alterados por
    `aws lambda update-function-configuration`, de propósito: assim o script
    não desfaz ajuste feito no console. Ver deploy/aws-deploy.md.
#>
param(
    [string]$FunctionName = 'raijin',
    [string]$Region = $env:AWS_REGION
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot

try {
    foreach ($tool in 'cargo-lambda', 'aws') {
        if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
            throw "$tool não encontrado no PATH. Ver deploy/aws-deploy.md."
        }
    }

    # O Zig é o linker da cross-compilação, e `winget install zig.zig` não o
    # registra no PATH — sem esta checagem o sintoma é erro de linker sem
    # relação aparente com o Zig.
    if (-not (Get-Command zig -ErrorAction SilentlyContinue)) {
        $winget = Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" `
            -Filter 'zig.exe' -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1

        if (-not $winget) {
            throw 'zig não encontrado. Instale com `winget install zig.zig` — é o linker da cross-compilação para ARM64.'
        }

        Write-Host "zig fora do PATH; usando $($winget.DirectoryName)" -ForegroundColor Yellow
        $env:PATH = "$($winget.DirectoryName);$env:PATH"
    }

    Write-Host 'Compilando para arm64...' -ForegroundColor Cyan
    cargo lambda build --release --arm64
    if ($LASTEXITCODE -ne 0) { throw 'cargo lambda build falhou.' }

    $zip = Join-Path $repoRoot 'target/lambda/raijin/bootstrap.zip'
    if (-not (Test-Path $zip)) { throw "artefato não encontrado em $zip" }

    Write-Host "Publicando $FunctionName..." -ForegroundColor Cyan
    $awsArgs = @(
        'lambda', 'update-function-code',
        '--function-name', $FunctionName,
        '--zip-file', "fileb://$zip",
        '--output', 'text',
        '--query', 'LastModified'
    )
    if ($Region) { $awsArgs += @('--region', $Region) }

    $lastModified = & aws @awsArgs
    if ($LASTEXITCODE -ne 0) { throw 'update-function-code falhou.' }

    Write-Host "Publicado: $lastModified" -ForegroundColor Green
}
finally {
    Pop-Location
}
