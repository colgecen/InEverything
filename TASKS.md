# FastFind Yapılacaklar Listesi

> Her görev bitiminde kapılar koşar: `cargo fmt` + `cargo clippy` + `cargo test` + `cargo build`.
> Yeşilse Türkçe conventional commit + `git push`. Kırmızıysa push yok.

## Aşama 0: Ortam ve İskelet
- [x] Windows kurulum betiğini ekle (`scripts/kurulum-windows.ps1`)
- [x] CI iş akışını ekle (fmt + clippy + test + build)
- [x] Rust proje iskeletini oluştur (`cargo init --bin fastfind`)
- [x] Bağımlılıkları `Cargo.toml` dosyasına ekle

## Aşama 1: Proje Kurulumu ve Mimarisi
- [x] Temel veri yapılarını tanımla (`FileItem`, `SearchQuery`, `AppConfig`)
- [x] Yapılandırma yükleme/kaydetme (`config.rs`)
- [x] Modül iskeletini kur (`app`, `indexer`, `search`, `actions`)

## Aşama 2: İndeksleme Motoru (Core)
- [x] Sistem sürücülerini tespit et (C:\, D:\ vb.)
- [x] Paralel dosya tarama (`walkdir` + `rayon`)
- [x] Canlı dosya izleme (`notify` crate)
- [x] RAM içi indeks optimizasyonu (`Arc<RwLock<Vec>>` + atomik sayaçlar)
- [ ] NTFS MFT araştırmasını belgele (uygulama ileri faza bırakıldı)

## Aşama 3: Arama Motoru
- [x] Tam eşleşme ve uzantı filtreleme (`*.ext`)
- [x] Bulanık arama (`fuzzy-matcher` skim)
- [x] Asenkron arama kanalı (`crossbeam-channel`, UI kilitlenmez)

## Aşama 4: Grafik Kullanıcı Arayüzü (GUI)
- [x] Ana pencere ve koyu tema (eframe/egui)
- [x] Üst arama çubuğu (otomatik odaklı)
- [x] Sanallaştırılmış sonuç listesi (10 bin+ satır akıcı)
- [x] Kolonlar: Ad, Yol, Boyut, Değiştirilme

## Aşama 5: Dosya İşlemleri ve Sağ Tık Menüsü
- [x] Yolu panoya kopyalama (`arboard`)
- [x] Kopyala / taşı (hedef klasörlü)
- [x] Varsayılan uygulamayla aç + konumu aç
- [x] Sağ tık bağlam menüsü

## Aşama 6: Optimizasyon ve Benchmark
- [ ] Bellek ve gecikme ölçüm notları (`docs/benchmark.md`)
- [ ] Başlangıç süresi ve release derlemesi
- [ ] README ve son rötuşlar
