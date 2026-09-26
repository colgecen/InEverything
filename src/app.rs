//! Grafik arayüz: eframe/egui tabanlı ana pencere.
//!
//! Açılışta diskteki indeks anında belleğe alınır (yüz binlerce kayıt için
//! onlarca milisaniye), tarama yalnızca indeks bayatsa arka planda koşar.

use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, RwLock,
};
use std::time::{Duration, SystemTime};

use crossbeam_channel::{Receiver, Sender, TryRecvError};
use eframe::egui;

use crate::{
    actions,
    config::AppConfig,
    depo::Indeks,
    indexer::{self, CanliIzleyici, Degisiklik, TaramaDurumu},
    search::{self, AramaIstegi, CanliKatman, Sonuclar},
};

/// Satırda tetiklenen dosya eylemi.
enum Eylem {
    Ac(PathBuf),
    KonumuAc(PathBuf),
    YoluKopyala(PathBuf),
}

/// Arayüzde listelenen bir satırın kaynağı.
#[derive(Debug, Clone, Copy)]
enum Satir {
    /// Kalıcı indeksteki kaydın konumu.
    Indeks(usize),
    /// Taramadan önce oluşan canlı kaydın konumu.
    Canli(usize),
}

/// Satır çizmek için gereken alanların dağılmadan taşınması.
#[derive(Debug, Clone)]
struct SatirVerisi {
    ad: String,
    yol: String,
    boyut: u64,
    zaman: Option<SystemTime>,
    klasor: bool,
}

/// Ana uygulama durumu.
pub struct InEverythingApp {
    sorgu_metni: String,
    son_sorgu: String,
    /// Kalıcı indeks; değişince `nesil` artar ve arama yenilenir.
    indeks: Arc<RwLock<Arc<Indeks>>>,
    canli: Arc<RwLock<CanliKatman>>,
    tarama: TaramaDurumu,
    taraniyor: Arc<AtomicBool>,
    tarama_iptal: Arc<AtomicBool>,
    nesil: Arc<AtomicUsize>,
    son_nesil: usize,
    sonuclar: Sonuclar,
    satirlar: Vec<Satir>,
    secili: Option<usize>,
    ilk_cerceve: bool,
    durum_mesaji: String,
    yapilandirma: AppConfig,
    ayar_hash: u64,
    indeks_yolu: PathBuf,
    arama_istek_tx: Sender<AramaIstegi>,
    arama_sonuc_rx: Receiver<Sonuclar>,
    degisiklik_rx: Receiver<Degisiklik>,
    _izleyici: Option<CanliIzleyici>,
}

/// Uygulamayı başlatır, pencere kapanana kadar dönmez.
pub fn calistir() -> eframe::Result<()> {
    let secenekler = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "InEverything",
        secenekler,
        Box::new(|baglam| Ok(Box::new(InEverythingApp::yeni(baglam)))),
    )
}

impl InEverythingApp {
    /// İlk kurulum: yapılandırma, indeks yükleme, kanallar, izleyici.
    pub fn yeni(baglam: &eframe::CreationContext<'_>) -> Self {
        baglam.egui_ctx.set_visuals(egui::Visuals::dark());
        let yapilandirma = AppConfig::yukle();
        let kokler = yapilandirma.etkin_kokler();
        let ayar_hash = yapilandirma.ayar_imzasi();
        let indeks_yolu = yapilandirma.indeks_yolu();

        let yukleme_bas = std::time::Instant::now();
        let indeks = Indeks::yukle(&indeks_yolu).ok();
        let yukleme_ms = yukleme_bas.elapsed().as_secs_f64() * 1000.0;
        let kayit_sayisi = indeks.as_ref().map_or(0, |i| i.kayit_sayisi());
        let bayat = indeks.as_ref().map_or(true, |i| {
            i.bayat_mi(ayar_hash, yapilandirma.indeks_yasi_saat)
        });

        let indeks = Arc::new(RwLock::new(Arc::new(indeks.unwrap_or_else(bos_indeks))));
        let canli = Arc::new(RwLock::new(CanliKatman::yeni()));
        let tarama = TaramaDurumu::default();
        let taraniyor = Arc::new(AtomicBool::new(false));
        let nesil = Arc::new(AtomicUsize::new(0));
        let (arama_istek_tx, arama_istek_rx) = crossbeam_channel::bounded::<AramaIstegi>(32);
        let (arama_sonuc_tx, arama_sonuc_rx) = crossbeam_channel::bounded::<Sonuclar>(32);
        let (degisiklik_tx, degisiklik_rx) = crossbeam_channel::unbounded::<Degisiklik>();

        let tarama_iptal = Arc::new(AtomicBool::new(false));
        let mut uygulama = Self {
            sorgu_metni: String::new(),
            son_sorgu: String::new(),
            indeks: Arc::clone(&indeks),
            canli: Arc::clone(&canli),
            tarama: tarama.clone(),
            taraniyor: Arc::clone(&taraniyor),
            tarama_iptal: Arc::clone(&tarama_iptal),
            nesil: Arc::clone(&nesil),
            son_nesil: nesil.load(Ordering::Relaxed),
            sonuclar: Sonuclar::default(),
            satirlar: Vec::new(),
            secili: None,
            ilk_cerceve: true,
            durum_mesaji: String::from("İndeks hazırlanıyor..."),
            yapilandirma: yapilandirma.clone(),
            ayar_hash,
            indeks_yolu: indeks_yolu.clone(),
            arama_istek_tx,
            arama_sonuc_rx,
            degisiklik_rx,
            _izleyici: None,
        };

        if bayat {
            uygulama.tarama_tetikle(&kokler);
        } else {
            uygulama.tarama.bitti.store(true, Ordering::Relaxed);
            uygulama.durum_mesaji = format!(
                "Diskteki indeks yüklendi: {kayit_sayisi} kayıt, {}.",
                sureyi_bicimlendir_ms(yukleme_ms)
            );
        }

        if uygulama.yapilandirma.canli_izleme {
            uygulama._izleyici = indexer::izlemeyi_baslat(&kokler, degisiklik_tx).ok();
        }
        let _ = search::arama_gorevlisi_baslat(indeks, canli, arama_istek_rx, arama_sonuc_tx);

        uygulama
    }

    /// Arka planda tam taramayı başlatır; çalışan önceki taramayı iptal eder.
    fn tarama_tetikle(&mut self, kokler: &[PathBuf]) {
        self.tarama_iptal.store(true, Ordering::Relaxed);
        let iptal = Arc::new(AtomicBool::new(false));
        self.tarama_iptal = iptal.clone();

        self.tarama.sayi.store(0, Ordering::Relaxed);
        self.tarama.klasor.store(0, Ordering::Relaxed);
        self.tarama.dizin.store(0, Ordering::Relaxed);
        self.tarama.bitti.store(false, Ordering::Relaxed);
        self.taraniyor.store(true, Ordering::Relaxed);
        self.durum_mesaji = String::from("Tarama başlatıldı...");

        let durum = self.tarama.clone();
        let indeks = Arc::clone(&self.indeks);
        let canli = Arc::clone(&self.canli);
        let taraniyor = Arc::clone(&self.taraniyor);
        let nesil = Arc::clone(&self.nesil);
        let yol = self.indeks_yolu.clone();
        let ayar_hash = self.ayar_hash;
        let kokler = kokler.to_vec();
        let haric = self.yapilandirma.haric.clone();

        std::thread::spawn(move || {
            let indek_uret = indexer::indeks_tara(&kokler, &haric, &durum, &iptal, ayar_hash);
            durum.bitti.store(true, Ordering::Relaxed);
            taraniyor.store(false, Ordering::Relaxed);
            nesil.fetch_add(1, Ordering::Relaxed);

            if let Ok(mut kilit) = canli.write() {
                kilit.temizle();
            }

            if indek_uret.kayit_sayisi() > 0 {
                if let Err(hata) = indek_uret.yaz(&yol) {
                    eprintln!("indeks yazılamadı: {hata:#}");
                }
                if let Ok(mut kilit) = indeks.write() {
                    *kilit = Arc::new(indek_uret);
                }
            }
        });
    }

    /// Kanallardan gelen arama sonuçlarını ve değişiklikleri işler.
    fn kanallari_yokla(&mut self) {
        match self.arama_sonuc_rx.try_recv() {
            Ok(sonuclar) => {
                self.sonuclar = sonuclar;
                self.satirlari_kur();
                self.secili = None;
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => {}
        }

        // Tarama bitti: sonuçlar eski indekse ait olabilir.
        let nesil = self.nesil.load(Ordering::Relaxed);
        if nesil != self.son_nesil {
            self.son_nesil = nesil;
            self.aramayi_tetikle();
            self.durum_mesaji = format!("İndeks yenilendi: {} kayıt", self.indeks_kayit_sayisi());
        }

        let mut degisiklik_var = false;
        while let Ok(d) = self.degisiklik_rx.try_recv() {
            if let Ok(mut katman) = self.canli.write() {
                katman.kaydet(&d);
                if katman.asildi_mi() && !self.taraniyor.load(Ordering::Relaxed) {
                    degisiklik_var = true;
                }
            }
        }
        if degisiklik_var {
            self.tarama_tetikle(&self.yapilandirma.etkin_kokler());
        } else if !self.sorgu_metni.trim().is_empty() {
            self.aramayi_tetikle();
        }
    }

    /// Güncel sorguyu arama görevlisine gönderir.
    fn aramayi_tetikle(&mut self) {
        let sorgu = self.sorgu_metni.clone();
        if sorgu.trim().is_empty() {
            return;
        }
        self.son_sorgu = sorgu.clone();
        let _ = self.arama_istek_tx.send(AramaIstegi {
            sorgu,
            limit: self.yapilandirma.sonuc_limiti,
        });
    }

    /// Sonuç vektöründen çizilecek satır listesini kurar.
    fn satirlari_kur(&mut self) {
        self.satirlar.clear();
        self.satirlar.extend(
            self.sonuclar
                .canli
                .iter()
                .enumerate()
                .map(|(i, _)| Satir::Canli(i)),
        );
        self.satirlar.extend(
            self.sonuclar
                .indeks
                .iter()
                .take(self.yapilandirma.sonuc_limiti)
                .map(|konum| Satir::Indeks(*konum)),
        );
    }

    /// İndeksteki kayıt sayısı.
    fn indeks_kayit_sayisi(&self) -> usize {
        self.indeks
            .read()
            .map(|kilit| kilit.kayit_sayisi())
            .unwrap_or(0)
    }

    /// Tamamlanan eylemin durum çubuğu mesajını üretir.
    fn eylemi_uygula(eylem: Eylem) -> String {
        match eylem {
            Eylem::Ac(yol) => actions::dosyayi_ac(&yol)
                .map(|()| format!("Açıldı: {}", yol.display()))
                .unwrap_or_else(|hata| format!("Açılamadı: {hata:#}")),
            Eylem::KonumuAc(yol) => actions::konumu_ac(&yol)
                .map(|()| format!("Konum açıldı: {}", yol.display()))
                .unwrap_or_else(|hata| format!("Konum açılamadı: {hata:#}")),
            Eylem::YoluKopyala(yol) => actions::panoya_yolu_kopyala(&yol)
                .map(|()| format!("Yol kopyalandı: {}", yol.display()))
                .unwrap_or_else(|hata| format!("Kopyalanamadı: {hata:#}")),
        }
    }
}

impl eframe::App for InEverythingApp {
    fn update(&mut self, ctx: &egui::Context, _cerceve: &mut eframe::Frame) {
        self.kanallari_yokla();
        ctx.request_repaint_after(Duration::from_millis(250));

        let taraniyor = self.taraniyor.load(Ordering::Relaxed);
        let taranan = self.tarama.sayi.load(Ordering::Relaxed);
        let klasor_sayisi = self.tarama.klasor.load(Ordering::Relaxed);
        let dizin_sayisi = self.tarama.dizin.load(Ordering::Relaxed);
        let indeks_kayit = self.indeks_kayit_sayisi();

        egui::TopBottomPanel::top("arama_cubugu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Metin kutusu tüm genişliği alırsa yanındaki düğme kırpılır;
                // düğme için yer ayırılır.
                let buton_yeri = 150.0;
                let yanit = ui.add(
                    egui::TextEdit::singleline(&mut self.sorgu_metni)
                        .hint_text("Dosya ara: rapor.pdf, *.png, *.pdf rapor ...")
                        .desired_width((ui.available_width() - buton_yeri).max(200.0)),
                );
                if self.ilk_cerceve {
                    yanit.request_focus();
                    self.ilk_cerceve = false;
                }
                if yanit.changed() {
                    if self.sorgu_metni.trim().is_empty() {
                        self.sonuclar = Sonuclar::default();
                        self.satirlari_kur();
                        self.secili = None;
                    } else {
                        self.aramayi_tetikle();
                    }
                }
                let dugme = ui
                    .add_enabled(
                        !taraniyor,
                        egui::Button::new(if taraniyor {
                            "Taranıyor..."
                        } else {
                            "Yeniden Tara"
                        }),
                    )
                    .on_disabled_hover_text("Tarama sürüyor");
                if dugme.clicked() {
                    self.tarama_tetikle(&self.yapilandirma.etkin_kokler());
                }
            });
            ui.horizontal(|ui| {
                if taraniyor {
                    ui.label(format!(
                        "Taranıyor... {taranan} dosya, {klasor_sayisi} klasör, {dizin_sayisi} dizin"
                    ));
                } else {
                    let (tarama_ms, indeks_boyut) = self
                        .indeks
                        .read()
                        .map(|k| (k.tarama_suresi_ms, k.boyut()))
                        .unwrap_or((0, 0));
                    ui.label(format!(
                        "{indeks_kayit} kayıt · son tarama {} · {}",
                        sureyi_bicimlendir_ms(tarama_ms as f64),
                        boyutu_bicimlendir(indeks_boyut as u64)
                    ));
                }
                ui.separator();
                ui.label(format!("• {0} sonuç", self.sonuclar.toplam()));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let secili_konum = self.secili;
            let mut secili_yeni = secili_konum;
            let mut eylem: Option<Eylem> = None;
            let satirlar = std::mem::take(&mut self.satirlar);

            if self.sorgu_metni.trim().is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("Aramaya başlamak için yukarıya dosya adı yazın");
                });
            } else {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show_rows(ui, 26.0, satirlar.len(), |ui, aralik| {
                        let okunan = self.indeks.read();
                        let kilit = match okunan.as_deref() {
                            Ok(kilit) => kilit,
                            Err(_) => return,
                        };
                        for satir in aralik {
                            let Some(satir) = satirlar.get(satir).copied() else {
                                continue;
                            };
                            let Some(veri) = satir_verisi(&self.sonuclar, &satir, kilit) else {
                                continue;
                            };
                            ui.horizontal(|ui| {
                                ui.set_width(ui.available_width());
                                let yanit = ui.selectable_label(
                                    secili_konum == Some(satir_konumu(&satir)),
                                    veri.ad.as_str(),
                                );
                                if yanit.clicked() {
                                    secili_yeni = Some(satir_konumu(&satir));
                                }
                                if yanit.double_clicked() {
                                    eylem = Some(Eylem::Ac(PathBuf::from(&veri.yol)));
                                }
                                yanit.context_menu(|ui| {
                                    if ui.button("Aç").clicked() {
                                        eylem = Some(Eylem::Ac(PathBuf::from(&veri.yol)));
                                        ui.close_menu();
                                    }
                                    if ui.button("Dosya konumunu aç").clicked() {
                                        eylem = Some(Eylem::KonumuAc(PathBuf::from(&veri.yol)));
                                        ui.close_menu();
                                    }
                                    if ui.button("Yolu kopyala").clicked() {
                                        eylem = Some(Eylem::YoluKopyala(PathBuf::from(&veri.yol)));
                                        ui.close_menu();
                                    }
                                });
                                ui.label(kisalt(&veri.yol, 70));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(zamani_bicimlendir(veri.zaman));
                                        ui.label(if veri.klasor {
                                            String::from("klasör")
                                        } else {
                                            boyutu_bicimlendir(veri.boyut)
                                        });
                                    },
                                );
                            });
                        }
                    });
            }

            self.satirlar = satirlar;
            self.secili = secili_yeni;
            if let Some(secilen) = eylem {
                self.durum_mesaji = Self::eylemi_uygula(secilen);
            }
        });

        egui::TopBottomPanel::bottom("durum_cubugu").show(ctx, |ui| {
            ui.label(&self.durum_mesaji);
        });
    }
}

/// Satırın benzersiz konumu (seçili satırın korunması için).
fn satir_konumu(satir: &Satir) -> usize {
    match satir {
        Satir::Indeks(konum) => *konum,
        Satir::Canli(konum) => usize::MAX - *konum,
    }
}

/// Satırın çizim için gereken alanlarını toplar.
///
/// Kaynak artık mevcut değilse (indeks yenilendi, satır sınır dışı kaldı)
/// `None` döner ve satır çizilmez.
fn satir_verisi(sonuclar: &Sonuclar, satir: &Satir, kilit: &Indeks) -> Option<SatirVerisi> {
    match satir {
        Satir::Indeks(konum) => {
            let oge = kilit.kayit(*konum)?;
            Some(SatirVerisi {
                ad: oge.ad.to_string(),
                yol: oge.yol.to_string(),
                boyut: oge.boyut,
                zaman: oge.degistirilme,
                klasor: oge.klasor_mu,
            })
        }
        Satir::Canli(konum) => {
            let oge = sonuclar.canli.get(*konum)?;
            Some(SatirVerisi {
                ad: oge.ad.clone(),
                yol: oge.yol.to_string_lossy().into_owned(),
                boyut: oge.boyut,
                zaman: oge.degistirilme,
                klasor: oge.klasor_mu,
            })
        }
    }
}

/// Henüz hiç indeks yokken kullanılan boş indeks.
fn bos_indeks() -> Indeks {
    crate::depo::KayitYazici::yeni().indeks_uret(0, 0, 0)
}

/// Baytı okunabilir boyuta çevirir (`1536` -> `1.5 KB`).
pub fn boyutu_bicimlendir(bayt: u64) -> String {
    const BIRIMLER: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut deger = bayt as f64;
    let mut birim = 0;
    while deger >= 1024.0 && birim < BIRIMLER.len() - 1 {
        deger /= 1024.0;
        birim += 1;
    }
    if birim == 0 {
        format!("{} B", bayt)
    } else {
        format!("{deger:.1} {}", BIRIMLER[birim])
    }
}

/// Milisaniyeyi okunabilir süreye çevirir (açılış ve tarama süreleri için).
///
/// Çok kısa sürelerde (`<10 ms`) onda bir hassasiyet, uzunlarda saniye
/// gösterilir; 0,1 ms'nin altındaki ölçümler `0 ms` olur.
pub fn sureyi_bicimlendir_ms(ms: f64) -> String {
    if ms >= 1000.0 {
        format!("{:.1} sn", ms / 1000.0)
    } else if ms >= 10.0 {
        format!("{ms:.0} ms")
    } else {
        format!("{ms:.1} ms")
    }
}

/// Zamanı göreli metne çevirir (`5 dk önce`), bilinmiyorsa `-`.
pub fn zamani_bicimlendir(zaman: Option<SystemTime>) -> String {
    let Some(an) = zaman else {
        return String::from("-");
    };
    let Ok(gecen) = an.elapsed() else {
        return String::from("-");
    };
    let saniye = gecen.as_secs();
    if saniye < 60 {
        String::from("az önce")
    } else if saniye < 3600 {
        format!("{} dk önce", saniye / 60)
    } else if saniye < 86_400 {
        format!("{} sa önce", saniye / 3600)
    } else {
        format!("{} g önce", saniye / 86_400)
    }
}

/// Uzun metni karakter bazında kısaltır, sonuna `...` ekler.
pub fn kisalt(metin: &str, en_fazla: usize) -> String {
    if metin.chars().count() <= en_fazla || en_fazla <= 3 {
        return metin.to_string();
    }
    let kesilmis: String = metin.chars().take(en_fazla - 3).collect();
    format!("{kesilmis}...")
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn boyut_birimleri_dogru() {
        assert_eq!(boyutu_bicimlendir(0), "0 B");
        assert_eq!(boyutu_bicimlendir(512), "512 B");
        assert_eq!(boyutu_bicimlendir(1536), "1.5 KB");
        assert_eq!(boyutu_bicimlendir(1024 * 1024), "1.0 MB");
        assert_eq!(boyutu_bicimlendir(5 * 1024 * 1024 * 1024), "5.0 GB");
    }

    #[test]
    fn zaman_bilinmiyorsa_tire() {
        assert_eq!(zamani_bicimlendir(None), "-");
        let yakin = SystemTime::now() - Duration::from_secs(10);
        assert_eq!(zamani_bicimlendir(Some(yakin)), "az önce");
        let saatler = SystemTime::now() - Duration::from_secs(3 * 3600);
        assert_eq!(zamani_bicimlendir(Some(saatler)), "3 sa önce");
    }

    #[test]
    fn kisa_metin_aynen_kalir() {
        assert_eq!(kisalt("abc", 10), "abc");
        assert_eq!(kisalt("abcdef", 6), "abcdef");
        assert_eq!(kisalt("abcdef", 5), "ab...");
    }

    #[test]
    fn satir_konumlari_birbirine_carismaz() {
        assert_eq!(satir_konumu(&Satir::Indeks(0)), 0);
        assert_eq!(satir_konumu(&Satir::Canli(0)), usize::MAX);
        assert_ne!(
            satir_konumu(&Satir::Indeks(5)),
            satir_konumu(&Satir::Canli(5))
        );
    }

    #[test]
    fn satir_verisi_indeksten_gelir() {
        let mut yazici = crate::depo::KayitYazici::yeni();
        yazici.ekle(b"/home/rapor.pdf", 42, None, false);
        let indeks = yazici.indeks_uret(1, 1, 0);
        let sonuclar = Sonuclar::default();

        let veri = satir_verisi(&sonuclar, &Satir::Indeks(0), &indeks).expect("veri");
        assert_eq!(veri.ad, "rapor.pdf");
        assert_eq!(veri.yol, "/home/rapor.pdf");
        assert_eq!(veri.boyut, 42);
        assert!(!veri.klasor);

        assert!(
            satir_verisi(&sonuclar, &Satir::Indeks(7), &indeks).is_none(),
            "sınır dışı konum None dönmeli"
        );
    }

    #[test]
    fn satir_verisi_canli_katmandan_gelir() {
        use crate::model::FileItem;
        let sonuclar = Sonuclar {
            indeks: Vec::new(),
            canli: vec![FileItem::dosya(PathBuf::from("/yeni/notlar.txt"), 7, None)],
        };
        let mut yazici = crate::depo::KayitYazici::yeni();
        yazici.ekle(b"/eski/x.txt", 1, None, false);
        let indeks = yazici.indeks_uret(1, 1, 0);

        let veri = satir_verisi(&sonuclar, &Satir::Canli(0), &indeks).expect("veri");
        assert_eq!(veri.ad, "notlar.txt");
        assert_eq!(veri.boyut, 7);
        assert!(satir_verisi(&sonuclar, &Satir::Canli(3), &indeks).is_none());
    }

    #[test]
    fn bos_indeks_kayit_icermez() {
        assert_eq!(bos_indeks().kayit_sayisi(), 0);
    }

    #[test]
    fn sure_bicimlendirme_birimleri_dogru() {
        assert_eq!(sureyi_bicimlendir_ms(0.081), "0.1 ms");
        assert_eq!(sureyi_bicimlendir_ms(8.4), "8.4 ms");
        assert_eq!(sureyi_bicimlendir_ms(42.4), "42 ms");
        assert_eq!(sureyi_bicimlendir_ms(14412.0), "14.4 sn");
        assert_eq!(sureyi_bicimlendir_ms(666.6), "667 ms");
    }

    #[test]
    fn boyut_bicimlendirme_birimleri_dogru() {
        assert_eq!(boyutu_bicimlendir(512), "512 B");
        assert_eq!(boyutu_bicimlendir(1536), "1.5 KB");
        assert_eq!(boyutu_bicimlendir(666_556_830), "635.7 MB");
    }
}
