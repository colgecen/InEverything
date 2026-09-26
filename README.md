# InEverything

Everything benzeri ultra hızlı dosya arama uygulaması (Rust + eframe/egui).
Windows'taki Everything, NTFS MFT'yi okuduğu için anlıktır; Linux'ta MFT
eşdeğeri olmadığından burada **kalıcı indeks dosyası** kullanılır: dosyalar
bir kez paralel olarak taranır, sonuç `mmap` ile okunabilen tek bir dosyaya
yazılır ve sonraki tüm açılışlar milisaniye mertebesindedir.

## Özellikler

- **Kalıcı indeks** (`depo.rs`): 40 baytlık sabit uzunluklu kayıtlar + tek
  metin bloğu (özgün yol ve küçük harfli kopya). Aşağıdaki `memmap2` ile
  doğrudan eşlenir; açılışta kopyalama yoktur.
- **Paralel sistem taraması** (`indexer.rs`): `rayon` + iş kuyruğu, hem
  dosya hem klasör kayıtları üretilir; kök/hariç listesi yapılandırmadan
  gelir.
- **Paralel, tahsissiz arama** (`search.rs`): `memchr::memmem` ile ham bayt
  araması, 0-3 puanlama (ad tam / ad öneki / ad içerir / yol içerir),
  `select_nth_unstable` ile en iyi N sonucu. Üstelik arama bir işçi iş parçası
  üzerinde çalışır; arayüz kilitlenmez.
- **Canlı izleme** (`notify`): değişen dosyalar ayrı bir katmanda tutulur,
  20 bin kayıt eşiği aşılırsa indeks yeniden taranır.
- Uzantı filtresi (`*.pdf`), birleşik sorgu (`*.pdf rapor`); canlı izleme
  katmanında bulanık eşleşme (`fuzzy-matcher`).
- Sanallaştırılmış sonuç listesi (varsayılan 10 bin satır).
- Sağ tık: aç, dosya konumunu aç, yolu kopyala.

## Kurulum

Linux (AppImage veya ham binary):

```bash
~/.local/bin/colgecen            # menü: AppImage / exe / macOS / çalıştır
```

Elle:

```bash
cargo build --release
./target/release/InEverything
```

Windows/macOS için aynı `colgecen` menüsündeki ilgili seçenek (`scripts/`
altındaki betikler).

## Kapılar

Her görev bitiminde dördü de yeşil olmalı:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release --locked --all-targets
```

## Dosya yolları

| Dosya | Varsayılan |
| --- | --- |
| Yapılandırma | `~/.config/InEverything/config.json` (eski: exe yanındaki `InEverything-config.json`) |
| İndeks | `~/.local/share/InEverything/indeks.bin` |

```json
{
  "kokler": [],
  "haric": [],
  "indeks_yolu": null,
  "sonuc_limiti": 10000,
  "koyu_tema": true,
  "indeks_yasi_saat": 12,
  "canli_izleme": true
}
```

- `kokler` boşsa varsayılan kök (`/`) kullanılır; `haric` boşsa
  `/proc /sys /dev /run /snap` gibi varsayılan hariçler uygulanır.
- Kök/hariç listesi değişirse imza değişir ve eski indeks "bayat" sayılır;
  uygulama arka planda kendiliğinden yeniden tarar.

## Proje yapısı

```text
src/
  main.rs      ince giriş (app::calistir)
  lib.rs       modül kökü
  depo.rs      kalıcı indeks biçimi, mmap yükleme, tahsissiz arama
  indexer.rs   paralel sistem taraması + notify izleme
  search.rs    eşlaşma puanlama + asenkron görevli + canlı katman
  config.rs    XDG yapılandırması (kök, hariç, indeks yolu, imza)
  model.rs     FileItem, SearchQuery
  actions.rs   dosya işlemleri (aç / konumu aç / yolu kopyala)
  app.rs       egui arayüzü, açılışta indeks yükleme, arka plan yeniden tarama
tests/         uçtan uca tarama + indeks yaz/oku + arama testi
examples/      bench.rs (tarama / yazma / okuma / arama ölçümü)
docs/          mft araştırması, benchmark notları
```

## Ölçüm

`cargo run --release --example bench` tam sistem taraması, indeks yazımı,
`mmap` okuması ve beş sorgu için süre üretir. Güncel sayılar
[`docs/benchmark.md`](docs/benchmark.md) içinde.

Özet (2026-09-26, 2,56 milyon kayıt): açılışta indeks yükleme **55 µs**,
tam tarama **22,1 s**, indeks yazımı **777 ms** (667 MB), arama
**36-63 ms**.
