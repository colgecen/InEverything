//! Uçtan uca testler: tarama, kalıcı indeks yazma/okuma ve arama.

use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};

use ineverything::depo::Indeks;
use ineverything::indexer::{sistemi_tara, sistemi_tara_indeks, TaramaDurumu};
use ineverything::model::FileItem;
use ineverything::search::ara;

fn ornek_agac() -> tempfile::TempDir {
    let dizin = tempfile::tempdir().expect("geçici dizin");
    std::fs::write(dizin.path().join("yillik-rapor.pdf"), b"a").expect("yaz");
    std::fs::write(dizin.path().join("toplanti-notlari.txt"), b"b").expect("yaz");
    let alt = dizin.path().join("arsiv");
    std::fs::create_dir(&alt).expect("klasör");
    std::fs::write(alt.join("yedek-rapor.pdf"), b"c").expect("yaz");
    dizin
}

#[test]
fn tara_ve_ara_uctan_uca() {
    let dizin = ornek_agac();
    let kok = dizin.path().to_path_buf();
    let durum = TaramaDurumu::default();
    let iptal = Arc::new(AtomicBool::new(false));
    let kayitlar = sistemi_tara(std::slice::from_ref(&kok), &durum, &iptal);
    assert_eq!(kayitlar.len(), 5, "3 dosya + 2 klasör");
    assert_eq!(durum.sayi.load(Ordering::Relaxed), 3);
    assert!(durum.bitti.load(Ordering::Relaxed));

    let paylasilan = Arc::new(RwLock::new(kayitlar));
    let okunan = paylasilan.read().expect("kilit");

    let pdf: Vec<PathBuf> = ara(&okunan, "*.pdf")
        .into_iter()
        .map(|i| okunan[i].yol.clone())
        .collect();
    assert_eq!(pdf.len(), 2);

    let rapor: Vec<String> = ara(&okunan, "rapor")
        .into_iter()
        .map(|i| okunan[i].ad.clone())
        .collect();
    assert_eq!(rapor.len(), 2);
    assert!(rapor.iter().all(|ad| ad.to_lowercase().contains("rapor")));

    let _ = FileItem::dosya(PathBuf::from("x"), 0, None);
}

#[test]
fn kalici_indeks_dosyaya_yazilip_okunur_ve_aranir() {
    let dizin = ornek_agac();
    let kok = dizin.path().to_path_buf();
    let durum = TaramaDurumu::default();
    let iptal = Arc::new(AtomicBool::new(false));
    let indeks = sistemi_tara_indeks(std::slice::from_ref(&kok), &[], &durum, &iptal, 1_234);
    assert_eq!(indeks.kayit_sayisi(), 5);
    assert_eq!(indeks.ayar_hash, 1_234);

    let yol = dizin.path().join("alt").join("indeks.bin");
    indeks.yaz(&yol).expect("yaz");
    let okunan = Indeks::yukle(&yol).expect("oku");
    assert_eq!(okunan.kayit_sayisi(), 5);
    assert_eq!(okunan.ayar_hash, 1_234);
    assert!(!okunan.bayat_mi(1_234, 24), "taze indeks bayat sayılmaz");
    assert!(okunan.bayat_mi(999, 0), "ayar değişince indeks bayat");

    let pdf = okunan.ara("*.pdf", 100);
    assert_eq!(pdf.len(), 2, "2 PDF kayıt bekleniyor");

    let rapor = okunan.ara("rapor", 100);
    assert_eq!(rapor.len(), 2);
    let adlar: Vec<&str> = rapor
        .iter()
        .filter_map(|i| okunan.kayit(*i).map(|k| k.ad))
        .collect();
    assert!(adlar.contains(&"yillik-rapor.pdf"));
    assert!(adlar.contains(&"yedek-rapor.pdf"));

    let arsiv = okunan.ara("arsiv", 100);
    assert!(!arsiv.is_empty(), "klasör kayıtları da aranabilmeli");
    assert!(
        okunan.kayit(arsiv[0]).expect("kayıt").klasor_mu,
        "arsiv bir klasör olmalı"
    );

    // Sonuç sayısı sınırı aşılmaz.
    assert_eq!(okunan.ara("rapor", 1).len(), 1);
}

#[test]
fn sistem_taramasi_kokler_iptalde_bos_doner() {
    let durum = TaramaDurumu::default();
    let iptal = Arc::new(AtomicBool::new(true));
    let indeks = sistemi_tara_indeks(&[PathBuf::from("/")], &[], &durum, &iptal, 1);
    assert_eq!(indeks.kayit_sayisi(), 0);
    assert!(durum.bitti.load(Ordering::Relaxed));
}
