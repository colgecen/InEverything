//! Dosya işlemleri: panoya kopyalama, açma, konumu gösterme, kopyala/taşı.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// `kaynak` dosyasının `hedef_klasor` içindeki karşılığını hesaplar.
pub fn hedef_yolu_hesapla(kaynak: &Path, hedef_klasor: &Path) -> Result<PathBuf> {
    let ad = kaynak
        .file_name()
        .with_context(|| format!("dosya adı yok: {}", kaynak.display()))?;
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

/// Windows Gezgini'nde dosyanın konumunu seçili gösterir.
pub fn konumu_ac(yol: &Path) -> Result<()> {
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

/// Dosyayı hedef klasöre kopyalar, yeni yolu döndürür.
pub fn dosyayi_kopyala(kaynak: &Path, hedef_klasor: &Path) -> Result<PathBuf> {
    let hedef = hedef_yolu_hesapla(kaynak, hedef_klasor)?;
    std::fs::copy(kaynak, &hedef).with_context(|| {
        format!(
            "kopyalanamadı: {} -> {}",
            kaynak.display(),
            hedef.display()
        )
    })?;
    Ok(hedef)
}

/// Dosyayı hedef klasöre taşır, yeni yolu döndürür.
pub fn dosyayi_tasi(kaynak: &Path, hedef_klasor: &Path) -> Result<PathBuf> {
    let hedef = hedef_yolu_hesapla(kaynak, hedef_klasor)?;
    std::fs::rename(kaynak, &hedef).with_context(|| {
        format!("taşınamadı: {} -> {}", kaynak.display(), hedef.display())
    })?;
    Ok(hedef)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn hedef_yol_dosya_adini_birlestirir() {
        let hedef =
            hedef_yolu_hesapla(Path::new("C:\\A\\rapor.pdf"), Path::new("D:\\Yedek")).expect("yol");
        assert_eq!(hedef, PathBuf::from("D:\\Yedek\\rapor.pdf"));
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
