//! İndeksleme motoru: sürücü tespiti, paralel tarama ve canlı izleme.
//!
//! MVP bilerek `walkdir` + `rayon` kullanır; NTFS MFT okuma ileri fazda
//! değerlendirilecek (bkz. `docs/mft-arastirma.md`).

use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};

use anyhow::{Context, Result};
use crossbeam_channel::Sender;
use notify::{event::ModifyKind, Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::model::FileItem;

/// Paylaşılan tarama ilerleme bilgisi (arayüz bu sayaçları okur).
#[derive(Debug, Clone, Default)]
pub struct TaramaDurumu {
    /// Tarama tamamen bitti mi?
    pub bitti: Arc<AtomicBool>,
    /// Şimdiye kadar dizine alınan dosya sayısı.
    pub sayi: Arc<std::sync::atomic::AtomicUsize>,
}

/// Var olan sürücü/kök dizinleri döndürür.
/// Windows: `C:\`, `D:\`, ...
/// Linux: `/home`, `/tmp`, `/etc`, `/opt`, `/var`
pub fn suruculeri_bul() -> Vec<PathBuf> {
    #[cfg(target_family = "windows")]
    {
        (b'C'..=b'Z')
            .map(|harf| PathBuf::from(format!("{}:\\", harf as char)))
            .filter(|kok| kok.exists())
            .collect()
    }
    #[cfg(target_family = "unix")]
    {
        vec![
            PathBuf::from("/home"),
            PathBuf::from("/tmp"),
            PathBuf::from("/etc"),
            PathBuf::from("/opt"),
            PathBuf::from("/var"),
        ]
        .into_iter()
        .filter(|kok| kok.exists())
        .collect()
    }
}

/// Verilen köklerin tamamını tara ve her drive'tan dosya buldukça indekse ekle.
/// `iptal` önceden kurulursa boş döner; tarama sırasında kurulursa kalan atlanır.
pub fn sistemi_tara_incele(
    kokler: &[PathBuf],
    durum: &TaramaDurumu,
    iptal: &AtomicBool,
    indeks: Arc<RwLock<Vec<FileItem>>>,
) {
    for kok in kokler {
        if iptal.load(Ordering::Relaxed) {
            break;
        }
        let bulunan: Vec<FileItem> = WalkDir::new(kok)
            .follow_links(false)
            .into_iter()
            .par_bridge()
            .filter_map(|girdi| girdi.ok())
            .filter(|girdi| girdi.file_type().is_file())
            .filter_map(|girdi| {
                if iptal.load(Ordering::Relaxed) {
                    return None;
                }
                let yol = girdi.path().to_path_buf();
                let (boyut, degistirilme) = girdi
                    .metadata()
                    .map(|m| (m.len(), m.modified().ok()))
                    .unwrap_or((0, None));
                durum.sayi.fetch_add(1, Ordering::Relaxed);
                Some(FileItem::dosya(yol, boyut, degistirilme))
            })
            .collect();
        if !bulunan.is_empty() {
            if let Ok(mut kilit) = indeks.write() {
                kilit.extend(bulunan);
            }
        }
    }
    durum.bitti.store(true, Ordering::Relaxed);
}

/// Tek klasör ağacını paralel tara, bulunan dosyaları döndürür.
///
/// Sembolik bağlar izlenmez; okunamayan girdiler sessizce atlanır.
pub fn klasoru_tara(kok: &Path) -> Vec<FileItem> {
    WalkDir::new(kok)
        .follow_links(false)
        .into_iter()
        .par_bridge()
        .filter_map(|girdi| girdi.ok())
        .filter(|girdi| girdi.file_type().is_file())
        .filter_map(|girdi| {
            let yol = girdi.path().to_path_buf();
            let (boyut, degistirilme) = girdi
                .metadata()
                .map(|m| (m.len(), m.modified().ok()))
                .unwrap_or((0, None));
            Some(FileItem::dosya(yol, boyut, degistirilme))
        })
        .collect()
}

/// Verilen köklerin tamamını tara. `iptal` önceden kurulursa boş döner;
/// tarama sırasında kurulursa kalan dosyalar atlanır.
pub fn sistemi_tara(kokler: &[PathBuf], durum: &TaramaDurumu, iptal: &AtomicBool) -> Vec<FileItem> {
    let mut tumu = Vec::new();
    for kok in kokler {
        if iptal.load(Ordering::Relaxed) {
            break;
        }
        let mut bulunan: Vec<FileItem> = WalkDir::new(kok)
            .follow_links(false)
            .into_iter()
            .par_bridge()
            .filter_map(|girdi| girdi.ok())
            .filter(|girdi| girdi.file_type().is_file())
            .filter_map(|girdi| {
                if iptal.load(Ordering::Relaxed) {
                    return None;
                }
                let yol = girdi.path().to_path_buf();
                let (boyut, degistirilme) = girdi
                    .metadata()
                    .map(|m| (m.len(), m.modified().ok()))
                    .unwrap_or((0, None));
                durum.sayi.fetch_add(1, Ordering::Relaxed);
                Some(FileItem::dosya(yol, boyut, degistirilme))
            })
            .collect();
        tumu.append(&mut bulunan);
    }
    durum.bitti.store(true, Ordering::Relaxed);
    tumu
}

/// Dosya sistemi değişikliğinin sadeleştirilmiş hali.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DegisiklikTuru {
    Olustu,
    Silindi,
    YenidenAdlandirildi,
    Diger,
}

/// İzleyiciden arayüze akan değişiklik kaydı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Degisiklik {
    pub yol: PathBuf,
    pub tur: DegisiklikTuru,
}

/// `notify` izleyicisinin sahibi. Yaşadığı sürece olaylar kanala akar.
/// Alan bilerek kullanılmaz; düşürülünce izleme durur.
pub struct CanliIzleyici {
    _izleyici: RecommendedWatcher,
}

/// Verilen kökleri özyinelemeli izlemeye başla, olayları kanala gönder.
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
    use std::sync::atomic::AtomicUsize;

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
        let durum = TaramaDurumu {
            bitti: Arc::new(AtomicBool::new(false)),
            sayi: Arc::new(AtomicUsize::new(0)),
        };
        let iptal = AtomicBool::new(false);
        let kok = dizin.path().to_path_buf();
        let bulunan = sistemi_tara(std::slice::from_ref(&kok), &durum, &iptal);
        assert_eq!(bulunan.len(), 3);
        assert_eq!(durum.sayi.load(Ordering::Relaxed), 3);
        assert!(durum.bitti.load(Ordering::Relaxed));
    }

    #[test]
    fn onceden_iptal_bos_dondurur() {
        let dizin = ornek_agac();
        let durum = TaramaDurumu::default();
        let iptal = AtomicBool::new(true);
        let kok = dizin.path().to_path_buf();
        let bulunan = sistemi_tara(std::slice::from_ref(&kok), &durum, &iptal);
        assert!(bulunan.is_empty());
    }

    #[test]
    fn incele_indeksi_artirir() {
        let dizin = ornek_agac();
        let durum = TaramaDurumu::default();
        let iptal = AtomicBool::new(false);
        let indeks: Arc<RwLock<Vec<FileItem>>> = Arc::new(RwLock::new(Vec::new()));
        let kokler = vec![dizin.path().to_path_buf()];
        sistemi_tara_incele(&kokler, &durum, &iptal, Arc::clone(&indeks));
        assert!(durum.bitti.load(Ordering::Relaxed));
        let okunan = indeks.read().unwrap();
        assert_eq!(okunan.len(), 3);
        drop(okunan);
        assert_eq!(durum.sayi.load(Ordering::Relaxed), 3);
    }
}
