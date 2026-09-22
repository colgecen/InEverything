//! Uygulama yapılandırması: JSON dosyadan yüklenir/kaydedilir.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Kalıcı uygulama ayarları.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    /// Taranacak kökler. Boşsa sistem sürücüleri otomatik bulunur.
    pub kokler: Vec<PathBuf>,
    /// Arayüzde gösterilecek en fazla sonuç satırı.
    pub sonuc_limiti: usize,
    /// Koyu tema varsayılanı.
    pub koyu_tema: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            kokler: Vec::new(),
            sonuc_limiti: 10_000,
            koyu_tema: true,
        }
    }
}

impl AppConfig {
    /// Çalışan exe yanındaki `fastfind-config.json` yolu.
    pub fn varsayilan_yol() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("fastfind-config.json")
    }

    /// Belirtilen yoldan yükle; dosya yoksa varsayılanları döndür.
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

    /// Belirtilen yola güzel baskılı JSON olarak kaydet.
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

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn varsayilan_saglikli_degerler_tasir() {
        let cfg = AppConfig::default();
        assert!(cfg.kokler.is_empty());
        assert!(cfg.sonuc_limiti >= 1000);
        assert!(cfg.koyu_tema);
    }

    #[test]
    fn kaydet_yukle_dongusu_kayipsizdir() {
        let dizin = tempfile::tempdir().expect("geçici dizin");
        let yol = dizin.path().join("alt").join("cfg.json");
        let kaynak = AppConfig {
            kokler: vec![PathBuf::from("C:\\"), PathBuf::from("D:\\")],
            sonuc_limiti: 500,
            koyu_tema: false,
        };
        kaynak.kaydet_yola(&yol).expect("kaydet");
        let okunan = AppConfig::yukle_yoldan(&yol).expect("yükle");
        assert_eq!(kaynak, okunan);
    }

    #[test]
    fn olmayan_dosya_varsayilan_dondurur() {
        let dizin = tempfile::tempdir().expect("geçici dizin");
        let okunan = AppConfig::yukle_yoldan(&dizin.path().join("yok.json")).expect("yükle");
        assert_eq!(okunan, AppConfig::default());
    }
}
