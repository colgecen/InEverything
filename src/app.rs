//! Grafik arayüz: eframe/egui tabanlı ana pencere.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};
use std::time::{Duration, SystemTime};

use crossbeam_channel::{Receiver, Sender, TryRecvError};
use eframe::egui;

use crate::{
    actions,
    config::AppConfig,
    indexer::{self, CanliIzleyici, Degisiklik, TaramaDurumu},
    model::FileItem,
    search,
};

/// Satırda tetiklenen dosya eylemi.
enum Eylem {
    Ac(std::path::PathBuf),
    KonumuAc(std::path::PathBuf),
    YoluKopyala(std::path::PathBuf),
}

/// Ana uygulama durumu.
pub struct FastFindApp {
    sorgu_metni: String,
    indeks: Arc<RwLock<Vec<FileItem>>>,
    tarama: TaramaDurumu,
    sonuclar: Vec<usize>,
    secili: Option<usize>,
    ilk_cerceve: bool,
    durum_mesaji: String,
    yapilandirma: AppConfig,
    arama_istek_tx: Sender<String>,
    arama_sonuc_rx: Receiver<Vec<usize>>,
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
        "FastFind",
        secenekler,
        Box::new(|baglam| Ok(Box::new(FastFindApp::yeni(baglam)))),
    )
}

impl FastFindApp {
    /// İlk kurulum: yapılandırma, kanallar, tarama ve arama görevlileri.
    pub fn yeni(baglam: &eframe::CreationContext<'_>) -> Self {
        baglam.egui_ctx.set_visuals(egui::Visuals::dark());
        let yapilandirma =
            AppConfig::yukle_yoldan(&AppConfig::varsayilan_yol()).unwrap_or_default();
        let kokler = if yapilandirma.kokler.is_empty() {
            indexer::suruculeri_bul()
        } else {
            yapilandirma.kokler.clone()
        };

        let indeks = Arc::new(RwLock::new(Vec::new()));
        let tarama = TaramaDurumu::default();
        let (arama_istek_tx, arama_istek_rx) = crossbeam_channel::bounded::<String>(32);
        let (arama_sonuc_tx, arama_sonuc_rx) = crossbeam_channel::bounded::<Vec<usize>>(32);
        let (degisiklik_tx, degisiklik_rx) = crossbeam_channel::unbounded::<Degisiklik>();

        {
            let indeks_yaz = Arc::clone(&indeks);
            let durum_yaz = tarama.clone();
            std::thread::spawn(move || {
                let iptal = AtomicBool::new(false);
                let bulunan = indexer::sistemi_tara(&kokler, &durum_yaz, &iptal);
                if let Ok(mut kilit) = indeks_yaz.write() {
                    *kilit = bulunan;
                }
            });
        }

        let _izleyici = indexer::izlemeyi_baslat(&kokler, degisiklik_tx).ok();
        let _ = search::arama_gorevlisi_baslat(Arc::clone(&indeks), arama_istek_rx, arama_sonuc_tx);

        Self {
            sorgu_metni: String::new(),
            indeks,
            tarama,
            sonuclar: Vec::new(),
            secili: None,
            ilk_cerceve: true,
            durum_mesaji: String::from("Dizin taranıyor..."),
            yapilandirma,
            arama_istek_tx,
            arama_sonuc_rx,
            degisiklik_rx,
            _izleyici,
        }
    }

    /// Kanallardan gelen arama sonuçlarını ve değişiklikleri işler.
    fn kanallari_yokla(&mut self) {
        match self.arama_sonuc_rx.try_recv() {
            Ok(sonuclar) => {
                self.sonuclar = sonuclar;
                self.secili = None;
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => {}
        }
        let mut degisiklik_var = false;
        while self.degisiklik_rx.try_recv().is_ok() {
            degisiklik_var = true;
        }
        if degisiklik_var && !self.sorgu_metni.trim().is_empty() {
            let _ = self.arama_istek_tx.send(self.sorgu_metni.clone());
        }
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

impl eframe::App for FastFindApp {
    fn update(&mut self, ctx: &egui::Context, _cerceve: &mut eframe::Frame) {
        self.kanallari_yokla();
        ctx.request_repaint_after(Duration::from_millis(250));

        let dizindeki_sayi = self.indeks.read().map(|kilit| kilit.len()).unwrap_or(0);
        let tarama_bitti = self.tarama.bitti.load(Ordering::Relaxed);
        let taranan = self.tarama.sayi.load(Ordering::Relaxed);

        egui::TopBottomPanel::top("arama_cubugu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let yanit = ui.add(
                    egui::TextEdit::singleline(&mut self.sorgu_metni)
                        .hint_text("Dosya ara: rapor.pdf, *.png, *.pdf rapor ...")
                        .desired_width(f32::INFINITY),
                );
                if self.ilk_cerceve {
                    yanit.request_focus();
                    self.ilk_cerceve = false;
                }
                if yanit.changed() {
                    if self.sorgu_metni.trim().is_empty() {
                        self.sonuclar.clear();
                    } else {
                        let _ = self.arama_istek_tx.send(self.sorgu_metni.clone());
                    }
                }
            });
            ui.horizontal(|ui| {
                if tarama_bitti {
                    ui.label(format!("{dizindeki_sayi} dosya dizinde"));
                } else {
                    ui.label(format!("Taranıyor... {taranan} dosya"));
                }
                ui.label(format!("• {0} sonuç", self.sonuclar.len()));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let satirlar: Vec<usize> = self
                .sonuclar
                .iter()
                .take(self.yapilandirma.sonuc_limiti)
                .copied()
                .collect();
            let secili_konum = self.secili;
            let mut secili_yeni = secili_konum;
            let mut eylem: Option<Eylem> = None;

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show_rows(ui, 26.0, satirlar.len(), |ui, aralik| {
                    let okunan = self.indeks.read();
                    let kilit = match okunan.as_deref() {
                        Ok(kilit) => kilit,
                        Err(_) => return,
                    };
                    for satir in aralik {
                        let konum = satirlar[satir];
                        let Some(oge) = kilit.get(konum) else {
                            continue;
                        };
                        ui.horizontal(|ui| {
                            ui.set_width(ui.available_width());
                            let yanit =
                                ui.selectable_label(secili_konum == Some(konum), oge.ad.as_str());
                            if yanit.clicked() {
                                secili_yeni = Some(konum);
                            }
                            if yanit.double_clicked() {
                                eylem = Some(Eylem::Ac(oge.yol.clone()));
                            }
                            yanit.context_menu(|ui| {
                                if ui.button("Aç").clicked() {
                                    eylem = Some(Eylem::Ac(oge.yol.clone()));
                                    ui.close_menu();
                                }
                                if ui.button("Dosya konumunu aç").clicked() {
                                    eylem = Some(Eylem::KonumuAc(oge.yol.clone()));
                                    ui.close_menu();
                                }
                                if ui.button("Yolu kopyala").clicked() {
                                    eylem = Some(Eylem::YoluKopyala(oge.yol.clone()));
                                    ui.close_menu();
                                }
                            });
                            ui.label(kisalt(&oge.yol.display().to_string(), 70));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(zamani_bicimlendir(oge.degistirilme));
                                    ui.label(boyutu_bicimlendir(oge.boyut));
                                },
                            );
                        });
                    }
                });

            self.secili = secili_yeni;
            if let Some(secilen) = eylem {
                self.durum_mesaji = Self::eylemi_uygula(secilen);
            }
            if self.sorgu_metni.trim().is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("Aramaya başlamak için yukarıya dosya adı yazın");
                });
            }
        });

        egui::TopBottomPanel::bottom("durum_cubugu").show(ctx, |ui| {
            ui.label(&self.durum_mesaji);
        });
    }
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
}
