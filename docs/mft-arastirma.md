# NTFS MFT Araştırma Notu

> Durum: **araştırıldı, uygulanmadı**. MVP `walkdir` + `rayon` paralel tarama kullanır.
> MFT okuma ileri fazda değerlendirilecek.

## Neden MFT?

Everything uygulamasının 1 saniyenin altında dizin oluşturmasının sırrı, NTFS
birimini dosya dosya gezmek yerine **Master File Table (MFT)** kaydını blok
olarak okumasıdır. `walkdir` yaklaşımı her dosyada en az bir `stat` yapar;
MFT yaklaşımı diskten ardışık okuma yapar.

## Teknik yol

1. `\\.\C:` birimini `CreateFileW` ile `GENERIC_READ` + `FILE_SHARE_*` ile aç.
2. `FSCTL_GET_NTFS_VOLUME_DATA` ile MFT konumunu öğren.
3. `FSCTL_ENUM_USN_DATA` (usn journal) ile dosya kayıtlarını numaralandır.
   Bu, MFT'yi ham parse etmekten daha güvenli ve belgeli yoldur.
4. `USN_RECORD_V2/V3` içinden dosya adı + üst dizin FRN alınır, yol ağacı
   kullanıcı alanında kurulur.
5. Değişiklikler için `FSCTL_READ_USN_JOURNAL` ile artımlı güncelleme
   (`notify` izleyicisinin yerini alır, daha az olay kaçırır).

Gerekli crate'ler: `windows-sys` (yalnızca `Win32_Storage_FileSystem`,
`Win32_Foundation` özellikleri) ya da `winapi`.

## Riskler ve karar

- **Yönetici yetkisi gerekir** (`SE_MANAGE_VOLUME_NAME` olmasa bile ham birim
  okuma genelde yükseltilmiş hak ister). Taşınabilirlik düşer.
- Ham MFT parse edilirse NTFS sürüm farkları (rezident / non-rezident
  öznitelikler, `$FILE_NAME` vs `$STANDARD_INFORMATION`) uzun test ister.
- `FSCTL_ENUM_USN_DATA` daha güvenli ama yine de yalnızca NTFS'te çalışır;
  ReFS/exFAT/FAT32 ve ağ sürücülerinde `walkdir` yedeği şarttır.

Bu yüzden plan: önce `walkdir` MVP kararlı hale gelir, ölçümler
`docs/benchmark.md` içine işlenir; MFT yalnızca ölçüm "yetersiz" derse,
özellik bayrağı (`--mft`) arkasında eklenir.
