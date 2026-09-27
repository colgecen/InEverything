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
- Sağ tık menüsü: aç, dosya konumunu aç, yolu kopyala, dosyayı panoya kopyala, yolu değiştir
- Satır sağında YOLU KOPYALA / DOSYAYI KOPYALA / YOLU DEĞİŞTİR düğmeleri; yolu ve dosyayı panoya kopyalama, dosyayı taşıma
- Klavye kısayolları: Ctrl+I arama, F5 tara, ↑↓/Home/End gezinme, F1 yol kopyala, F2 dosya kopyala, F3/Enter taşı
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

Arama kutusuna yazmaya başlayın; sonuçlar yazarken filtrelenir, ilk satır
otomatik seçilir. Satır **çift tık** ile dosya açılır, **sağ tık** menüsünde
`Aç`, `Dosya konumunu aç`, `Yolu kopyala`, `Dosyayı panoya kopyala` ve
`Yolu değiştir` bulunur. Satır sağındaki **YOLU KOPYALA** yolu panoya,
**DOSYAYI KOPYALA** dosyayı panoya kopyalar, **YOLU DEĞİŞTİR** ise dosyayı
seçilen klasöre taşır. Klavyeden `↑↓` ile gezinip `F1`/`F2`/`F3` ile aynı
işlemler yapılır.

Yapılandırma `~/.config/InEverything/config.json`, indeks
`~/.local/share/InEverything/indeks.bin` altında tutulur.

## Lisans

Bu proje [MIT](LICENSE) lisansı altında lisanslanmıştır.
