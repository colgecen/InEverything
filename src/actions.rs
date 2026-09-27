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

/// Dosyayı dosya yöneticisindeki Ctrl+C gibi panoya kopyalar.
///
/// Dosya yöneticileri `text/uri-list` ile kopyalanan dosyaları yapıştırır;
/// böylece panoya alınan dosya başka bir klasöre yapıştırılabilir.
pub fn dosyayi_panoya_kopyala(yol: &Path) -> Result<()> {
    let mut pano = arboard::Clipboard::new().context("pano açılamadı")?;
    pano.set()
        .file_list(&[yol])
        .context("dosya panoya yazılamadı")?;
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

/// "Yolu değiştir" işleminin sonucu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TasimaSonucu {
    /// Klasör seçici iptal edildi; hiçbir şey yapılmadı.
    Iptal,
    /// Dosya zaten seçilen klasörde.
    AyniKlasor,
    /// Dosya yeni yola taşındı.
    Tasindi(PathBuf),
}

/// Dosya yöneticisi penceresinde hedef klasör seçtirir.
///
/// İptal edilirse `None` döner. `baslangic` başlangıç klasörüdür (genelde
/// dosyanın bulunduğu klasör); boş verilirse sistem varsayılanı açılır.
pub fn klasor_sec(baslangic: Option<&Path>) -> Option<PathBuf> {
    let istek = rfd::FileDialog::new().set_title("Dosyanın taşınacağı klasörü seç");
    let istek = match baslangic {
        Some(klasor) if !klasor.as_os_str().is_empty() => istek.set_directory(klasor),
        _ => istek,
    };
    istek.pick_folder()
}

/// Dosya yöneticisiyle hedef klasörü seçtirip dosyayı oraya taşır.
pub fn yolu_degistir(kaynak: &Path) -> Result<TasimaSonucu> {
    let baslangic = kaynak.parent();
    let Some(hedef_klasor) = klasor_sec(baslangic) else {
        return Ok(TasimaSonucu::Iptal);
    };
    tasi_hedefe(kaynak, &hedef_klasor)
}

/// Seçilen hedef klasöre taşımanın kurallarını doğrular ve uygular.
///
/// Aynı klasör seçilmişse dosyaya dokunmadan `AyniKlasor` döner; hedefte
/// aynı adlı bir kayıt varsa mevcut veriyi ezmemek için hata verir.
pub fn tasi_hedefe(kaynak: &Path, hedef_klasor: &Path) -> Result<TasimaSonucu> {
    let hedef = hedef_yolu_hesapla(kaynak, hedef_klasor)?;
    if hedef == kaynak {
        return Ok(TasimaSonucu::AyniKlasor);
    }
    if hedef_klasor.starts_with(kaynak) {
        anyhow::bail!("bir klasör kendi içine taşınamaz: {}", kaynak.display());
    }
    if hedef.exists() {
        anyhow::bail!("hedefte aynı adlı kayıt var: {}", hedef.display());
    }
    let yeni = dosyayi_tasi(kaynak, hedef_klasor)?;
    Ok(TasimaSonucu::Tasindi(yeni))
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

    #[test]
    fn ayni_klasor_secilirse_dosya_kalir() {
        let dizin = tempfile::tempdir().expect("geçici dizin");
        let kaynak = dizin.path().join("not.txt");
        std::fs::write(&kaynak, b"ic").expect("yaz");

        let sonuc = tasi_hedefe(&kaynak, dizin.path()).expect("taşı");
        assert_eq!(sonuc, TasimaSonucu::AyniKlasor);
        assert!(kaynak.exists(), "dosya yerinde kalmalı");
    }

    #[test]
    fn hedefte_ayni_ad_varken_ezilmez() {
        let dizin = tempfile::tempdir().expect("geçici dizin");
        let kaynak = dizin.path().join("kaynak");
        let hedef = dizin.path().join("hedef");
        std::fs::create_dir(&hedef).expect("klasör");
        std::fs::write(&kaynak, b"yeni").expect("yaz");
        std::fs::write(hedef.join("kaynak"), b"eski").expect("yaz");

        let hata = tasi_hedefe(&kaynak, &hedef).expect_err("çakışma hatası vermeli");
        assert!(hata.to_string().contains("aynı adlı"));
        assert_eq!(
            std::fs::read(hedef.join("kaynak")).expect("oku"),
            b"eski",
            "mevcut dosya korunmalı"
        );
    }

    #[test]
    fn klasor_kendisine_tasinamaz() {
        let dizin = tempfile::tempdir().expect("geçici dizin");
        let ic = dizin.path().join("ic");
        std::fs::create_dir(&ic).expect("klasör");

        let hata = tasi_hedefe(&ic, &ic).expect_err("kendi içine taşınamaz");
        assert!(hata.to_string().contains("kendi içine"));
        assert!(ic.exists());
    }

    #[test]
    fn secilen_klasore_tasinir() {
        let dizin = tempfile::tempdir().expect("geçici dizin");
        let kaynak = dizin.path().join("tasinacak.txt");
        let hedef = dizin.path().join("yeni_yer");
        std::fs::write(&kaynak, b"veri").expect("yaz");
        std::fs::create_dir(&hedef).expect("klasör");

        let sonuc = tasi_hedefe(&kaynak, &hedef).expect("taşı");
        assert_eq!(sonuc, TasimaSonucu::Tasindi(hedef.join("tasinacak.txt")));
        assert!(!kaynak.exists());
        assert_eq!(
            std::fs::read(hedef.join("tasinacak.txt")).expect("oku"),
            b"veri"
        );
    }
}
