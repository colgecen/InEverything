//! İki dilli arayüz metinleri: Türkçe / İngilizce.
//!
//! Dil değişimi anlık olur; tüm yazılar her karede `Dil` değerinden okunur,
//! bu yüzden düğmeye basınca yeniden başlatmaya gerek kalmaz. Kalıcılık
//! `AppConfig::dil` üzerinden sağlanır.

use serde::{Deserialize, Serialize};

/// Arayüz dili.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dil {
    /// Türkçe (varsayılan).
    #[default]
    #[serde(rename = "tr")]
    Turkce,
    /// İngilizce.
    #[serde(rename = "en")]
    Ingilizce,
}

impl Dil {
    /// Diğer dile geçer (yuvarlak düğmenin işi).
    #[must_use]
    pub fn diger(self) -> Self {
        match self {
            Self::Turkce => Self::Ingilizce,
            Self::Ingilizce => Self::Turkce,
        }
    }

    /// Kısaltma (`tr` / `en`).
    #[must_use]
    pub fn kodu(self) -> &'static str {
        match self {
            Self::Turkce => "tr",
            Self::Ingilizce => "en",
        }
    }

    /// Koddan dile çevirir; bilinmeyende Türkçe döner.
    #[must_use]
    pub fn koddan(kod: &str) -> Self {
        match kod.trim().to_ascii_lowercase().as_str() {
            "en" | "en-us" | "en-gb" | "ingilizce" | "english" => Self::Ingilizce,
            _ => Self::Turkce,
        }
    }

    /// Yuvarlak düğmede görünen yazı: Türkçe iken `EN`, İngilizce iken `TR`.
    #[must_use]
    pub fn dugme_etiket(self) -> &'static str {
        match self {
            Self::Turkce => "EN",
            Self::Ingilizce => "TR",
        }
    }

    /// Yuvarlak düğmenin ipucu.
    #[must_use]
    pub fn dugme_ipucu(self) -> &'static str {
        match self {
            Self::Turkce => "İngilizceye geç (Switch to English)",
            Self::Ingilizce => "Türkçeye geç (Switch to Turkish)",
        }
    }

    /// Başlık altındaki slogan.
    #[must_use]
    pub fn alt_baslik(self) -> &'static str {
        match self {
            Self::Turkce => "ULTRA HIZLI DOSYA ARAMA MOTORU",
            Self::Ingilizce => "ULTRA FAST FILE SEARCH ENGINE",
        }
    }

    /// Arama kutusu ipucu.
    #[must_use]
    pub fn arama_ipucu(self) -> &'static str {
        match self {
            Self::Turkce => "dosya ara — rapor.pdf, *.png, *belgeler* …",
            Self::Ingilizce => "search files — report.pdf, *.png, *documents* …",
        }
    }

    /// "Yeniden tara" düğmesi.
    #[must_use]
    pub fn yeniden_tara(self) -> &'static str {
        match self {
            Self::Turkce => "⟳  YENİDEN TARA",
            Self::Ingilizce => "⟳  RESCAN",
        }
    }

    /// Tarama sürerken düğmede görünen yazı.
    #[must_use]
    pub fn taraniyor_etiket(self) -> &'static str {
        match self {
            Self::Turkce => "TARANIYOR …",
            Self::Ingilizce => "SCANNING …",
        }
    }

    /// Tarama düğmesi ipucu.
    #[must_use]
    pub fn tarama_sürüyor(self) -> &'static str {
        match self {
            Self::Turkce => "Tarama sürüyor",
            Self::Ingilizce => "Scan in progress",
        }
    }

    /// Üst durum satırı (tarama sürerken).
    #[must_use]
    pub fn taraniyor_durum(self, dosya: usize, klasor: usize, dizin: usize) -> String {
        match self {
            Self::Turkce => format!("TARANIYOR · {dosya} dosya · {klasor} klasör · {dizin} dizin"),
            Self::Ingilizce => {
                format!("SCANNING · {dosya} files · {klasor} folders · {dizin} dirs")
            }
        }
    }

    /// Üst durum satırı (indeks hazır).
    #[must_use]
    pub fn indeks_hazir(self, kayit: usize, sonuc: usize) -> String {
        match self {
            Self::Turkce => format!("INDEKS HAZIR · {kayit} kayıt · sonuç {sonuc}"),
            Self::Ingilizce => format!("INDEX READY · {kayit} entries · {sonuc} results"),
        }
    }

    /// Canlı izleme çipleri.
    #[must_use]
    pub fn canli_acik(self) -> &'static str {
        match self {
            Self::Turkce => "CANLI İZLEME",
            Self::Ingilizce => "LIVE WATCH",
        }
    }

    /// Canlı izleme kapalıyken.
    #[must_use]
    pub fn canli_kapali(self) -> &'static str {
        match self {
            Self::Turkce => "CANLI KAPALI",
            Self::Ingilizce => "LIVE OFF",
        }
    }

    /// Başlıktaki kayıt çipi.
    #[must_use]
    pub fn kayit_cipi(self, kayit: usize) -> String {
        match self {
            Self::Turkce => format!("{kayit} KAYIT"),
            Self::Ingilizce => format!("{kayit} ENTRIES"),
        }
    }

    /// Başlıktaki son tarama çipi.
    #[must_use]
    pub fn son_tarama(self, sure: &str) -> String {
        match self {
            Self::Turkce => format!("SON TARAMA {sure}"),
            Self::Ingilizce => format!("LAST SCAN {sure}"),
        }
    }

    /// Sütun başlıkları.
    #[must_use]
    pub fn sutun_dosya(self) -> &'static str {
        match self {
            Self::Turkce => "DOSYA ADI",
            Self::Ingilizce => "FILE NAME",
        }
    }

    /// Sütun başlığı: konum.
    #[must_use]
    pub fn sutun_konum(self) -> &'static str {
        match self {
            Self::Turkce => "KONUM",
            Self::Ingilizce => "LOCATION",
        }
    }

    /// Sütun başlığı: boyut.
    #[must_use]
    pub fn sutun_boyut(self) -> &'static str {
        match self {
            Self::Turkce => "BOYUT",
            Self::Ingilizce => "SIZE",
        }
    }

    /// Sütun başlığı: değiştirilme.
    #[must_use]
    pub fn sutun_zaman(self) -> &'static str {
        match self {
            Self::Turkce => "DEĞİŞTİRİLME",
            Self::Ingilizce => "MODIFIED",
        }
    }

    /// Sütun başlığı: işlem.
    #[must_use]
    pub fn sutun_islem(self) -> &'static str {
        match self {
            Self::Turkce => "İŞLEM",
            Self::Ingilizce => "ACTION",
        }
    }

    /// Boş ekrandaki örnek sorgu çipleri.
    #[must_use]
    pub fn ornek_sorgular(self) -> [&'static str; 5] {
        match self {
            Self::Turkce => ["*.pdf", "*.png", "rapor", "belgeler", "cmakelists.txt"],
            Self::Ingilizce => ["*.pdf", "*.png", "report", "documents", "cmakelists.txt"],
        }
    }

    /// Boş ekrandaki açıklama.
    #[must_use]
    pub fn bos_aciklama(self) -> &'static str {
        match self {
            Self::Turkce => "aramaya başlamak için yukarıya yazın",
            Self::Ingilizce => "type above to start searching",
        }
    }

    /// Boş ekrandaki kısayol ipucu.
    #[must_use]
    pub fn bos_ipucu(self) -> &'static str {
        match self {
            Self::Turkce => {
                "↑↓ ile gez · F1 yol · F2 dosya · F3 taşı · veya sağdaki düğmeleri tıklayın"
            }
            Self::Ingilizce => {
                "↑↓ navigate · F1 path · F2 file · F3 move · or click the buttons on the right"
            }
        }
    }

    /// Sonuç yok başlığı.
    #[must_use]
    pub fn sonuc_yok(self) -> &'static str {
        match self {
            Self::Turkce => "SONUÇ YOK",
            Self::Ingilizce => "NO RESULTS",
        }
    }

    /// Sonuç yok açıklaması.
    #[must_use]
    pub fn sonuc_yok_aciklama(self, sorgu: &str) -> String {
        match self {
            Self::Turkce => format!("«{sorgu}» için eşleşme bulunamadı"),
            Self::Ingilizce => format!("no match for «{sorgu}»"),
        }
    }

    /// Alt çubuktaki kısayollar.
    #[must_use]
    pub fn alt_kisayollar(self) -> &'static str {
        match self {
            Self::Turkce => "Ctrl+I ara · F5 tara · ↑↓ gez · F1 yol · F2 dosya · F3/Enter taşı",
            Self::Ingilizce => {
                "Ctrl+I search · F5 scan · ↑↓ navigate · F1 path · F2 file · F3/Enter move"
            }
        }
    }

    /// Alt çubuktaki sonuç sayacı.
    #[must_use]
    pub fn sonuc_sayisi(self, toplam: usize) -> String {
        match self {
            Self::Turkce => format!("{toplam} sonuç"),
            Self::Ingilizce => format!("{toplam} results"),
        }
    }

    /// Sağ tık menüsü: aç.
    #[must_use]
    pub fn menu_ac(self) -> &'static str {
        match self {
            Self::Turkce => "Aç",
            Self::Ingilizce => "Open",
        }
    }

    /// Sağ tık menüsü: konumu aç.
    #[must_use]
    pub fn menu_konum(self) -> &'static str {
        match self {
            Self::Turkce => "Dosya konumunu aç",
            Self::Ingilizce => "Open file location",
        }
    }

    /// Sağ tık menüsü: yolu kopyala.
    #[must_use]
    pub fn menu_yol(self) -> &'static str {
        match self {
            Self::Turkce => "Yolu kopyala",
            Self::Ingilizce => "Copy path",
        }
    }

    /// Sağ tık menüsü: dosyayı kopyala.
    #[must_use]
    pub fn menu_dosya(self) -> &'static str {
        match self {
            Self::Turkce => "Dosyayı panoya kopyala",
            Self::Ingilizce => "Copy file to clipboard",
        }
    }

    /// Sağ tık menüsü: yolu değiştir.
    #[must_use]
    pub fn menu_tasi(self) -> &'static str {
        match self {
            Self::Turkce => "Yolu değiştir…",
            Self::Ingilizce => "Move…",
        }
    }

    /// Satır düğmesi: yolu kopyala.
    #[must_use]
    pub fn dugme_yol(self) -> &'static str {
        match self {
            Self::Turkce => "YOLU KOPYALA",
            Self::Ingilizce => "COPY PATH",
        }
    }

    /// Satır düğmesi: dosyayı kopyala.
    #[must_use]
    pub fn dugme_dosya(self) -> &'static str {
        match self {
            Self::Turkce => "DOSYAYI KOPYALA",
            Self::Ingilizce => "COPY FILE",
        }
    }

    /// Satır düğmesi: yolu değiştir.
    #[must_use]
    pub fn dugme_tasi(self) -> &'static str {
        match self {
            Self::Turkce => "YOLU DEĞİŞTİR",
            Self::Ingilizce => "MOVE",
        }
    }

    /// Satır düğmesi ipucu: yolu kopyala.
    #[must_use]
    pub fn ipucu_yol(self) -> &'static str {
        match self {
            Self::Turkce => "Dosya yolunu panoya kopyala (F1)",
            Self::Ingilizce => "Copy file path to clipboard (F1)",
        }
    }

    /// Satır düğmesi ipucu: dosyayı kopyala.
    #[must_use]
    pub fn ipucu_dosya(self) -> &'static str {
        match self {
            Self::Turkce => "Dosyayı panoya kopyala (F2)",
            Self::Ingilizce => "Copy file to clipboard (F2)",
        }
    }

    /// Satır düğmesi ipucu: yolu değiştir.
    #[must_use]
    pub fn ipucu_tasi(self) -> &'static str {
        match self {
            Self::Turkce => {
                "Dosyayı başka klasöre taşı — hedefi dosya yöneticisinden seç (F3/Enter)"
            }
            Self::Ingilizce => {
                "Move file to another folder — pick target in file manager (F3/Enter)"
            }
        }
    }

    /// Klasör etiketi (boyut / zaman sütununda).
    #[must_use]
    pub fn klasor_etiket(self) -> &'static str {
        match self {
            Self::Turkce => "klasör",
            Self::Ingilizce => "folder",
        }
    }

    /// Süre birimi: saniye.
    #[must_use]
    pub fn birim_saniye(self) -> &'static str {
        match self {
            Self::Turkce => "sn",
            Self::Ingilizce => "s",
        }
    }

    /// Hazırlık durum mesajı.
    #[must_use]
    pub fn durum_hazirlaniyor(self) -> String {
        match self {
            Self::Turkce => String::from("İndeks hazırlanıyor..."),
            Self::Ingilizce => String::from("Preparing index..."),
        }
    }

    /// Tarama başladı mesajı.
    #[must_use]
    pub fn durum_tarama_basladi(self) -> String {
        match self {
            Self::Turkce => String::from("Tarama başlatıldı..."),
            Self::Ingilizce => String::from("Scan started..."),
        }
    }

    /// İndeks yenilendi mesajı.
    #[must_use]
    pub fn durum_indeks_yenilendi(self, kayit: usize) -> String {
        match self {
            Self::Turkce => format!("İndeks yenilendi: {kayit} kayıt"),
            Self::Ingilizce => format!("Index refreshed: {kayit} entries"),
        }
    }

    /// Diskten yükleme mesajı.
    #[must_use]
    pub fn durum_yuklendi(self, kayit: usize, sure: &str) -> String {
        match self {
            Self::Turkce => format!("Diskteki indeks yüklendi: {kayit} kayıt, {sure}."),
            Self::Ingilizce => format!("Loaded index from disk: {kayit} entries, {sure}."),
        }
    }

    /// Eylem sonucu: açıldı.
    #[must_use]
    pub fn eylem_acildi(self, yol: &str) -> String {
        match self {
            Self::Turkce => format!("Açıldı: {yol}"),
            Self::Ingilizce => format!("Opened: {yol}"),
        }
    }

    /// Eylem sonucu: açılamadı.
    #[must_use]
    pub fn eylem_acilamadi(self, yol: &str, hata: &str) -> String {
        match self {
            Self::Turkce => format!("Açılamadı: {yol} — {hata}"),
            Self::Ingilizce => format!("Failed to open: {yol} — {hata}"),
        }
    }

    /// Eylem sonucu: konum açıldı.
    #[must_use]
    pub fn eylem_konum_acildi(self, yol: &str) -> String {
        match self {
            Self::Turkce => format!("Konum açıldı: {yol}"),
            Self::Ingilizce => format!("Location opened: {yol}"),
        }
    }

    /// Eylem sonucu: konum açılamadı.
    #[must_use]
    pub fn eylem_konum_acilamadi(self, yol: &str, hata: &str) -> String {
        match self {
            Self::Turkce => format!("Konum açılamadı: {yol} — {hata}"),
            Self::Ingilizce => format!("Failed to open location: {yol} — {hata}"),
        }
    }

    /// Eylem sonucu: yol kopyalandı.
    #[must_use]
    pub fn eylem_yol_kopyalandi(self, yol: &str) -> String {
        match self {
            Self::Turkce => format!("Yol kopyalandı: {yol}"),
            Self::Ingilizce => format!("Path copied: {yol}"),
        }
    }

    /// Eylem sonucu: kopyalanamadı.
    #[must_use]
    pub fn eylem_kopyalanamadi(self, hata: &str) -> String {
        match self {
            Self::Turkce => format!("Kopyalanamadı: {hata}"),
            Self::Ingilizce => format!("Copy failed: {hata}"),
        }
    }

    /// Eylem sonucu: panoya kopyalandı.
    #[must_use]
    pub fn eylem_panoya_kopyalandi(self, yol: &str) -> String {
        match self {
            Self::Turkce => format!("Panoya kopyalandı: {yol}"),
            Self::Ingilizce => format!("Copied to clipboard: {yol}"),
        }
    }

    /// Eylem sonucu: taşıma iptal.
    #[must_use]
    pub fn eylem_iptal(self) -> String {
        match self {
            Self::Turkce => String::from("Taşıma iptal edildi"),
            Self::Ingilizce => String::from("Move cancelled"),
        }
    }

    /// Eylem sonucu: zaten aynı klasörde.
    #[must_use]
    pub fn eylem_ayni_klasor(self, yol: &str) -> String {
        match self {
            Self::Turkce => format!("Dosya zaten bu klasörde: {yol}"),
            Self::Ingilizce => format!("File is already in this folder: {yol}"),
        }
    }

    /// Eylem sonucu: taşındı.
    #[must_use]
    pub fn eylem_tasindi(self, yol: &str) -> String {
        match self {
            Self::Turkce => format!("Taşındı: {yol}"),
            Self::Ingilizce => format!("Moved: {yol}"),
        }
    }

    /// Eylem sonucu: taşınamadı.
    #[must_use]
    pub fn eylem_tasinamadi(self, hata: &str) -> String {
        match self {
            Self::Turkce => format!("Taşınamadı: {hata}"),
            Self::Ingilizce => format!("Move failed: {hata}"),
        }
    }

    /// Klasör seçici başlığı.
    #[must_use]
    pub fn klasor_sec_baslik(self) -> &'static str {
        match self {
            Self::Turkce => "Dosyanın taşınacağı klasörü seç",
            Self::Ingilizce => "Select destination folder",
        }
    }

    /// Hata: pano açılamadı.
    #[must_use]
    pub fn hata_pano_acilamadi(self) -> &'static str {
        match self {
            Self::Turkce => "pano açılamadı",
            Self::Ingilizce => "clipboard unavailable",
        }
    }

    /// Hata: yol panoya yazılamadı.
    #[must_use]
    pub fn hata_yol_panoya_yazilamadi(self) -> &'static str {
        match self {
            Self::Turkce => "yol panoya yazılamadı",
            Self::Ingilizce => "path could not be written to the clipboard",
        }
    }

    /// Hata: dosya panoya yazılamadı.
    #[must_use]
    pub fn hata_dosya_panoya_yazilamadi(self) -> &'static str {
        match self {
            Self::Turkce => "dosya panoya yazılamadı",
            Self::Ingilizce => "file could not be written to the clipboard",
        }
    }

    /// Hata: varsayılan uygulama çalıştırılamadı.
    #[must_use]
    pub fn hata_varsayilan_uygulama(self) -> &'static str {
        match self {
            Self::Turkce => "varsayılan uygulama çalıştırılamadı",
            Self::Ingilizce => "the default app could not be launched",
        }
    }

    /// Hata: dosya yöneticisi başlatılamadı.
    #[must_use]
    pub fn hata_dosya_yoneticisi(self) -> &'static str {
        match self {
            Self::Turkce => "dosya yöneticisi başlatılamadı",
            Self::Ingilizce => "the file manager could not be launched",
        }
    }

    /// Hata: dosya yöneticisi dosyayı seçemedi.
    #[must_use]
    pub fn hata_secim_gosterilemedi(self) -> &'static str {
        match self {
            Self::Turkce => "dosya yöneticisi dosyayı seçemedi",
            Self::Ingilizce => "the file manager could not reveal the file",
        }
    }

    /// Hata: üst klasör yok.
    #[must_use]
    pub fn hata_ust_klasor_yok(self) -> &'static str {
        match self {
            Self::Turkce => "üst klasör yok",
            Self::Ingilizce => "no parent folder",
        }
    }

    /// Hata: klasör açılamadı.
    #[must_use]
    pub fn hata_klasor_acilamadi(self) -> &'static str {
        match self {
            Self::Turkce => "klasör açılamadı",
            Self::Ingilizce => "the folder could not be opened",
        }
    }

    /// Hata: dosya adı yok.
    #[must_use]
    pub fn hata_dosya_adi_yok(self) -> &'static str {
        match self {
            Self::Turkce => "dosya adı yok",
            Self::Ingilizce => "the file has no name",
        }
    }

    /// Hata: kopyalanamadı.
    #[must_use]
    pub fn hata_kopyalanamadi(self, kaynak: &str, hedef: &str) -> String {
        match self {
            Self::Turkce => format!("kopyalanamadı: {kaynak} → {hedef}"),
            Self::Ingilizce => format!("copy failed: {kaynak} → {hedef}"),
        }
    }

    /// Hata: taşınamadı.
    #[must_use]
    pub fn hata_tasinamadi(self, kaynak: &str, hedef: &str) -> String {
        match self {
            Self::Turkce => format!("taşınamadı: {kaynak} → {hedef}"),
            Self::Ingilizce => format!("move failed: {kaynak} → {hedef}"),
        }
    }

    /// Hata: klasör kendi içine taşınamaz.
    #[must_use]
    pub fn hata_kendi_icine(self, yol: &str) -> String {
        match self {
            Self::Turkce => format!("bir klasör kendi içine taşınamaz: {yol}"),
            Self::Ingilizce => format!("a folder cannot be moved into itself: {yol}"),
        }
    }

    /// Hata: hedefte aynı adlı kayıt var.
    #[must_use]
    pub fn hata_ayni_adli(self, hedef: &str) -> String {
        match self {
            Self::Turkce => format!("hedefte aynı adlı kayıt var: {hedef}"),
            Self::Ingilizce => format!("a file with the same name already exists: {hedef}"),
        }
    }

    /// Göreli zaman: az önce.
    #[must_use]
    pub fn zaman_az_once(self) -> &'static str {
        match self {
            Self::Turkce => "az önce",
            Self::Ingilizce => "just now",
        }
    }

    /// Göreli zaman: dakika.
    #[must_use]
    pub fn zaman_dakika(self, dakika: u64) -> String {
        match self {
            Self::Turkce => format!("{dakika} dk önce"),
            Self::Ingilizce => format!("{dakika} min ago"),
        }
    }

    /// Göreli zaman: saat.
    #[must_use]
    pub fn zaman_saat(self, saat: u64) -> String {
        match self {
            Self::Turkce => format!("{saat} sa önce"),
            Self::Ingilizce => format!("{saat} h ago"),
        }
    }

    /// Göreli zaman: gün.
    #[must_use]
    pub fn zaman_gun(self, gun: u64) -> String {
        match self {
            Self::Turkce => format!("{gun} g önce"),
            Self::Ingilizce => format!("{gun} d ago"),
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn dugme_dili_hedefi_gosterir() {
        // Türkçe iken düğmede EN, İngilizce iken TR yazar.
        assert_eq!(Dil::Turkce.dugme_etiket(), "EN");
        assert_eq!(Dil::Ingilizce.dugme_etiket(), "TR");
        assert_eq!(Dil::Turkce.diger(), Dil::Ingilizce);
        assert_eq!(Dil::Ingilizce.diger(), Dil::Turkce);
    }

    #[test]
    fn kod_donusumu_dogrudur() {
        assert_eq!(Dil::koddan("en"), Dil::Ingilizce);
        assert_eq!(Dil::koddan("en-US"), Dil::Ingilizce);
        assert_eq!(Dil::koddan("  EN  "), Dil::Ingilizce);
        assert_eq!(Dil::koddan("tr"), Dil::Turkce);
        assert_eq!(Dil::koddan("bilinmeyen"), Dil::Turkce);
        assert_eq!(Dil::Turkce.kodu(), "tr");
        assert_eq!(Dil::Ingilizce.kodu(), "en");
    }

    #[test]
    fn sabit_yazilar_iki_dilde_de_dolu() {
        let (tr, en) = (Dil::Turkce, Dil::Ingilizce);
        let ciftler: [(&str, &str); 43] = [
            (tr.dugme_etiket(), en.dugme_etiket()),
            (tr.dugme_ipucu(), en.dugme_ipucu()),
            (tr.alt_baslik(), en.alt_baslik()),
            (tr.arama_ipucu(), en.arama_ipucu()),
            (tr.yeniden_tara(), en.yeniden_tara()),
            (tr.taraniyor_etiket(), en.taraniyor_etiket()),
            (tr.tarama_sürüyor(), en.tarama_sürüyor()),
            (tr.canli_acik(), en.canli_acik()),
            (tr.canli_kapali(), en.canli_kapali()),
            (tr.sutun_dosya(), en.sutun_dosya()),
            (tr.sutun_konum(), en.sutun_konum()),
            (tr.sutun_boyut(), en.sutun_boyut()),
            (tr.sutun_zaman(), en.sutun_zaman()),
            (tr.sutun_islem(), en.sutun_islem()),
            (tr.bos_aciklama(), en.bos_aciklama()),
            (tr.bos_ipucu(), en.bos_ipucu()),
            (tr.sonuc_yok(), en.sonuc_yok()),
            (tr.alt_kisayollar(), en.alt_kisayollar()),
            (tr.menu_ac(), en.menu_ac()),
            (tr.menu_konum(), en.menu_konum()),
            (tr.menu_yol(), en.menu_yol()),
            (tr.menu_dosya(), en.menu_dosya()),
            (tr.menu_tasi(), en.menu_tasi()),
            (tr.dugme_yol(), en.dugme_yol()),
            (tr.dugme_dosya(), en.dugme_dosya()),
            (tr.dugme_tasi(), en.dugme_tasi()),
            (tr.ipucu_yol(), en.ipucu_yol()),
            (tr.ipucu_dosya(), en.ipucu_dosya()),
            (tr.ipucu_tasi(), en.ipucu_tasi()),
            (tr.klasor_etiket(), en.klasor_etiket()),
            (tr.birim_saniye(), en.birim_saniye()),
            (tr.klasor_sec_baslik(), en.klasor_sec_baslik()),
            (tr.hata_pano_acilamadi(), en.hata_pano_acilamadi()),
            (
                tr.hata_yol_panoya_yazilamadi(),
                en.hata_yol_panoya_yazilamadi(),
            ),
            (
                tr.hata_dosya_panoya_yazilamadi(),
                en.hata_dosya_panoya_yazilamadi(),
            ),
            (tr.hata_varsayilan_uygulama(), en.hata_varsayilan_uygulama()),
            (tr.hata_dosya_yoneticisi(), en.hata_dosya_yoneticisi()),
            (tr.hata_secim_gosterilemedi(), en.hata_secim_gosterilemedi()),
            (tr.hata_ust_klasor_yok(), en.hata_ust_klasor_yok()),
            (tr.hata_klasor_acilamadi(), en.hata_klasor_acilamadi()),
            (tr.hata_dosya_adi_yok(), en.hata_dosya_adi_yok()),
            (tr.zaman_az_once(), en.zaman_az_once()),
            (tr.kodu(), en.kodu()),
        ];
        for (turkce, ingilizce) in ciftler {
            assert!(!turkce.is_empty(), "Türkçe yazı boş olamaz");
            assert!(!ingilizce.is_empty(), "İngilizce yazı boş olamaz");
            assert_ne!(
                turkce, ingilizce,
                "`{turkce}` çevirisi eksik: iki dil aynı olamaz"
            );
        }
    }

    #[test]
    fn degisken_yazilar_iki_dilde_de_dolu() {
        let (tr, en) = (Dil::Turkce, Dil::Ingilizce);
        let ciftler: [(String, String); 28] = [
            (tr.taraniyor_durum(1, 2, 3), en.taraniyor_durum(1, 2, 3)),
            (tr.indeks_hazir(7, 4), en.indeks_hazir(7, 4)),
            (tr.kayit_cipi(1234), en.kayit_cipi(1234)),
            (tr.son_tarama("1.5 s"), en.son_tarama("1.5 s")),
            (tr.sonuc_yok_aciklama("x"), en.sonuc_yok_aciklama("x")),
            (tr.sonuc_sayisi(9), en.sonuc_sayisi(9)),
            (tr.durum_hazirlaniyor(), en.durum_hazirlaniyor()),
            (tr.durum_tarama_basladi(), en.durum_tarama_basladi()),
            (tr.durum_indeks_yenilendi(5), en.durum_indeks_yenilendi(5)),
            (tr.durum_yuklendi(5, "1.5 s"), en.durum_yuklendi(5, "1.5 s")),
            (tr.eylem_acildi("/a/b"), en.eylem_acildi("/a/b")),
            (
                tr.eylem_acilamadi("/a/b", "neden"),
                en.eylem_acilamadi("/a/b", "neden"),
            ),
            (tr.eylem_konum_acildi("/a"), en.eylem_konum_acildi("/a")),
            (
                tr.eylem_konum_acilamadi("/a", "neden"),
                en.eylem_konum_acilamadi("/a", "neden"),
            ),
            (tr.eylem_yol_kopyalandi("/a"), en.eylem_yol_kopyalandi("/a")),
            (
                tr.eylem_kopyalanamadi("neden"),
                en.eylem_kopyalanamadi("neden"),
            ),
            (
                tr.eylem_panoya_kopyalandi("/a"),
                en.eylem_panoya_kopyalandi("/a"),
            ),
            (tr.eylem_iptal(), en.eylem_iptal()),
            (tr.eylem_ayni_klasor("/a"), en.eylem_ayni_klasor("/a")),
            (tr.eylem_tasindi("/a"), en.eylem_tasindi("/a")),
            (tr.eylem_tasinamadi("neden"), en.eylem_tasinamadi("neden")),
            (
                tr.hata_kopyalanamadi("/a", "/b"),
                en.hata_kopyalanamadi("/a", "/b"),
            ),
            (
                tr.hata_tasinamadi("/a", "/b"),
                en.hata_tasinamadi("/a", "/b"),
            ),
            (tr.hata_kendi_icine("/a"), en.hata_kendi_icine("/a")),
            (tr.hata_ayni_adli("/b"), en.hata_ayni_adli("/b")),
            (tr.zaman_dakika(3), en.zaman_dakika(3)),
            (tr.zaman_saat(3), en.zaman_saat(3)),
            (tr.zaman_gun(3), en.zaman_gun(3)),
        ];
        for (turkce, ingilizce) in ciftler {
            assert!(!turkce.is_empty(), "Türkçe yazı boş olamaz");
            assert!(!ingilizce.is_empty(), "İngilizce yazı boş olamaz");
            assert_ne!(
                turkce, ingilizce,
                "`{turkce}` çevirisi eksik: iki dil aynı olamaz"
            );
        }
    }

    #[test]
    fn ornek_sorgular_her_dilde_dolu() {
        let tr = Dil::Turkce.ornek_sorgular();
        let en = Dil::Ingilizce.ornek_sorgular();
        assert_eq!(tr.len(), 5);
        assert_eq!(en.len(), 5);
        assert!(tr.iter().all(|s| !s.is_empty()));
        assert!(en.iter().all(|s| !s.is_empty()));
        assert_ne!(tr, en, "örnek sorgular dile göre değişmeli");
    }

    #[test]
    fn saniye_birimi_dil_degistirir() {
        assert_eq!(Dil::Turkce.birim_saniye(), "sn");
        assert_eq!(Dil::Ingilizce.birim_saniye(), "s");
    }
}
