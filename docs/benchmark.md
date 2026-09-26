# Benchmark Notları

Hedefler (sr7t planından):

- 1 milyon dosyada arama gecikmesi **< 10 ms** (indeks bellekteyken).
- Uygulama açılışı + ilk sonuç makul sürede (tam tarama sürücü hızına bağlı).

## Nasıl ölçülür?

```bash
cargo build --release --example bench
./target/release/examples/bench
```

Betik önce `/` kökü üzerinden tam sistem taraması yapar, indeks dosyasına
(`target/bench-indeks.bin`) yazar, geri okur ve beş sorguyu ölçer. Süreler
betiğin stdout'undan alınır.

## Ölçüm (2026-09-26, güncel koşu)

Makine: 8 çekirdek, 15 GB RAM, Fedora 44 (`7.1.10-200.fc44.x86_64`),
tek dosya sistemi `/dev/sdb3` (464 GB, %68 dolu).

```text
kökler: ["/"]
tarama:            22.11s  (2559838 kayıt, 2282683 dosya, 277155 klasör, 0 dizin)
yazma:           776.84ms  (667.2 MB)
okuma:            55.06µs  (açılışta bu kadar sürer)
bellek:         667223956 bayt
arama rapor                       46.99ms  (0 sonuç)
arama *.pdf                       35.93ms  (135 sonuç)
arama belgeler                    62.83ms  (10000 sonuç)
arama cmakelists                  41.75ms  (1227 sonuç)
arama zxyzbulunamaz               40.91ms  (0 sonuç)
```

## İlk ölçüm (2026-09-24, tarihsel)

Makine: 8 çekirdek, 15 GB RAM, Fedora 44 (`7.1.10-200.fc44.x86_64`),
tek dosya sistemi `/dev/sdb3` (464 GB, %68 dolu).

```text
kökler: ["/"]
tarama:            14.41s  (2557726 kayıt, 2280665 dosya, 277061 klasör)
yazma:           771.90ms  (666.6 MB)
okuma:            81.22µs  (açılışta bu kadar sürer)
bellek:         666556830 bayt
arama rapor                       42.40ms  (0 sonuç)
arama *.pdf                       40.10ms  (135 sonuç)
arama belgeler                    67.82ms  (10000 sonuç)
arama cmakelists                  42.63ms  (1227 sonuç)
arama zxyzbulunamaz               37.84ms  (0 sonuç)
```

Soğuk ilk koşuda tarama **28.23 s**, dolan dosya önbelleği ile **14.4 s**
(ilk ölçüm: `okuma` 886 ms idi; `mmap`'e geçince **81 µs**, arama da
100-240 ms'den 38-68 ms'ye indi).

## Yorum

- **Açılış**: indeks dosyası `mmap` ile eşlenir, kopyalama yok → **81 µs**.
  "Her şeyi anında aç" hedefi bu.
- **Arama**: 666 MB'lık indeksin tamamı taranır (bant genişliği sınırlı,
  ~9 GB/s); tek çekirdekte ~300 ms olacak iş 8 çekirdekte ~40 ms.
  `< 10 ms` hedefi yalnızca ön-dizini (trigram/atlas) ile mümkün — bkz.
  "Sonraki adım".
- **Tarama**: 2.5M dosyaya `stat` çekmek fiziksel sınırdır; `find` benzeri
  tek çekirdekli araçların ~2 katı hızdayız (paralel + hem dosya hem klasör
  kaydı).
- **Alan**: kayıt başına 40 B + yolun iki kopyası (özgün + küçük harfli)
  ≈ 260 B/kayıt → 2.5M kayıt ≈ 667 MB. İkinci kopya arama için gereklidir.

## Sonraki adım (Aşama 6, gerekiyorsa)

`< 10 ms` hedefi için kayda dayalı trigram indeksi (Everything/mlocate
tarzı): 3 baytlık parçaların konum listeleri ayrı bir dosyada, arama önce
trigram adaylarını indirger. Maliyet: ikinci bir indeks dosyası ve karmaşık
yazıcı; ölçümler "40 ms yetersiz" çıkarsa başlar.
