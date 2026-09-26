use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

use anyhow::{Context, Result};
use crossbeam_channel::Sender;
use notify::{event::ModifyKind, Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{
    depo::{Indeks, KayitYazici},
    model::FileItem,
};

/// Tarama ilerlemesi: kaç kayıt bulundu ve süreç hâlâ çalışıyor mu.
#[derive(Debug, Clone, Default)]
pub struct TaramaDurumu {
    pub bitti: Arc<AtomicBool>,
    pub sayi: Arc<std::sync::atomic::AtomicUsize>,
    /// Bulunan klasör sayısı (yalnızca bilgi amaçlı).
    pub klasor: Arc<std::sync::atomic::AtomicUsize>,
    /// Taranan dizin sayısı.
    pub dizin: Arc<std::sync::atomic::AtomicUsize>,
}

/// Sık gidilen, indekslenmeye değmeyen sistem yolları.
pub const VARSAYILAN_HARIC: [&str; 10] = [
    "/proc",
    "/sys",
    "/dev",
    "/run",
    "/tmp/.X11-unix",
    "/var/lib/docker",
    "/var/lib/flatpak",
    "/var/cache/flatpak",
    "/snap",
    "/var/tmp",
];

/// Sistem kökü: tek satırda her şeyi indeksler.
#[cfg(target_family = "unix")]
pub const VARSAYILAN_KOK: &str = "/";
/// Windows'ta her sabit disk sürücüsü.
#[cfg(target_family = "windows")]
pub const VARSAYILAN_KOK: &str = "C:\\";

/// Taranacak kökleri belirler: boş liste verilirse platform varsayılanı.
pub fn suruculeri_bul() -> Vec<PathBuf> {
    vec![PathBuf::from(VARSAYILAN_KOK)]
}

/// Verilen kökten bir üst dizine ya da köke kadar çıkan yolun, listede
/// geçen bir dizinin altında olup olmadığını söyler.
pub fn haric_mi(yol: &Path, haric: &[PathBuf]) -> bool {
    haric.iter().any(|kok| yol.starts_with(kok))
}

/// Bir yolun bayt halini döndürür (Unix: ham bayt, Windows: kodlanmış bayt).
pub fn yol_baytlari(yol: &Path) -> &[u8] {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        yol.as_os_str().as_bytes()
    }
    #[cfg(not(unix))]
    {
        yol.as_os_str().as_encoded_bytes()
    }
}

/// Tarama sırasında kaç dizin kaldığını tutan ortak sayaç.
struct Kuyruk {
    bekleyen: crossbeam_channel::Sender<PathBuf>,
    alan: crossbeam_channel::Receiver<PathBuf>,
    kalan: Arc<AtomicUsize>,
}

impl Kuyruk {
    fn yeni(kokler: &[PathBuf]) -> Self {
        let (gonder, alan) = crossbeam_channel::unbounded::<PathBuf>();
        let kalan = Arc::new(AtomicUsize::new(0));
        for kok in kokler {
            let _ = gonder.send(kok.clone());
        }
        Self {
            bekleyen: gonder,
            alan,
            kalan,
        }
    }

    fn ekle(&self, yol: PathBuf) {
        let _ = self.bekleyen.send(yol);
    }

    /// Kuyruktan sıradaki dizini alır; iş kalmadıysa `None`.
    ///
    /// Kuyruk boş görünür ama başka bir çalışan hâlâ dizin okuyorsa
    /// (yeni alt dizinler üretebilir) kısa süre bekler. `kalan` yalnızca
    /// o anda kuyrukta/işlemde olan dizin sayısını tutar.
    fn al(&self, iptal: &AtomicBool) -> Option<PathBuf> {
        let mut bos_dene = 0u32;
        loop {
            if iptal.load(Ordering::Relaxed) {
                return None;
            }
            if let Ok(yol) = self.alan.try_recv() {
                self.kalan.fetch_add(1, Ordering::Relaxed);
                return Some(yol);
            }
            // Kuyruk boş: uçuşta iş yoksa bitti.
            if self.kalan.load(Ordering::Acquire) == 0 {
                return None;
            }
            bos_dene += 1;
            if bos_dene < 64 {
                std::hint::spin_loop();
            } else {
                std::thread::yield_now();
            }
        }
    }

    fn isaretle(&self) {
        self.kalan.fetch_sub(1, Ordering::Release);
    }
}

/// Tek bir iş parçacığının taradığı bölüm: kendi kayıt tamponu vardır,
/// böylece hiçbir kilit alınmaz.
struct Parcacik {
    yazici: KayitYazici,
    dizin: Arc<AtomicUsize>,
    durum: TaramaDurumu,
}

/// Bir dizini okur, alt dizinleri kuyruğa ekler, dosyaları kendi tamponuna yazar.
fn isleyici(
    kuyruk: Arc<Kuyruk>,
    parca: Parcacik,
    iptal: Arc<AtomicBool>,
    haric: Arc<Vec<PathBuf>>,
) -> KayitYazici {
    let Parcacik {
        mut yazici,
        dizin,
        durum,
    } = parca;

    while let Some(yol) = kuyruk.al(&iptal) {
        if iptal.load(Ordering::Relaxed) {
            kuyruk.isaretle();
            continue;
        }
        dizin.fetch_add(1, Ordering::Relaxed);
        // Dizinlerin kendisi de sonuçlarda görünsün ("her şeyi" araması).
        if let Ok(meta) = std::fs::metadata(&yol) {
            durum.klasor.fetch_add(1, Ordering::Relaxed);
            yazici.ekle(yol_baytlari(&yol), 0, meta.modified().ok(), true);
        }

        if let Ok(okunan) = std::fs::read_dir(&yol) {
            for girdi in okunan.flatten() {
                if iptal.load(Ordering::Relaxed) {
                    break;
                }
                let Ok(file_type) = girdi.file_type() else {
                    continue;
                };
                let cocuk = girdi.path();
                if file_type.is_dir() {
                    if !haric_mi(&cocuk, &haric) {
                        kuyruk.ekle(cocuk);
                    }
                    continue;
                }
                let Ok(meta) = girdi.metadata() else {
                    continue;
                };
                if file_type.is_file() {
                    durum.sayi.fetch_add(1, Ordering::Relaxed);
                    yazici.ekle(
                        yol_baytlari(&cocuk),
                        meta.len(),
                        meta.modified().ok(),
                        false,
                    );
                } else if file_type.is_symlink() {
                    // Kısayolları da kaydet; boyut/zaman bilgisi hedefin kendisindendir.
                    durum.sayi.fetch_add(1, Ordering::Relaxed);
                    yazici.ekle(
                        yol_baytlari(&cocuk),
                        meta.len(),
                        meta.modified().ok(),
                        false,
                    );
                }
            }
        }

        kuyruk.isaretle();
    }

    yazici
}

/// Tüm çekirdekleri tpkayan ortak kuyruklu tarama; kalıcı indeks üretir.
///
/// İş parçacığı sayısı çekirdek sayısıyla sınırlıdır: tarama tamamen
/// disk/`stat` tarafından sınırlı olduğu için daha fazla iş parçacığı
/// yalnızca bağlam değiştirme maliyeti getirir.
pub fn indeks_tara(
    kokler: &[PathBuf],
    haric: &[PathBuf],
    durum: &TaramaDurumu,
    iptal: &Arc<AtomicBool>,
    ayar_hash: u64,
) -> Indeks {
    let baslangic = std::time::Instant::now();
    let yazici = cok_parallel_tara(kokler, haric, durum, iptal);
    let sure = baslangic.elapsed().as_millis() as u64;
    yazici.indeks_uret(ayar_hash, kokler.len() as u32, sure)
}

fn cok_parallel_tara(
    kokler: &[PathBuf],
    haric: &[PathBuf],
    durum: &TaramaDurumu,
    iptal: &Arc<AtomicBool>,
) -> KayitYazici {
    if kokler.is_empty() {
        return KayitYazici::yeni();
    }

    let cekirdek = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(1, 32);
    let dizin = Arc::new(AtomicUsize::new(0));
    let haric = Arc::new(haric.to_vec());
    let kuyruk = Arc::new(Kuyruk::yeni(kokler));
    let (cikti_tx, cikti_rx) = crossbeam_channel::unbounded::<KayitYazici>();
    let mut hepsi = KayitYazici::yeni();
    let mut ilk = true;

    std::thread::scope(|scope| {
        for _ in 0..cekirdek {
            let kuyruk = Arc::clone(&kuyruk);
            let iptal = Arc::clone(iptal);
            let haric = Arc::clone(&haric);
            let dizin = Arc::clone(&dizin);
            let durum = durum.clone();
            let cikti_tx = cikti_tx.clone();
            scope.spawn(move || {
                let parca = Parcacik {
                    yazici: KayitYazici::yeni(),
                    dizin: Arc::clone(&dizin),
                    durum,
                };
                let sonuc = isleyici(kuyruk, parca, iptal, haric);
                let _ = cikti_tx.send(sonuc);
            });
        }
        drop(cikti_tx);
        for parca in cikti_rx.iter() {
            if ilk {
                // İlk parçacının tamponunu doğrudan devral: kopya yok.
                hepsi = parca;
                ilk = false;
            } else {
                hepsi.birles(parca);
            }
        }
    });

    hepsi
}

/// Tarama bitince indeksi belleğe alan ince sarmalayıcı (testler/eş zamanlı
/// çağrılar için): `bitti` bayrağını da set eder.
pub fn sistemi_tara_indeks(
    kokler: &[PathBuf],
    haric: &[PathBuf],
    durum: &TaramaDurumu,
    iptal: &Arc<AtomicBool>,
    ayar_hash: u64,
) -> Indeks {
    let indeks = indeks_tara(kokler, haric, durum, iptal, ayar_hash);
    durum.bitti.store(true, Ordering::Relaxed);
    indeks
}

/// Tek klasörü özyinelemeli tarar; `iptal` set edilirse tarama durur.
fn dizin_oku(kok: &Path, indeks: &mut Vec<FileItem>, durum: &TaramaDurumu, iptal: &AtomicBool) {
    if iptal.load(Ordering::Relaxed) {
        return;
    }
    let okunan = match std::fs::read_dir(kok) {
        Ok(r) => r,
        Err(_) => return,
    };

    for girdi in okunan {
        if iptal.load(Ordering::Relaxed) {
            return;
        }
        if girdi.is_err() {
            continue;
        }
        let girdi = girdi.unwrap();
        let yol = girdi.path();
        let file_type = match girdi.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        if file_type.is_dir() {
            dizin_oku(&yol, indeks, durum, iptal);
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        let meta = match girdi.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        durum.sayi.fetch_add(1, Ordering::Relaxed);
        indeks.push(FileItem::dosya(yol, meta.len(), meta.modified().ok()));
    }
}

pub fn klasoru_tara(kok: &Path) -> Vec<FileItem> {
    let mut sonuc = Vec::with_capacity(4096);
    dizin_oku(
        kok,
        &mut sonuc,
        &TaramaDurumu::default(),
        &AtomicBool::new(false),
    );
    sonuc
}

pub fn sistemi_tara(
    kokler: &[PathBuf],
    durum: &TaramaDurumu,
    iptal: &Arc<AtomicBool>,
) -> Vec<FileItem> {
    let bulunan = cok_parallel_tara(kokler, &[], durum, iptal);
    durum.bitti.store(true, Ordering::Relaxed);
    hepsi_dosyalara(bulunan)
}

/// Indeks yazıcısındaki kayıtları `FileItem` listesine çevirir.
fn hepsi_dosyalara(yazici: KayitYazici) -> Vec<FileItem> {
    let indeks = yazici.indeks_uret(0, 0, 0);
    let mut sonuc = Vec::with_capacity(indeks.kayit_sayisi());
    for konum in 0..indeks.kayit_sayisi() {
        if let Some(kayit) = indeks.kayit(konum) {
            let yol = PathBuf::from(kayit.yol);
            let oge = if kayit.klasor_mu {
                FileItem::klasor(yol)
            } else {
                FileItem::dosya(yol, kayit.boyut, kayit.degistirilme)
            };
            sonuc.push(oge);
        }
    }
    sonuc
}

#[derive(Clone)]
pub enum DegisiklikTuru {
    Olustu,
    Silindi,
    YenidenAdlandirildi,
    Diger,
}

#[derive(Clone)]
pub struct Degisiklik {
    pub yol: PathBuf,
    pub tur: DegisiklikTuru,
}

pub struct CanliIzleyici {
    _izleyici: RecommendedWatcher,
}

pub fn izlemeyi_baslat(kokler: &[PathBuf], gonder: Sender<Degisiklik>) -> Result<CanliIzleyici> {
    let mut izleyici = RecommendedWatcher::new(
        move |sonuc: std::result::Result<Event, notify::Error>| {
            let olay = match sonuc {
                Ok(olay) => olay,
                Err(_) => return,
            };
            let tur = match &olay.kind {
                notify::EventKind::Create(_) => DegisiklikTuru::Olustu,
                notify::EventKind::Remove(_) => DegisiklikTuru::Silindi,
                notify::EventKind::Modify(ModifyKind::Name(_)) => {
                    DegisiklikTuru::YenidenAdlandirildi
                }
                _ => DegisiklikTuru::Diger,
            };
            for yol in olay.paths {
                let _ = gonder.send(Degisiklik {
                    yol,
                    tur: tur.clone(),
                });
            }
        },
        Config::default(),
    )
    .context("dosya izleyici kurulamadı")?;

    for kok in kokler {
        if kok.exists() {
            izleyici
                .watch(kok, RecursiveMode::Recursive)
                .with_context(|| format!("izlenmedi: {}", kok.display()))?;
        }
    }
    Ok(CanliIzleyici {
        _izleyici: izleyici,
    })
}

#[cfg(test)]
mod testler {
    use super::*;

    fn ornek_agac() -> tempfile::TempDir {
        let dizin = tempfile::tempdir().expect("geçici dizin");
        std::fs::write(dizin.path().join("rapor.pdf"), b"a").expect("yaz");
        std::fs::write(dizin.path().join("NOTLAR.TXT"), b"bb").expect("yaz");
        let alt = dizin.path().join("alt");
        std::fs::create_dir(&alt).expect("klasör");
        std::fs::write(alt.join("foto.png"), b"ccc").expect("yaz");
        dizin
    }

    #[test]
    fn en_az_bir_surucu_bulunur() {
        assert!(!suruculeri_bul().is_empty());
    }

    #[test]
    fn klasor_tarama_tum_dosyalari_bulur() {
        let dizin = ornek_agac();
        let mut adlar: Vec<String> = klasoru_tara(dizin.path())
            .into_iter()
            .map(|o| o.ad.to_lowercase())
            .collect();
        adlar.sort();
        assert_eq!(adlar, vec!["foto.png", "notlar.txt", "rapor.pdf"]);
    }

    #[test]
    fn sistem_tarama_sayaci_dogru_artar() {
        let dizin = ornek_agac();
        let durum = TaramaDurumu::default();
        let iptal = Arc::new(AtomicBool::new(false));
        let kok = dizin.path().to_path_buf();
        let bulunan = sistemi_tara(std::slice::from_ref(&kok), &durum, &iptal);
        assert_eq!(bulunan.len(), 5, "3 dosya + 2 klasör");
        assert_eq!(durum.sayi.load(Ordering::Relaxed), 3);
        assert!(durum.bitti.load(Ordering::Relaxed));
    }

    #[test]
    fn onceden_iptal_bos_dondurur() {
        let dizin = ornek_agac();
        let durum = TaramaDurumu::default();
        let iptal = Arc::new(AtomicBool::new(true));
        let kok = dizin.path().to_path_buf();
        let bulunan = sistemi_tara(std::slice::from_ref(&kok), &durum, &iptal);
        assert!(bulunan.is_empty());
    }

    #[test]
    fn indeks_taramasi_dosya_ve_klasor_kaydeder() {
        let dizin = ornek_agac();
        let durum = TaramaDurumu::default();
        let iptal = Arc::new(AtomicBool::new(false));
        let kokler = vec![dizin.path().to_path_buf()];
        let indeks = sistemi_tara_indeks(&kokler, &[], &durum, &iptal, 1);

        assert_eq!(indeks.kayit_sayisi(), 5, "3 dosya + 2 klasör");
        assert_eq!(durum.sayi.load(Ordering::Relaxed), 3);
        assert_eq!(durum.klasor.load(Ordering::Relaxed), 2);
        assert!(durum.bitti.load(Ordering::Relaxed));

        let adlar: Vec<String> = (0..indeks.kayit_sayisi())
            .filter_map(|i| indeks.kayit(i))
            .map(|k| k.ad.to_lowercase())
            .collect();
        assert!(adlar.contains(&"rapor.pdf".to_string()));
        assert!(adlar.contains(&"alt".to_string()));
    }

    #[test]
    fn haric_klasor_taranmaz() {
        let dizin = ornek_agac();
        let gizli = dizin.path().join("gizli");
        std::fs::create_dir(&gizli).expect("klasör");
        std::fs::write(gizli.join("sir.txt"), b"s").expect("yaz");

        let durum = TaramaDurumu::default();
        let iptal = Arc::new(AtomicBool::new(false));
        let kokler = vec![dizin.path().to_path_buf()];
        let indeks = sistemi_tara_indeks(&kokler, std::slice::from_ref(&gizli), &durum, &iptal, 1);

        let adlar: Vec<String> = (0..indeks.kayit_sayisi())
            .filter_map(|i| indeks.kayit(i))
            .map(|k| k.ad.to_lowercase())
            .collect();
        assert!(!adlar.contains(&"gizli".to_string()));
        assert!(!adlar.contains(&"sir.txt".to_string()));
        assert!(adlar.contains(&"rapor.pdf".to_string()));
    }

    #[test]
    fn taranan_klasor_yollari_dogru_kaydedilir() {
        let dizin = ornek_agac();
        let durum = TaramaDurumu::default();
        let iptal = Arc::new(AtomicBool::new(false));
        let kok = dizin.path().to_path_buf();
        let indeks = sistemi_tara_indeks(std::slice::from_ref(&kok), &[], &durum, &iptal, 1);
        let alt = kok.join("alt");

        let klasorler: Vec<(PathBuf, u64)> = (0..indeks.kayit_sayisi())
            .filter_map(|i| indeks.kayit(i))
            .filter(|k| k.klasor_mu)
            .map(|k| (PathBuf::from(k.yol), k.boyut))
            .collect();
        let mut yollar: Vec<PathBuf> = klasorler.iter().map(|(y, _)| y.clone()).collect();
        yollar.sort();
        assert_eq!(yollar, vec![kok.clone(), alt]);
        assert!(
            klasorler.iter().all(|(_, boyut)| *boyut == 0),
            "klasörler boyutsuz kaydedilmeli"
        );
    }

    #[test]
    fn haric_mi_ust_yollari_da_kapsar() {
        let haric = vec![PathBuf::from("/proc")];
        assert!(haric_mi(Path::new("/proc"), &haric));
        assert!(haric_mi(Path::new("/proc/123"), &haric));
        assert!(!haric_mi(Path::new("/program"), &haric));
        assert!(!haric_mi(Path::new("/home"), &haric));
    }
}
