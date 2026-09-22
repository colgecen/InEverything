//! Uçtan uca test: geçici dizini tara, indekste ara.

use std::path::PathBuf;
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicBool, Ordering},
};

use fastfind::indexer::{TaramaDurumu, sistemi_tara};
use fastfind::model::FileItem;
use fastfind::search::ara;

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
    let iptal = AtomicBool::new(false);
    let indeks = sistemi_tara(std::slice::from_ref(&kok), &durum, &iptal);
    assert_eq!(indeks.len(), 3);
    assert!(durum.bitti.load(Ordering::Relaxed));

    let paylasilan = Arc::new(RwLock::new(indeks));
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
