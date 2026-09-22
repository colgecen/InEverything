//! Temel veri yapıları: dizine alınan dosya kaydı ve arama sorgusu.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Bellek içi indekste tutulan tek dosya/klasör kaydı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileItem {
    /// Uzantı dahil dosya adı (örn. `rapor.pdf`).
    pub ad: String,
    /// Tam yol (örn. `C:\Belgeler\rapor.pdf`).
    pub yol: PathBuf,
    /// Bayt cinsinden boyut (klasörlerde 0).
    pub boyut: u64,
    /// Son değiştirilme zamanı (okunamazsa `None`).
    pub degistirilme: Option<SystemTime>,
    /// `true` ise kayıt bir klasörü temsil eder.
    pub klasor_mu: bool,
}

impl FileItem {
    /// Sıradan bir dosya kaydı oluşturur.
    pub fn dosya(yol: PathBuf, boyut: u64, degistirilme: Option<SystemTime>) -> Self {
        let ad = yol
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self {
            ad,
            yol,
            boyut,
            degistirilme,
            klasor_mu: false,
        }
    }

    /// Klasör kaydı oluşturur.
    pub fn klasor(yol: PathBuf) -> Self {
        let ad = yol
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self {
            ad,
            yol,
            boyut: 0,
            degistirilme: None,
            klasor_mu: false,
        }
    }

    /// Küçük harfli uzantıyı noktasız döndürür (`pdf`), yoksa `None`.
    pub fn uzanti(&self) -> Option<String> {
        Path::new(&self.ad)
            .extension()
            .map(|s| s.to_string_lossy().to_lowercase())
    }
}

/// Kullanıcının yazdığı ham metnin çözümlenmiş hali.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    /// Ham metin (kırpılmış).
    pub ham: String,
    /// Küçük harfli hali (eşleşmede kullanılır).
    pub kucuk: String,
    /// `*.pdf` / `.pdf` yazıldıysa noktasız küçük harfli uzantı.
    pub uzanti_filtresi: Option<String>,
}

impl SearchQuery {
    /// Ham metni çözümle. Boş sorgu boş filtre demektir.
    pub fn cozumle(ham: &str) -> Self {
        let kirpilmis = ham.trim().to_string();
        let kucuk = kirpilmis.to_lowercase();
        let uzanti_filtresi = if let Some(kalan) = kucuk.strip_prefix("*.") {
            Self::temiz_uzanti(kalan)
        } else if kucuk.starts_with('.')
            && !kucuk.contains([' ', '*', '/', '\\'])
            && kucuk.len() > 1
        {
            Self::temiz_uzanti(&kucuk[1..])
        } else {
            None
        };
        Self {
            ham: kirpilmis,
            kucuk,
            uzanti_filtresi,
        }
    }

    fn temiz_uzanti(kalan: &str) -> Option<String> {
        let temiz = kalan.trim_matches('.').trim().to_lowercase();
        if temiz.is_empty() || temiz.contains([' ', '*', '/', '\\']) {
            None
        } else {
            Some(temiz)
        }
    }

    /// Sorgu boş mu (listeyi doldurmaya gerek yok)?
    pub fn bos_mu(&self) -> bool {
        self.kucuk.is_empty()
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn uzanti_kucuk_harfe_cevrilir() {
        let oge = FileItem::dosya(PathBuf::from("C:\\A\\RAPOR.PDF"), 10, None);
        assert_eq!(oge.uzanti().as_deref(), Some("pdf"));
    }

    #[test]
    fn uzantisiz_dosya_none_dondurur() {
        let oge = FileItem::dosya(PathBuf::from("C:\\A\\Makefile"), 10, None);
        assert_eq!(oge.uzanti(), None);
    }

    #[test]
    fn yildizli_uzanti_filtresi_cozulur() {
        let sorgu = SearchQuery::cozumle("*.png");
        assert_eq!(sorgu.uzanti_filtresi.as_deref(), Some("png"));
    }

    #[test]
    fn noktali_uzanti_filtresi_cozulur() {
        let sorgu = SearchQuery::cozumle(".PDF");
        assert_eq!(sorgu.uzanti_filtresi.as_deref(), Some("pdf"));
    }

    #[test]
    fn duz_metin_filtresiz_cozulur() {
        let sorgu = SearchQuery::cozumle("rapor 2026");
        assert_eq!(sorgu.uzanti_filtresi, None);
        assert!(!sorgu.bos_mu());
    }

    #[test]
    fn bos_sorgu_bos_sayilir() {
        assert!(SearchQuery::cozumle("   ").bos_mu());
    }
}
