# InEverything Yapılacaklar Listesi

> Her görev bitiminde kapılar koşar: `cargo fmt` + `cargo clippy` + `cargo test` + `cargo build`.
> Yeşilse Türkçe conventional commit + `git push`. Kırmızıysa push yok.

## Aşama 0: Ortam ve İskelet
- [x] Windows kurulum betiğini ekle (`scripts/kurulum-windows.ps1`)
- [x] Çalıştırınca bağımlılıkları otomatik kuran betiği ekle (`scripts/calistir-windows.ps1`)
- [x] CI iş akışını ekle (fmt + clippy + test + build)
- [x] Rust proje iskeletini oluştur (`cargo init --bin ineverything`)
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
- [x] NTFS MFT araştırmasını belgele (uygulama ileri faza bırakıldı)

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
- [x] Bellek ve gecikme ölçüm notları (`docs/benchmark.md`)
- [x] Başlangıç süresi ve release derlemesi (release profili tanımlı)
- [x] README ve son rötuşlar
- [x] `lto`/`codegen-units` son birim tıkanmasını kaldır (1m27s → 0.7s)

## Aşama 7: Kalıcı İndeks (Everything benzeri anlık açılış)

Linux'ta NTFS MFT eşdeğeri olmadığından tek yolumuz diske yazılmış indeks.

- [x] `depo.rs`: 40 B sabit uzunluklu kayıt + metin bloğu, atomik yazma
      (`.tmp` + `rename`), sürüm/başlık doğrulaması
- [x] `memmap2` ile açılışta `mmap` (886 ms → **81 µs**)
- [x] `indexer.rs`: iş kuyruklu paralel tarama, hem dosya hem klasör kaydı,
      kök/hariç varsayılanları, tarama durumu sayaçları
- [x] `search.rs`: tahsissiz paralel arama, 0-3 puanlama,
      `select_nth_unstable` ile en iyi N (38-68 ms / 2.5M kayıt)
- [x] Canlı katman: silinen/eklenen, 20 bin eşikte tam yeniden tarama
- [x] `config.rs`: XDG yolları, kök/hariç/indeks yolu, bayatlık imzası
- [x] `app.rs`: açılışta indeks yükleme, arka planda bayat tarama,
      `nesil` ile sonuç yenileme, "Yeniden Tara", durum çubuğu
- [x] `colgecen` betiği: log (`target/paketleme.log`), hata yayılımı,
      AppImage başarısızsa ham binary'ye düşme, "sadece çalıştır" seçeneği
- [x] Ölçümler (`examples/bench.rs`), README + `docs/benchmark.md`
- [x] Uygulamayı çalıştırıp görsel doğrulama + AppImage üretimi
- [x] Uygulama adı `fastfind` → **InEverything** (paket, ikili, pencere başlığı,
      XDG yolları, README/docs, CI, Windows betikleri, `colgecen`)
- [x] Arayüz hataları: çift `ms` düzeltildi, "Yeniden Tara" düğmesi için
      genişlik ayrıldı (metin kutusu tüm satırı kaplıyordu)

## Aşama 8: Futuristik arayüz (tema)

- [x] `tema.rs`: renk paleti (camgöbeği/mor/pembe neon), font kurulumu
      (Hack gömülü normal, kalın ağırlık sistemden okunur, bulunamazsa gömülür),
      `Visuals`/`Spacing` (koyu widget katmanları, ince neon kaydırma çubuğu)
- [x] Neon çizim yardımcıları: gradyan + ızgara arka plan, tarama çizgisi,
      katmanlı parıltılı çerçeve, dört yönlü parıltılı metin, ışıklı nokta,
      bilgi çipleri, ölçüye göre metin kısaltma
- [x] Başlık satırı: elmas logo, harf aralıklı `I N E V E R Y T H I N G`,
      kayıt/boyut/tarama çipleri, altında koşan neon ışın çizgisi
- [x] Arama kutusu: odakta parıltı, büyüteç imlesi, neon "Yeniden Tara" çipi
- [x] Sütun başlıkları (ADI/KONUM/BOYUT/DEĞİŞTİRİLME) ve neon satırlar:
      uzantıya göre renkli işaret, hover/seçim vurgusu, sağa hizalı sayılar
- [x] Boş ekran: büyük başlık + tıklanabilir örnek sorgu çipleri
- [x] Alt çubuk: canlı durum noktası, kısayol ipuçları
- [x] Kapılar yeşil: `fmt`, `clippy -D warnings`, 66 test, release derlemesi
- [x] Görsel doğrulama: dolu liste, boş ekran ve odakta arama kutusu ekran
      görüntüleriyle kontrol edildi
