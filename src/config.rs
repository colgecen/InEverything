//! Uygulama yapılandırması: JSON dosyadan yüklenir/kaydedilir.
//!
//! Ayar ve indeks dosyaları XDG dizinlerinde tutulur (AppImage dâhil her
//! paket için yazılabilir konum); eski sürümün exe yanına yazdığı dosya
//! geriye dönük uyum için okunmaya devam eder.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Kalıcı uygulama ayarları.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Taranacak kökler. Boşsa platform varsayılanı (Linux'ta `/`).
    pub kokler: Vec<PathBuf>,
    /// Taramadan çıkarılacak yollar (`/proc`, `/sys`, ...).
    pub haric: Vec<PathBuf>,
    /// Kalıcı indeks dosyası. Boşsa `~/.local/share/InEverything/indeks.bin`.
    pub indeks_yolu: Option<PathBuf>,
    /// Arayüzde gösterilecek en fazla sonuç satırı.
    pub sonuc_limiti: usize,
    /// Koyu tema varsayılanı.
    pub koyu_tema: bool,
    /// İndeksin en fazla kaç saat sonra yeniden kurulacağı (0 = sadece
    /// ayar değişince).
    pub indeks_yasi_saat: u64,
    /// Dosya değişimlerini canlı izle.
    pub canli_izleme: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            kokler: Vec::new(),
            haric: crate::indexer::VARSAYILAN_HARIC
                .iter()
                .map(PathBuf::from)
                .collect(),
            indeks_yolu: None,
            sonuc_limiti: 10_000,
            koyu_tema: true,
            indeks_yasi_saat: 12,
            canli_izleme: true,
        }
    }
}

impl AppConfig {
    /// Taranacak kökler (boşsa platform varsayılanı).
    pub fn etkin_kokler(&self) -> Vec<PathBuf> {
        if self.kokler.is_empty() {
            crate::indexer::suruculeri_bul()
        } else {
            self.kokler.clone()
        }
    }

    /// Kalıcı indeks dosyasının yolu.
    pub fn indeks_yolu(&self) -> PathBuf {
        self.indeks_yolu
            .clone()
            .unwrap_or_else(|| veri_dizini().join("InEverything").join("indeks.bin"))
    }

    /// Kök ve hariç listelerinden hesaplanan imza.
    ///
    /// Liste değişirse imza değişir ve eski indeks "bayat" sayılır; böylece
    /// kullanıcı ayarı değiştirdiğinde tarama otomatik yenilenir.
    pub fn ayar_imzasi(&self) -> u64 {
        let kokler = self.etkin_kokler();
        let mut imza: u64 = 0xcbf2_9ce4_8422_2325;
        let mut karistir = |veri: &[u8]| {
            for bayt in veri {
                imza ^= u64::from(*bayt);
                imza = imza.wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        for kok in &kokler {
            karistir(yol_baytlarını(kok).as_ref());
            karistir(&[0]);
        }
        karistir(&(kokler.len() as u64).to_le_bytes());
        for haric in &self.haric {
            karistir(yol_baytlarını(haric).as_ref());
            karistir(&[0]);
        }
        imza
    }

    /// Tercih edilen yapılandırma yolu (XDG).
    pub fn varsayilan_yol() -> PathBuf {
        yapilandirma_dizini()
            .join("InEverything")
            .join("config.json")
    }

    /// Geriye dönük uyum: eski sürümün exe yanına yazdığı dosya.
    pub fn eski_yol() -> Option<PathBuf> {
        std::env::current_exe().ok().and_then(|p| {
            p.parent()
                .map(|ust| ust.join("InEverything-config.json"))
                .filter(|y| y.exists())
        })
    }

    /// Yapılandırmayı yükler: önce XDG yolu, sonra eski konum.
    ///
    /// Hiçbiri yoksa varsayılanları döndürür.
    pub fn yukle() -> Self {
        let yeni = Self::varsayilan_yol();
        if let Ok(cfg) = Self::yukle_yoldan(&yeni) {
            return cfg;
        }
        if let Some(eski) = Self::eski_yol() {
            if let Ok(cfg) = Self::yukle_yoldan(&eski) {
                return cfg;
            }
        }
        Self::default()
    }

    /// Belirtilen yoldan yükle; dosya yoksa varsayılanları döndürür.
    pub fn yukle_yoldan(yol: &Path) -> Result<Self> {
        if !yol.exists() {
            return Ok(Self::default());
        }
        let metin = std::fs::read_to_string(yol)
            .with_context(|| format!("yapılandırma okunamadı: {}", yol.display()))?;
        let cfg: Self = serde_json::from_str(&metin)
            .with_context(|| format!("yapılandırma bozuk: {}", yol.display()))?;
        Ok(cfg)
    }

    /// Tercih edilen yola kaydeder.
    pub fn kaydet(&self) -> Result<()> {
        self.kaydet_yola(&Self::varsayilan_yol())
    }

    /// Belirtilan yola güzel baskılı JSON olarak kaydet.
    pub fn kaydet_yola(&self, yol: &Path) -> Result<()> {
        if let Some(ust) = yol.parent() {
            if !ust.as_os_str().is_empty() {
                std::fs::create_dir_all(ust)
                    .with_context(|| format!("klasör açılamadı: {}", ust.display()))?;
            }
        }
        let metin = serde_json::to_string_pretty(self).context("JSON yazımı başarısız")?;
        std::fs::write(yol, metin)
            .with_context(|| format!("yapılandırma yazılamadı: {}", yol.display()))?;
        Ok(())
    }
}

/// `XDG_CONFIG_HOME` (yoksa `~/.config`).
pub fn yapilandirma_dizini() -> PathBuf {
    xdg_dizini("XDG_CONFIG_HOME", ".config")
}

/// `XDG_DATA_HOME` (yoksa `~/.local/share`).
pub fn veri_dizini() -> PathBuf {
    xdg_dizini("XDG_DATA_HOME", ".local/share")
}

fn xdg_dizini(degisken: &str, yedek: &str) -> PathBuf {
    if let Some(deger) = std::env::var_os(degisken) {
        let yol = PathBuf::from(deger);
        if !yol.as_os_str().is_empty() {
            return yol;
        }
    }
    if let Some(ev) = std::env::var_os("HOME") {
        return PathBuf::from(ev).join(yedek);
    }
    PathBuf::from(yedek)
}

fn yol_baytlarını(yol: &Path) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        yol.as_os_str().as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        yol.as_os_str().as_encoded_bytes().to_vec()
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn varsayilan_saglikli_degerler_tasir() {
        let cfg = AppConfig::default();
        assert!(cfg.kokler.is_empty());
        assert!(cfg.sonuc_limiti >= 1000);
        assert!(cfg.koyu_tema);
        assert_eq!(cfg.etkin_kokler(), crate::indexer::suruculeri_bul());
        assert!(cfg.haric.iter().any(|y| y == &PathBuf::from("/proc")));
    }

    #[test]
    fn kaydet_yukle_dongusu_kayipsizdir() {
        let dizin = tempfile::tempdir().expect("geçici dizin");
        let yol = dizin.path().join("alt").join("cfg.json");
        let kaynak = AppConfig {
            kokler: vec![PathBuf::from("/home"), PathBuf::from("/opt")],
            haric: vec![PathBuf::from("/proc")],
            indeks_yolu: Some(PathBuf::from("/tmp/indeks.bin")),
            sonuc_limiti: 500,
            koyu_tema: false,
            indeks_yasi_saat: 6,
            canli_izleme: false,
        };
        kaynak.kaydet_yola(&yol).expect("kaydet");
        let okunan = AppConfig::yukle_yoldan(&yol).expect("yükle");
        assert_eq!(kaynak, okunan);
    }

    #[test]
    fn eski_yapilandirma_eksik_alanlarla_yuklenir() {
        let dizin = tempfile::tempdir().expect("geçici dizin");
        let yol = dizin.path().join("eski.json");
        std::fs::write(&yol, r#"{"kokler":["/home"],"sonuc_limiti":100}"#).expect("yaz");
        let okunan = AppConfig::yukle_yoldan(&yol).expect("yükle");
        assert_eq!(okunan.kokler, vec![PathBuf::from("/home")]);
        assert_eq!(okunan.sonuc_limiti, 100);
        assert!(okunan.canli_izleme, "eksik alanlar varsayılana düşmeli");
    }

    #[test]
    fn olmayan_dosya_varsayilan_dondurur() {
        let dizin = tempfile::tempdir().expect("geçici dizin");
        let okunan = AppConfig::yukle_yoldan(&dizin.path().join("yok.json")).expect("yükle");
        assert_eq!(okunan, AppConfig::default());
    }

    #[test]
    fn ayar_imzasi_liste_degisince_degisir() {
        let temel = AppConfig::default();
        let mut degisik = temel.clone();
        degisik.haric.push(PathBuf::from("/opt/ozel"));
        assert_ne!(temel.ayar_imzasi(), degisik.ayar_imzasi());

        let mut koklu = temel.clone();
        koklu.kokler = vec![PathBuf::from("/home")];
        assert_ne!(temel.ayar_imzasi(), koklu.ayar_imzasi());

        let ayni = temel.clone();
        assert_eq!(temel.ayar_imzasi(), ayni.ayar_imzasi());
    }

    #[test]
    fn indeks_yolu_xdg_altinda() {
        let cfg = AppConfig::default();
        let yol = cfg.indeks_yolu();
        assert!(yol.ends_with("InEverything/indeks.bin"), "{yol:?}");
    }
}
