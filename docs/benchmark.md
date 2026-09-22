# Benchmark Notları

Hedefler (sr7t planından):

- 1 milyon dosyada arama gecikmesi **< 10 ms** (indeks bellekteyken).
- Uygulama açılışı + ilk sonuç makul sürede (tam tarama sürücü hızına bağlı).

## Nasıl ölçülür?

Release derlemesiyle (MFT yok, `walkdir` + `rayon`):

```powershell
cargo build --release
$exe = ".\target\release\fastfind.exe"
Measure-Command { Start-Process $exe -PassThru | Wait-Process }
```

Arama gecikmesi için: sorgu yazıldıktan sonra sonuçların boyanması gözle
ölçülür; ileride `search::ara` etrafına `Instant` ölçümü eklenip
`--bench` çıktısı alınacak.

## Bellek kaba hesabı

Kayıt başına: yol (~130 bayt) + ad + `FileItem` iskeleti ≈ 200-300 bayt.
1 milyon dosya ≈ **250-350 MB RAM**. Bu MVP için kabul edilebilir sınırın
üstüdür; düşürme yolları:

1. Yolları tek arena `String` havuzunda tutup kayıtta `(u32, u32)` dilim
   saklamak.
2. Boyut + zamanı 64 bit yerine 32 bite indirmek (2038 sonrası için zamanı
   `u32` gün çözünürlüğünde tutmak).
3. Sıkıştırılmış dizi (`rkyv` / `zerocopy`) ve bellek eşlemeli dosya.

Bu optimizasyonlar, yukarıdaki ölçüm "yetersiz" çıkarsa Aşama 6 devam
işi olarak `TASKS.md` altına eklenecek.
