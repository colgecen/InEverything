#Requires -Version 5.1
<#
.SYNOPSIS
    InEverything geliştirme ortamı kurulum betiği (Windows).
.DESCRIPTION
    Rust stable toolchain + MSVC linker önkoşulunu kurar ve projeyi derler.
    Kurumsal proxy arkasındaysanız .exe indirmeleri engellenebilir; o durumda
    bu betiği engelsiz bir ağda çalıştırın.
#>
$ErrorActionPreference = 'Stop'

Write-Host '== InEverything kurulumu basliyor ==' -ForegroundColor Cyan

# 1) Rustup (yoksa kur)
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host '-- Rustup kuruluyor...' -ForegroundColor Yellow
    winget install --id Rustlang.Rustup -e --silent `
        --accept-package-agreements --accept-source-agreements
    $env:Path = [System.Environment]::GetEnvironmentVariable('Path', 'Machine') + ';' +
                [System.Environment]::GetEnvironmentVariable('Path', 'User')
}

rustup toolchain install stable --profile default
rustup default stable

cargo --version
rustc --version

# 2) MSVC linker kontrolü (eframe/egui MSVC hedefi ister)
if (-not (Get-Command cl -ErrorAction SilentlyContinue)) {
    Write-Host 'UYARI: MSVC derleyicisi (cl.exe) bulunamadi.' -ForegroundColor Red
    Write-Host 'Visual Studio Build Tools + "C++ ile masaustu gelistirme" yukleyin:'
    Write-Host 'https://visualstudio.microsoft.com/downloads/'
    throw 'MSVC linker eksik, kurulum durduruldu.'
}

# 3) Proje dogrulama kapilari
Write-Host '-- fmt / clippy / test / build --' -ForegroundColor Cyan
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo build --locked

Write-Host '== Kurulum tamam, hersey yesil ==' -ForegroundColor Green
