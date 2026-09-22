# FastFind

Everything benzeri ultra hızlı dosya arama ve yönetim uygulaması (Rust +
eframe/egui). Durum: **MVP kaynak kodu tamam**, CI kapısı aktif.

## Özellikler

- Tüm sürücülerde paralel dizin tarama (`walkdir` + `rayon`), canlı izleme
  (`notify`).
- Tam eşleşme, uzantı filtresi (`*.pdf`), birleşik sorgu (`*.pdf rapor`) ve
  bulanık arama (`fuzzy-matcher`).
- Arayüzü kilitlemeyen asenkron arama (`crossbeam-channel`).
- Sanallaştırılmış sonuç listesi (10 bin satır varsayılan limit, ayarlanabilir).
- Sağ tık: aç, dosya konumunu aç, yolu kopyala.

## Gereksinimler

- Rust stable + MSVC linker (Visual Studio Build Tools, "C++ ile masaüstü
  geliştirme").
- Kurumsal proxy arkasında `.exe` indirmeleri engelleniyorsa engelsiz ağ gerekir.

## Kurulum ve çalıştırma

Tek komut (eksikse Rust + Build Tools + crate'leri otomatik kurar):

```powershell
powershell -ExecutionPolicy Bypass -File scripts\calistir-windows.ps1
```

Seçenekler: `-Derle` (çalıştırmadan release derler), `-Test` (önce testleri koşar).

Elle kurulum + kapılar (fmt, clippy, test, build):

# Geliştirme
cargo test
cargo run --release
```

Yapılandırma ilk çalışmada exe yanına `fastfind-config.json` olarak yazılır:

```json
{
  "kokler": [],
  "sonuc_limiti": 10000,
  "koyu_tema": true
}
```

`kokler` boşsa sürücüler otomatik bulunur.

## Proje yapısı

```text
src/
  main.rs      ince giriş (app::calistir)
  lib.rs       modül kökü
  model.rs     FileItem, SearchQuery
  config.rs    JSON yapılandırma
  indexer.rs   sürücü tarama + notify izleme
  search.rs    eşleşme + asenkron görevli
  actions.rs   dosya işlemleri
  app.rs       egui arayüzü
tests/         uçtan uca tarama + arama testi
docs/          mft araştırması, benchmark notları
```

## Kapılar

Her görev bitiminde: `cargo fmt` + `cargo clippy -- -D warnings` +
`cargo test` + `cargo build` yeşilse Türkçe conventional commit + push.
Yerel toolchain yoksa kapı GitHub Actions (`ci`) üzerinde koşar.
