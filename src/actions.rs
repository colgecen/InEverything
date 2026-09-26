//! Dosya işlemleri: panoya kopyalama, açma, konumu gösterme, kopyala/taşı.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// `kaynak` dosyasının `hedef_klasor` içindeki karşılığını hesaplar.
///
/// Ayırıcı olarak `/` ve `\` birlikte kabul edilir; böylece Windows biçiminde
/// verilmiş yollar Linux'ta da doğru çözülür.
pub fn hedef_yolu_hesapla(kaynak: &Path, hedef_klasor: &Path) -> Result<PathBuf> {
    let metin = kaynak.to_string_lossy();
    let ad = metin
        .rfind(['/', '\\'])
        .map(|i| &metin[i + 1..])
        .unwrap_or(metin.as_ref());
    if ad.is_empty() {
        anyhow::bail!("dosya adı yok: {}", kaynak.display());
    }
    Ok(hedef_klasor.join(ad))
}

/// Dosya yolunu metin olarak panoya kopyalar.
pub fn panoya_yolu_kopyala(yol: &Path) -> Result<()> {
    let mut pano = arboard::Clipboard::new().context("pano açılamadı")?;
    pano.set_text(yol.display().to_string())
        .context("yol panoya yazılamadı")?;
    Ok(())
}

/// Dosyayı varsayılan uygulamayla açar.
pub fn dosyayi_ac(yol: &Path) -> Result<()> {
    open::that(yol).with_context(|| format!("açılamadı: {}", yol.display()))?;
    Ok(())
}

/// Dosyanın bulunduğu klasörü dosya yöneticisinde açar.
///
/// Windows'ta dosya seçili gelir; diğer platformlarda klasör açılır.
pub fn konumu_ac(yol: &Path) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let durum = std::process::Command::new("explorer")
            .arg("/select,")
            .arg(yol)
            .status()
            .context("explorer başlatılamadı")?;
        if durum.success() {
            Ok(())
        } else {
            anyhow::bail!("konum açılamadı: {}", yol.display());
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let ust = yol
            .parent()
            .filter(|ust| !ust.as_os_str().is_empty())
            .with_context(|| format!("üst klasör yok: {}", yol.display()))?;
        open::that(ust).with_context(|| format!("klasör açılamadı: {}", ust.display()))?;
        Ok(())
    }
}

/// Dosyayı hedef klasöre kopyalar, yeni yolu döndürür.
pub fn dosyayi_kopyala(kaynak: &Path, hedef_klasor: &Path) -> Result<PathBuf> {
    let hedef = hedef_yolu_hesapla(kaynak, hedef_klasor)?;
    std::fs::copy(kaynak, &hedef)
        .with_context(|| format!("kopyalanamadı: {} -> {}", kaynak.display(), hedef.display()))?;
    Ok(hedef)
}

/// Dosyayı hedef klasöre taşır, yeni yolu döndürür.
pub fn dosyayi_tasi(kaynak: &Path, hedef_klasor: &Path) -> Result<PathBuf> {
    let hedef = hedef_yolu_hesapla(kaynak, hedef_klasor)?;
    std::fs::rename(kaynak, &hedef)
        .with_context(|| format!("taşınamadı: {} -> {}", kaynak.display(), hedef.display()))?;
    Ok(hedef)
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn hedef_yol_dosya_adini_birlestirir() {
        let hedef =
            hedef_yolu_hesapla(Path::new("C:\\A\\rapor.pdf"), Path::new("D:\\Yedek")).expect("yol");
        // Ayırıcı platforma göre değişir; önemli olan doğru dosya adının
        // hedef klasörün altına eklenmesi.
        assert_eq!(hedef, PathBuf::from("D:\\Yedek").join("rapor.pdf"));
        assert_eq!(hedef.file_name(), Some(OsStr::new("rapor.pdf")));
    }

    #[test]
    fn unix_yolu_da_cozulur() {
        let hedef =
            hedef_yolu_hesapla(Path::new("/home/x/not.txt"), Path::new("/yedek")).expect("yol");
        assert_eq!(hedef, PathBuf::from("/yedek/not.txt"));
    }

    #[test]
    fn dosya_adi_yoksa_hata_verir() {
        assert!(hedef_yolu_hesapla(Path::new("/home/x/"), Path::new("/yedek")).is_err());
    }

    #[test]
    fn kopyala_icerigi_aynen_tasir() {
        let dizin = tempfile::tempdir().expect("geçici dizin");
        let kaynak = dizin.path().join("kaynak.txt");
        let hedef_klasor = dizin.path().join("hedef");
        std::fs::create_dir(&hedef_klasor).expect("klasör");
        std::fs::write(&kaynak, b"merhaba").expect("yaz");
        let yeni = dosyayi_kopyala(&kaynak, &hedef_klasor).expect("kopyala");
        assert!(kaynak.exists());
        assert_eq!(std::fs::read(&yeni).expect("oku"), b"merhaba");
    }

    #[test]
    fn tasi_kaynagi_kaldirir() {
        let dizin = tempfile::tempdir().expect("geçici dizin");
        let kaynak = dizin.path().join("tasinacak.txt");
        let hedef_klasor = dizin.path().join("hedef");
        std::fs::create_dir(&hedef_klasor).expect("klasör");
        std::fs::write(&kaynak, b"veri").expect("yaz");
        let yeni = dosyayi_tasi(&kaynak, &hedef_klasor).expect("taşı");
        assert!(!kaynak.exists());
        assert_eq!(std::fs::read(&yeni).expect("oku"), b"veri");
    }
}
