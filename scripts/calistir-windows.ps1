#Requires -Version 5.1
<#
.SYNOPSIS
    FastFind tek komutla calistirir; eksik bagimliliklari otomatik kurar.
.DESCRIPTION
    1. cargo yoksa: winget ile Rustup kurar, stable toolchain ekler.
    2. MSVC linker (cl.exe) yoksa: VS 2022 Build Tools kurmayi dener.
    3. cargo fetch ile crate bagimliliklarini indirir.
    4. cargo run --release ile uygulamayi baslatir.
.PARAMETER Derle
    Uygulamayi calistirmadan yalnizca release derlemesi yapar.
.PARAMETER Test
    Calistirmadan once testleri kosar.
.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\calistir-windows.ps1
#>
param(
    [switch]$Derle,
    [switch]$Test
)
$ErrorActionPreference = 'Stop'

function KomutVar([string]$ad) {
    return $null -ne (Get-Command $ad -ErrorAction SilentlyContinue)
}

function YoluYenile {
    $m = [System.Environment]::GetEnvironmentVariable('Path', 'Machine')
    $u = [System.Environment]::GetEnvironmentVariable('Path', 'User')
    $env:Path = "$m;$u"
}

$projeKok = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projeKok
Write-Host "== FastFind calistiriliyor ($projeKok) ==" -ForegroundColor Cyan

# 1) Rust toolchain yoksa kur
if (-not (KomutVar 'cargo')) {
    Write-Host '-- cargo bulunamadi, Rustup kuruluyor...' -ForegroundColor Yellow
    winget install --id Rustlang.Rustup -e --silent `
        --accept-package-agreements --accept-source-agreements
    YoluYenile
}
if (-not (KomutVar 'cargo')) {
    throw 'cargo kurulamadi. https://rustup.rs adresinden elle kurun.'
}
rustup toolchain install stable --profile default | Out-Null
rustup default stable | Out-Null
cargo --version

# 2) MSVC linker yoksa Build Tools kurmayi dene
if (-not (KomutVar 'cl')) {
    Write-Host '-- MSVC linker bulunamadi, Build Tools kuruluyor (uzun surebilir)...' -ForegroundColor Yellow
    try {
        winget install --id Microsoft.VisualStudio.2022.BuildTools -e --silent `
            --accept-package-agreements --accept-source-agreements `
            --override '--wait --quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --passive'
        YoluYenile
    } catch {
        Write-Host 'Otomatik kurulum olmadi, elle kurun:' -ForegroundColor Red
        Write-Host 'https://visualstudio.microsoft.com/downloads/ ("C++ ile masaustu gelistirme")'
        throw 'MSVC linker eksik.'
    }
}

# 3) Crate bagimliliklarini indir (yoksa yuklenir, varsa tazelenir)
Write-Host '-- bagimliliklar indiriliyor...' -ForegroundColor Cyan
cargo fetch

# 4) Test / derle / calistir
if ($Test) {
    Write-Host '-- testler kosuyor...' -ForegroundColor Cyan
    cargo test --locked
}
if ($Derle) {
    Write-Host '-- release derleniyor...' -ForegroundColor Cyan
    cargo build --release --locked
} else {
    Write-Host '-- uygulama baslatiliyor...' -ForegroundColor Green
    cargo run --release --locked
}
