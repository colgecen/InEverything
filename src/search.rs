//! Arama motoru: kalıcı indeks üzerinde anlık arama + canlı katman.
//!
//! Ağır iş (yüz binlerce kaydı taramak) arka plan görevlisinde yapılır ve
//! tahsis yapmaz; arayüz yalnızca sonuç konumlarını alır.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crossbeam_channel::{Receiver, Sender};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

use crate::{
    depo::Indeks,
    indexer::DegisiklikTuru,
    model::{FileItem, SearchQuery},
};

/// Silinen kayıt sayısı bu eşiği aşarsa katman atılır ve tam tarama
/// tetiklenir (yüz binlerce silme kaydını her aramada süzmek pahalıdır).
const SILINEN_ESIK: usize = 4_000;
/// Canlı katmanda tutulabilecek en fazla yeni kayıt.
const CANLI_ESIK: usize = 20_000;

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
/// Kanaldan gelen her sorgunun en güncelini kalıcı indeks + canlı katman
/// üzerinde arar, sonucu gönderir. İstek gönderen düşünce görevli sessizce
/// sonlanır.
pub fn arama_gorevlisi_baslat(
    indeks: Arc<RwLock<Arc<Indeks>>>,
    canli: Arc<RwLock<CanliKatman>>,
    istek: Receiver<AramaIstegi>,
    sonuc: Sender<Sonuclar>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        while let Ok(ilk) = istek.recv() {
            let mut guncel = ilk;
            while let Ok(yeni) = istek.try_recv() {
                guncel = yeni;
            }
            let Ok(indeks) = indeks.read() else {
                break;
            };
            let Ok(katman) = canli.read() else {
                break;
            };
            let mut indeks_sonuc = indeks.ara(&guncel.sorgu, guncel.limit);
            if !katman.silinen_bos_mu() {
                indeks_sonuc.retain(|konum| {
                    indeks
                        .kayit(*konum)
                        .map(|k| !katman.silinmis_mi(k.yol))
                        .unwrap_or(false)
                });
            }
            let yeniler = katman.ekle_olanlar();
            let canli_sonuc: Vec<FileItem> = if yeniler.is_empty() {
                Vec::new()
            } else {
                ara(&yeniler, &guncel.sorgu)
                    .into_iter()
                    .filter_map(|konum| yeniler.get(konum).cloned())
                    .collect()
            };
            let cikti = Sonuclar {
                indeks: indeks_sonuc,
                canli: canli_sonuc,
            };
            drop(katman);
            drop(indeks);
            if sonuc.send(cikti).is_err() {
                break;
            }
        }
    })
}

/// Bir arama isteği: sorgu metni ve gösterilecek en fazla sonuç sayısı.
#[derive(Debug, Clone)]
pub struct AramaIstegi {
    pub sorgu: String,
    pub limit: usize,
}

/// Arama sonucu: kalıcı indeks konumları + henüz indekse girmemiş canlı
/// kayıtlar.
#[derive(Debug, Clone, Default)]
pub struct Sonuclar {
    pub indeks: Vec<usize>,
    pub canli: Vec<FileItem>,
}

impl Sonuclar {
    /// Toplam sonuç sayısı.
    pub fn toplam(&self) -> usize {
        self.indeks.len() + self.canli.len()
    }
}

/// İndeks taraması sürerken oluşan değişikliklerin tutulduğu katman.
///
/// Tam tarama her seferinde saniyeler sürdüğü için, arama sonuçlarını
/// anında güncel tutmak için silinen yollar (indeks kaydı gizlenir) ve
/// yeni oluşan kayıtlar (doğrudan sonuçlara eklenir) saklanır.
#[derive(Debug, Default)]
pub struct CanliKatman {
    silinen: HashSet<PathBuf>,
    yeniler: Vec<FileItem>,
    /// Katman bu sınırı aştıysa tam tarama gerekir.
    asildi: bool,
}

impl CanliKatman {
    /// Yeni boş katman.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bildirilen değişikliği kaydeder.
    pub fn kaydet(&mut self, degisiklik: &crate::indexer::Degisiklik) {
        if self.asildi {
            return;
        }
        match degisiklik.tur {
            DegisiklikTuru::Silindi => {
                self.silinen.insert(degisiklik.yol.clone());
                if self.silinen.len() > SILINEN_ESIK {
                    self.asildi = true;
                }
            }
            DegisiklikTuru::Olustu | DegisiklikTuru::YenidenAdlandirildi => {
                // Yeniden adlandırmada eski ad da indeksde kalır; tam tarama
                // ile temizlenecek, burada yalnızca yeni adı ekliyoruz.
                let Ok(meta) = std::fs::symlink_metadata(&degisiklik.yol) else {
                    return;
                };
                let oge = if meta.is_dir() {
                    FileItem::klasor(degisiklik.yol.clone())
                } else {
                    FileItem::dosya(degisiklik.yol.clone(), meta.len(), meta.modified().ok())
                };
                self.yeniler.push(oge);
                if self.yeniler.len() > CANLI_ESIK {
                    self.asildi = true;
                }
            }
            DegisiklikTuru::Diger => {}
        }
    }

    /// Tam tarama sonrası katmanı sıfırlar.
    pub fn temizle(&mut self) {
        self.silinen.clear();
        self.yeniler.clear();
        self.asildi = false;
    }

    /// Katmanın taştığını (tam tarama gerektiğini) bildirir.
    pub fn asildi_mi(&self) -> bool {
        self.asildi
    }

    /// Silinen kayıt listesi boş mu (arama sırasında süzme yapılmaz)?
    pub fn silinen_bos_mu(&self) -> bool {
        self.silinen.is_empty()
    }

    /// Kayıtlı yeni dosyaların kopyası.
    pub fn ekle_olanlar(&self) -> Vec<FileItem> {
        self.yeniler.clone()
    }

    /// Verilen yol silinmiş mi (ya da silinmiş bir klasörün içinde mi)?
    pub fn silinmis_mi(&self, yol: &str) -> bool {
        if self.silinen.is_empty() {
            return false;
        }
        let yol = PathBuf::from(yol);
        self.silinen
            .iter()
            .any(|silinen| yol == *silinen || yol.starts_with(silinen))
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::path::PathBuf;
    use std::time::Duration;

    fn ornek_indeks() -> Vec<FileItem> {
        vec![
            FileItem::dosya(PathBuf::from("C:\\A\\rapor.pdf"), 100, None),
            FileItem::dosya(PathBuf::from("C:\\A\\rapor-taslak.docx"), 200, None),
            FileItem::dosya(PathBuf::from("C:\\A\\foto.png"), 300, None),
        ]
    }

    fn ornek_indeks_depo() -> Indeks {
        let mut yazici = crate::depo::KayitYazici::yeni();
        yazici.ekle(b"/a/rapor.pdf", 100, None, false);
        yazici.ekle(b"/a/rapor-taslak.docx", 200, None, false);
        yazici.ekle(b"/a/foto.png", 300, None, false);
        yazici.indeks_uret(1, 1, 0)
    }

    fn indeksteki_ad(indeks: &Arc<RwLock<Arc<Indeks>>>, konum: usize) -> String {
        let okunan = indeks.read().expect("kilit");
        okunan
            .kayit(konum)
            .map(|k| k.ad.to_string())
            .unwrap_or_default()
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
    fn gorevli_indeks_ve_canli_katman_uzerinde_arar() {
        let (istek_tx, istek_rx) = crossbeam_channel::unbounded();
        let (sonuc_tx, sonuc_rx) = crossbeam_channel::unbounded();
        let indeks = Arc::new(RwLock::new(Arc::new(ornek_indeks_depo())));
        let canli = Arc::new(RwLock::new(CanliKatman::yeni()));
        let tutamac = arama_gorevlisi_baslat(Arc::clone(&indeks), canli, istek_rx, sonuc_tx);

        istek_tx
            .send(AramaIstegi {
                sorgu: "foto".to_string(),
                limit: 100,
            })
            .expect("gönder");
        let cikti = sonuc_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("yanıt");
        assert_eq!(cikti.indeks.len(), 1);
        assert_eq!(indeksteki_ad(&indeks, cikti.indeks[0]), "foto.png");
        assert!(cikti.canli.is_empty());

        drop(istek_tx);
        tutamac.join().expect("görevli kapandı");
    }

    #[test]
    fn silinen_kayit_sonuctan_diser() {
        let (istek_tx, istek_rx) = crossbeam_channel::unbounded();
        let (sonuc_tx, sonuc_rx) = crossbeam_channel::unbounded();
        let indeks = Arc::new(RwLock::new(Arc::new(ornek_indeks_depo())));
        let canli = Arc::new(RwLock::new(CanliKatman::yeni()));
        {
            let mut katman = canli.write().expect("kilit");
            katman.kaydet(&crate::indexer::Degisiklik {
                yol: PathBuf::from("/a/foto.png"),
                tur: DegisiklikTuru::Silindi,
            });
        }
        let tutamac = arama_gorevlisi_baslat(indeks, canli, istek_rx, sonuc_tx);

        istek_tx
            .send(AramaIstegi {
                sorgu: "foto".to_string(),
                limit: 100,
            })
            .expect("gönder");
        let cikti = sonuc_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("yanıt");
        assert!(cikti.indeks.is_empty(), "silinen kayıt gizlenmeli");
        drop(istek_tx);
        tutamac.join().expect("görevli kapandı");
    }

    #[test]
    fn yeni_olusan_kayit_sonuca_eklenir() {
        let dizin = tempfile::tempdir().expect("geçici dizin");
        let yeni = dizin.path().join("yeni-notlar.txt");
        std::fs::write(&yeni, b"x").expect("yaz");

        let mut katman = CanliKatman::yeni();
        katman.kaydet(&crate::indexer::Degisiklik {
            yol: yeni.clone(),
            tur: DegisiklikTuru::Olustu,
        });
        assert_eq!(katman.ekle_olanlar().len(), 1);
        assert_eq!(ara(&katman.ekle_olanlar(), "notlar"), vec![0]);
        assert!(!katman.silinmis_mi("/a/b"));
    }

    #[test]
    fn silinen_klasor_icerigi_de_gizlenir() {
        let mut katman = CanliKatman::yeni();
        katman.kaydet(&crate::indexer::Degisiklik {
            yol: PathBuf::from("/a/arsiv"),
            tur: DegisiklikTuru::Silindi,
        });
        assert!(katman.silinmis_mi("/a/arsiv"));
        assert!(katman.silinmis_mi("/a/arsiv/ic/rapor.pdf"));
        assert!(!katman.silinmis_mi("/a/arsivler"));
        katman.temizle();
        assert!(!katman.silinmis_mi("/a/arsiv"));
    }

    #[test]
    fn katman_tasma_durumunda_isaretlenir() {
        let mut katman = CanliKatman::yeni();
        for i in 0..=SILINEN_ESIK {
            katman.kaydet(&crate::indexer::Degisiklik {
                yol: PathBuf::from(format!("/a/{i}.txt")),
                tur: DegisiklikTuru::Silindi,
            });
        }
        assert!(katman.asildi_mi());
        katman.temizle();
        assert!(!katman.asildi_mi());
    }
}
