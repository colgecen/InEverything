//! Arama motoru: tam eşleşme, uzantı filtresi ve bulanık arama.
//!
//! Ağır eşleşme işi arka plan görevlisinde yapılır; arayüz asla kilitlenmez.

use std::sync::{Arc, RwLock};

use crossbeam_channel::{Receiver, Sender};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

use crate::model::{FileItem, SearchQuery};

/// Sorgudaki aranacak iğne metni (`*.pdf rapor` -> `rapor`).
fn igneyi_ayikla(sorgu: &SearchQuery) -> String {
    if sorgu.uzanti_filtresi.is_none() {
        return sorgu.kucuk.clone();
    }
    let mut parcalar = sorgu.kucuk.split_whitespace();
    let _ = parcalar.next();
    parcalar.collect::<Vec<_>>().join(" ")
}

/// Bellek içi indekste ara, eşleşen kayıtların konumlarını skor sırasıyla döndür.
///
/// Sıralama: tam alt-dize eşleşmeleri önce (kayıt sırasını korur),
/// ardından bulanık eşleşmeler skora göre.
pub fn ara(ogeler: &[FileItem], ham_sorgu: &str) -> Vec<usize> {
    let sorgu = SearchQuery::cozumle(ham_sorgu);
    if sorgu.bos_mu() {
        return Vec::new();
    }
    let igne = igneyi_ayikla(&sorgu);
    let bulanik = SkimMatcherV2::default();
    let mut skorlu: Vec<(i64, usize)> = Vec::new();

    for (konum, oge) in ogeler.iter().enumerate() {
        if let Some(filtre) = &sorgu.uzanti_filtresi {
            if oge.uzanti().as_deref() != Some(filtre.as_str()) {
                continue;
            }
        }
        if igne.is_empty() {
            skorlu.push((0, konum));
            continue;
        }
        let ad_kucuk = oge.ad.to_lowercase();
        if ad_kucuk.contains(&igne) {
            skorlu.push((0, konum));
        } else if let Some(skor) = bulanik.fuzzy_match(&ad_kucuk, &igne) {
            skorlu.push((i64::MAX - skor, konum));
        }
    }

    skorlu.sort();
    skorlu.into_iter().map(|(_, konum)| konum).collect()
}

/// Arka plan arama görevlisini başlat.
///
/// Kanaldan gelen her sorgunun en güncelini dizinde arar, sonucu gönderir.
/// İstek gönderen düşünce görevli sessizce sonlanır.
pub fn arama_gorevlisi_baslat(
    indeks: Arc<RwLock<Vec<FileItem>>>,
    istek: Receiver<String>,
    sonuc: Sender<Vec<usize>>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || loop {
        let ilk = match istek.recv() {
            Ok(sorgu) => sorgu,
            Err(_) => break,
        };
        let mut guncel = ilk;
        while let Ok(yeni) = istek.try_recv() {
            guncel = yeni;
        }
        let okunan = match indeks.read() {
            Ok(kilit) => kilit,
            Err(_) => break,
        };
        let cikti = ara(&okunan, &guncel);
        drop(okunan);
        if sonuc.send(cikti).is_err() {
            break;
        }
    })
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::path::PathBuf;
    use std::time::Duration;

    fn ornek_indeks() -> Vec<FileItem> {
        vec![
            FileItem::dosya(PathBuf::from("C:\\A\\rapor.pdf"), 100, None),
            FileItem::dosya(PathBuf::from("C:\\A\\rapor-taslagi.docx"), 200, None),
            FileItem::dosya(PathBuf::from("C:\\A\\foto.png"), 300, None),
        ]
    }

    #[test]
    fn tam_eslesme_bulunur() {
        let cikti = ara(&ornek_indeks(), "foto");
        assert_eq!(cikti, vec![2]);
    }

    #[test]
    fn buyuk_kucuk_harf_duyarsizdir() {
        let cikti = ara(&ornek_indeks(), "RAPOR");
        assert_eq!(cikti, vec![0, 1]);
    }

    #[test]
    fn uzanti_filtresi_daraltir() {
        let cikti = ara(&ornek_indeks(), "*.pdf");
        assert_eq!(cikti, vec![0]);
    }

    #[test]
    fn birlesik_sorgu_uzanti_ve_metin_ister() {
        let cikti = ara(&ornek_indeks(), "*.docx taslak");
        assert_eq!(cikti, vec![1]);
        let bos = ara(&ornek_indeks(), "*.docx foto");
        assert!(bos.is_empty());
    }

    #[test]
    fn bulanik_eslesme_yakini_bulur() {
        let cikti = ara(&ornek_indeks(), "rpr");
        assert!(cikti.contains(&0));
    }

    #[test]
    fn eslesmeyince_bos_doner() {
        assert!(ara(&ornek_indeks(), "zzzbulunamaz").is_empty());
    }

    #[test]
    fn bos_sorgu_bos_doner() {
        assert!(ara(&ornek_indeks(), "   ").is_empty());
    }

    #[test]
    fn gorevli_sorguyu_yanitlar() {
        let (istek_tx, istek_rx) = crossbeam_channel::unbounded();
        let (sonuc_tx, sonuc_rx) = crossbeam_channel::unbounded();
        let indeks = Arc::new(RwLock::new(ornek_indeks()));
        let tutamac = arama_gorevlisi_baslat(indeks, istek_rx, sonuc_tx);

        istek_tx.send("foto".to_string()).expect("gönder");
        let cikti = sonuc_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("yanıt");
        assert_eq!(cikti, vec![2]);

        drop(istek_tx);
        tutamac.join().expect("görevli kapandı");
    }
}
