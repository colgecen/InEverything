# InEverything

Everything benzeri, anlık açılan ultra hızlı dosya arama uygulaması (Rust + eframe/egui).

![InEverything](assets/InEverything-App.png)

## Özellikler

- Kalıcı indeks: dosyalar bir kez paralel taranır, `mmap` ile okunan tek dosyaya yazılır; sonraki açılışlar milisaniye sürer
- Paralel sistem taraması (`rayon` + iş kuyruğu), kök ve hariç listesi yapılandırmadan gelir
- Tahsissiz, kilitlenmeyen arama: `memchr::memmem` ham bayt araması + 0-3 puanlama, en iyi N sonucu `select_nth_unstable` ile seçilir
- Canlı izleme (`notify`): eklenen/silen dosyalar anında yansır, 20 bin değişiklikte indeks otomatik yenilenir
- Uzantı filtresi (`*.pdf`) ve birleşik sorgu (`*.pdf rapor`)
- Sanallaştırılmış sonuç listesi (varsayılan 10 bin satır akıcı)
- Sağ tık menüsü: aç, dosya konumunu aç, yolu kopyala
- Satır sağında YOLU KOPYALA / DOSYAYI KOPYALA / YOLU DEĞİŞTİR düğmeleri; yolu panoya kopyalama, dosyayı dosya yöneticisiyle seçilen klasöre kopyalama ve dosyayı taşıma
- Futuristik neon tema, gömülü logo, Windows/macOS/Linux desteği

## Kurulum

Projeyi yerel ortamınızda çalıştırmak için aşağıdaki adımları takip edin:

```bash
git clone https://github.com/colgecen/InEverything.git
cd InEverything
cargo build --release
```

Hazır ikililer için [sürümler](https://github.com/colgecen/InEverything/releases) sayfasına bakabilirsiniz.

## Kullanım

Projenin nasıl kullanılacağına dair örnek:

```bash
./target/release/InEverything
```

Arama kutusuna yazmaya başlayın; sonuçlar yazarken filtrelenir. Satır
**çift tık** ile dosya açılır, **sağ tık** menüsünde `Aç`,
`Dosya konumunu aç` ve `Yolu kopyala` bulunur. Satır sağındaki
**YOLU KOPYALA** panoya kopyalar, **DOSYAYI KOPYALA** dosya yöneticisi
penceresiyle hedef klasür seçtirip dosyayı oraya kopyalar,
**YOLU DEĞİŞTİR** ise dosyayı seçilen klasöre taşır.

Yapılandırma `~/.config/InEverything/config.json`, indeks
`~/.local/share/InEverything/indeks.bin` altında tutulur.

## Lisans

Bu proje [MIT](LICENSE) lisansı altında lisanslanmıştır.
