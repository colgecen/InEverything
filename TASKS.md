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

## Aşama 9: Uygulama logosu (kapak)

- [x] `assets/InEverything.jpg` kaynak logo olarak depoya alındı; `image`
      (yalnızca `jpeg` özelliği) bağımlılığı eklendi
- [x] `tema.rs`: gömülü JPEG açılışta bir kez çözülür (256×256 RGBA,
      `OnceLock`) → `uygulama_logosu()` pencere ikonunu besler
- [x] `app.rs::calistir()`: `ViewportBuilder::with_icon` ile logo pencere
      ikonu olarak veriliyor (Wayland'de winit ikonu yok saydığı için
      masaüstü girişi ikonu asıl kanaldır)
- [x] `colgecen`: `icon.png` artık `assets/InEverything.jpg`'den 256×256
      üretilir; masaüstü girişi, hicolor ikonu ve AppImage simgesi tek
      kaynaktan gelir (kaynak yoksa eski yedek korunur)
- [x] `gomulu_logo_cozulur` testi: gömülü JPEG çözülemezse kapı düşer
- [x] Kapılar yeşil: `fmt`, `clippy -D warnings`, 67 test, release derlemesi

## Aşama 10: Satır eylem düğmeleri + hedef klasör seçici (2026-09-27)

İstenen: sonuç satırının sağında **kopyalama** ve **yolu değiştir**
düğmeleri; yol seçilirken **dosya yöneticisi** açılıp klasör seçilecek.

### Tamamlanan

- [x] `Cargo.toml`: `rfd = "0.17"` eklendi (Windows/macOS yerel klasör
      seçici, Linux'ta xdg-desktop-portal; `Cargo.lock` güncellendi)
- [x] `actions.rs`:
      - `TasimaSonucu` (`Iptal` / `AyniKlasor` / `Tasindi(PathBuf)`)
      - `klasor_sec(baslangic)` → dosya yöneticisi penceresi (İptal ise `None`)
      - `yolu_degistir(kaynak)` → dosyanın klasöründen başlayıp hedefi seçtirir
      - `tasi_hedefe(kaynak, hedef)` → aynı klasör / hedefte aynı ad /
        klasörü kendi içine taşıma kontrolleri, sonra `dosyayi_tasi`
      - 5 yeni test: aynı klasör, çakışma (ezme yok), kendi içine taşıma,
        başarılı taşıma, `dosyayi_tasi`
- [x] `app.rs` arayüzü:
      - `Eylem::YoluDegistir` + `eylemi_uygula` durum çubuğu mesajları
        (iptal / zaten burada / taşındı / hata)
      - `SatirDugmesi` (`Kopyala`, `YoluDegistir`), `DugmeTanimi`,
        `SatirCizim` (argüman taşması için; clippy `too_many_arguments` geçti)
      - `satir_dugmeleri(alan)` → sağda `KOPYALA` (mor) + `YOLU DEĞİŞTİR`
        (neon) dikdörtgenleri; `satir_dugmesi_ciz` hover'da neon parlar,
        tooltip taşır
      - `satiri_ciz` artık `Option<SatirDugmesi>` döner; düğmeye basılınca
        satır seçimi/çift tık tetiklenmez (egui hit-test düğmeyi üstte alır)
      - sütunlar: `Sutunlar.zaman_goster`, `DUGME_ALANI = 160`,
        `ZAMAN_ESIGI = 800` — dar pencerede "DEĞİŞTİRİLME" kapanır, yeri
        yol/boyut sütununa kalır; klasör satırında "—" yerine "klasör"
      - sütun başlığına **İŞLEM** etiketi; sağ tık menüsüne "Yolu değiştir…";
        alt çubuk ipucu ve boş ekran metni güncellendi
- [x] Yeni testler: `dugmeler_zaman_sutununun_saginda_kalir`,
      `zaman_sutunu_esiginde_kapanir`, `sutunlar_genisle_kaymaz` güncellendi
- [x] Kapılar (şu an yeşil): `cargo fmt --check`,
      `cargo clippy --all-targets -- -D warnings`, `cargo test` → **70 lib +
      3 entegrasyon = 73 test geçti**

### Sıradaki adımlar (yapılacak)

- [ ] `cargo build --release --locked --all-targets` (4. kapı; `Cargo.lock`
      rfd yüzünden değişti, release derlemesi henüz koşulmadı)
- [ ] Uygulamayı çalıştırıp **görsel doğrulama**: satır sağındaki iki düğme
      hizası, hover parıltısı, dar pencerede (<800 px) zaman sütununun
      kapanması, `YOLU DEĞİŞTİR` → dosya yöneticisi penceresi → taşıma
      mesajı (Windows/macOS diyalogları da denenecek)
- [ ] `README.md`: Özellikler'e "satır sağında KOPYALA / YOLU DEĞİŞTİR
      düğmeleri, hedef klasörü dosya yöneticisinden seçme" satırını ekle
- [ ] Kapılar yeşilse commit + push: `feat(satir): kopyala ve yolu değiştir
      düğmeleri, hedef klasör seçici`
- [ ] Sürüm istenirse tag + release (`release.yml` artık tag'de otomatik
      GitHub Release açıp `InEverything.exe` ekliyor)
