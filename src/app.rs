//! Grafik arayüz: eframe/egui tabanlı ana pencere, futuristik tema ile.
//!
//! Açılışta diskteki indeks anında belleğe alınır (yüz binlerce kayıt için
//! onlarca milisaniye), tarama yalnızca indeks bayatsa arka planda koşar.
//! Renk, tipografi ve parıltı efektleri `tema` modülünden gelir; bu modül
//! yalnızca düzeni ve etkileşimi kurar.

use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, RwLock,
};
use std::time::{Duration, SystemTime};

use crossbeam_channel::{Receiver, Sender, TryRecvError};
use eframe::egui::{self, Align2, Color32, Pos2, Rect, Rounding, Sense, Vec2};

use crate::{
    actions,
    config::AppConfig,
    depo::Indeks,
    indexer::{self, CanliIzleyici, Degisiklik, TaramaDurumu},
    search::{self, AramaIstegi, CanliKatman, Sonuclar},
    tema,
};

/// Sonuç satırının yüksekliği (satır aralığı hariç).
const SATIR_Y: f32 = 30.0;
/// Sol kenardaki renkli dosya işaretçisinin sol x konumu.
const ISARET_X: f32 = 15.0;
/// Dosya adının başladığı x konumu.
const AD_X: f32 = 36.0;
/// Sağa hizalı sütunların sağ kenar payı.
const SAG_PAY: f32 = 16.0;
/// Boyut sütununun zaman sütununa göre geride kaldığı miktar.
const SUTUN_ARASI: f32 = 96.0;

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

/// Üst durum satırının gösterdiği tarama sayaçları.
struct TaramaOzeti {
    taranan: usize,
    klasor: usize,
    dizin: usize,
}

/// Sonuç listesindeki sütunların x konumları.
///
/// Pencere daraldıkça dosya adı sütununun payı azalır, böylece yol ve
/// sağdaki sayısal sütunlar ezilmez.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Sutunlar {
    ad: f32,
    yol: f32,
    yol_en: f32,
    boyut: f32,
    zaman: f32,
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
    /// İlk kurulum: tema, yapılandırma, indeks yükleme, kanallar, izleyici.
    pub fn yeni(baglam: &eframe::CreationContext<'_>) -> Self {
        tema::uygula(&baglam.egui_ctx);
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

    /// Başlık satırı: elmas logo, uygulama adı ve sağdaki bilgi çipleri.
    fn baslik_satiri(&self, ui: &mut egui::Ui) {
        let yukseklik = 54.0;
        let (yanit, p) =
            ui.allocate_painter(Vec2::new(ui.available_width(), yukseklik), Sense::hover());
        let alan = yanit.rect;
        let dikey = alan.center().y - 7.0;

        // elmas logo
        let merkez = Pos2::new(alan.left() + 22.0, dikey);
        let kose = vec![
            Pos2::new(merkez.x, merkez.y - 16.0),
            Pos2::new(merkez.x + 13.0, merkez.y),
            Pos2::new(merkez.x, merkez.y + 16.0),
            Pos2::new(merkez.x - 13.0, merkez.y),
        ];
        p.add(egui::Shape::closed_line(
            kose,
            tema::kontur(1.8, tema::NEON.gamma_multiply(0.9)),
        ));
        tema::nokta(&p, merkez, 3.2, tema::NEON);

        // uygulama adı ve alt başlık
        let sol = alan.left() + 48.0;
        tema::neon_yazi(
            &p,
            Pos2::new(sol, dikey - 3.0),
            Align2::LEFT_CENTER,
            tema::ISIM,
            tema::kalin(19.0),
            tema::NEON,
            0.55,
        );
        p.text(
            Pos2::new(sol + 1.0, dikey + 18.0),
            Align2::LEFT_CENTER,
            "ULTRA HIZLI DOSYA ARAMA MOTORU",
            tema::mono(10.0),
            tema::METIN_3,
        );

        // sağdaki çipler
        let (kayit, boyut, tarama_ms) = self
            .indeks
            .read()
            .map(|k| (k.kayit_sayisi(), k.boyut(), k.tarama_suresi_ms))
            .unwrap_or((0, 0, 0));
        let mut sag = alan.right();
        let dikey_cip = alan.center().y;
        tema::cip_sagdan(
            ui,
            &p,
            &mut sag,
            dikey_cip,
            &format!("{kayit} KAYIT"),
            tema::NEON,
            22.0,
        );
        tema::cip_sagdan(
            ui,
            &p,
            &mut sag,
            dikey_cip,
            &boyutu_bicimlendir(boyut as u64),
            tema::MOR,
            22.0,
        );
        tema::cip_sagdan(
            ui,
            &p,
            &mut sag,
            dikey_cip,
            &format!("SON TARAMA {}", sureyi_bicimlendir_ms(tarama_ms as f64)),
            tema::TURUNCU,
            22.0,
        );
    }

    /// Arama kutusu ve tarama düğmesi satırı.
    fn arama_satiri(&mut self, ui: &mut egui::Ui, taraniyor: bool) {
        let yukseklik = 46.0;
        let buton_en = 178.0;
        let genislik = ui.available_width();
        let bas = ui.cursor().min;
        let kutu_en = (genislik - buton_en - 16.0).max(160.0);
        let kutu = Rect::from_min_size(bas, Vec2::new(kutu_en, yukseklik));
        let buton = Rect::from_min_size(
            Pos2::new(kutu.right() + 16.0, bas.y),
            Vec2::new(buton_en, yukseklik),
        );

        // kutunun zemini ve büyüteç imlesi (metnin altında kalır)
        ui.painter()
            .rect_filled(kutu, Rounding::same(12.0), tema::YUZEY_2);
        let merkez = Pos2::new(kutu.left() + 24.0, kutu.center().y);
        ui.painter().circle_stroke(
            merkez,
            7.5,
            tema::kontur(2.0, tema::NEON.gamma_multiply(0.8)),
        );
        ui.painter().line_segment(
            [
                Pos2::new(merkez.x + 5.4, merkez.y + 5.4),
                Pos2::new(merkez.x + 11.0, merkez.y + 11.0),
            ],
            tema::kontur(2.4, tema::NEON.gamma_multiply(0.8)),
        );

        let yanit = ui.put(
            kutu,
            egui::TextEdit::singleline(&mut self.sorgu_metni)
                .frame(false)
                .font(tema::mono(15.0))
                .text_color(tema::METIN)
                .hint_text(
                    egui::RichText::new("dosya ara — rapor.pdf, *.png, *belgeler* …")
                        .font(tema::mono(13.5))
                        .color(tema::METIN_3),
                )
                .vertical_align(egui::Align::Center)
                .margin(egui::Margin {
                    left: 46.0,
                    right: 14.0,
                    top: 0.0,
                    bottom: 0.0,
                })
                .desired_width(kutu_en)
                .min_size(Vec2::new(0.0, yukseklik)),
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

        // çerçeve: odaktayken neon parıltısı, değilse sakin kenar
        if yanit.has_focus() {
            tema::neon_cerceve(ui.painter(), kutu, 12.0, tema::NEON, 1.0);
        } else if yanit.hovered() {
            ui.painter().rect_stroke(
                kutu,
                Rounding::same(12.0),
                tema::kontur(1.3, tema::NEON.gamma_multiply(0.55)),
            );
        } else {
            ui.painter()
                .rect_stroke(kutu, Rounding::same(12.0), tema::kontur(1.0, tema::CETVEL));
        }

        // tarama düğmesi
        let yanit = ui.interact(buton, ui.id().with("yeniden_tara"), Sense::click());
        let yanit = if taraniyor {
            yanit.on_hover_text("Tarama sürüyor")
        } else {
            yanit
        };
        let ugrunda = yanit.hovered() && !taraniyor;
        let dolgu = if taraniyor {
            tema::MOR.gamma_multiply(0.12)
        } else if ugrunda {
            tema::NEON.gamma_multiply(0.24)
        } else {
            tema::NEON.gamma_multiply(0.10)
        };
        let kenar = if taraniyor {
            tema::MOR.gamma_multiply(0.45)
        } else {
            tema::NEON.gamma_multiply(0.75)
        };
        ui.painter().rect_filled(buton, Rounding::same(12.0), dolgu);
        ui.painter()
            .rect_stroke(buton, Rounding::same(12.0), tema::kontur(1.4, kenar));
        if ugrunda {
            tema::neon_cerceve(ui.painter(), buton, 12.0, tema::NEON, 0.75);
        }
        let etiket = if taraniyor {
            "TARANIYOR …"
        } else {
            "⟳  YENİDEN TARA"
        };
        let renk = if taraniyor {
            tema::MOR.gamma_multiply(0.85)
        } else {
            tema::NEON
        };
        tema::neon_yazi(
            ui.painter(),
            buton.center(),
            Align2::CENTER_CENTER,
            etiket,
            tema::kalin(13.0),
            renk,
            if taraniyor { 0.3 } else { 0.5 },
        );
        if !taraniyor && yanit.clicked() {
            self.tarama_tetikle(&self.yapilandirma.etkin_kokler());
        }
    }

    /// Arama kutusunun altındaki durum satırı: canlı sayaç ve sağ çipler.
    fn ust_durum_satiri(
        &self,
        ui: &mut egui::Ui,
        zaman: f32,
        taraniyor: bool,
        ozet: &TaramaOzeti,
        indeks_kayit: usize,
    ) {
        let yukseklik = 26.0;
        let (yanit, p) =
            ui.allocate_painter(Vec2::new(ui.available_width(), yukseklik), Sense::hover());
        let alan = yanit.rect;
        let renk = if taraniyor {
            tema::TURUNCU
        } else {
            tema::YESIL
        };
        let nabiz = 0.70 + 0.30 * (zaman * 3.0).sin();
        tema::nokta(
            &p,
            Pos2::new(alan.left() + 5.0, alan.center().y),
            3.0 * nabiz,
            renk,
        );

        let metin = if taraniyor {
            format!(
                "TARANIYOR · {} dosya · {} klasör · {} dizin",
                ozet.taranan, ozet.klasor, ozet.dizin
            )
        } else {
            format!(
                "INDEKS HAZIR · {indeks_kayit} kayıt · sonuç {}",
                self.sonuclar.toplam()
            )
        };
        p.text(
            Pos2::new(alan.left() + 18.0, alan.center().y),
            Align2::LEFT_CENTER,
            metin,
            tema::mono(12.0),
            tema::METIN_2,
        );

        let mut sag = alan.right();
        let dikey = alan.center().y;
        let (etiket, renk) = if self.yapilandirma.canli_izleme {
            ("CANLI İZLEME", tema::YESIL)
        } else {
            ("CANLI KAPALI", tema::METIN_3)
        };
        tema::cip_sagdan(ui, &p, &mut sag, dikey, etiket, renk, 20.0);
    }

    /// Başlığın altında uzanan, tarama sırasında üzerinde ışık koşan çizgi.
    fn isin_cizgisi(&self, ui: &mut egui::Ui, zaman: f32, taraniyor: bool) {
        let (yanit, p) = ui.allocate_painter(Vec2::new(ui.available_width(), 5.0), Sense::hover());
        let alan = yanit.rect;
        let yukseklik = 1.5;
        let adim = 7.0;
        let mut x = alan.left();
        while x < alan.right() {
            let t = ((x - alan.left()) / alan.width().max(1.0)).clamp(0.0, 1.0);
            let sag = (x + adim - 2.0).min(alan.right());
            p.rect_filled(
                Rect::from_min_max(
                    Pos2::new(x, alan.bottom() - yukseklik),
                    Pos2::new(sag, alan.bottom()),
                ),
                0.0,
                tema::NEON.gamma_multiply(0.10 + 0.55 * (1.0 - t)),
            );
            x += adim;
        }

        if taraniyor {
            let konum = alan.left() + (zaman * 0.6) % 1.0 * alan.width();
            p.rect_filled(
                Rect::from_min_max(
                    Pos2::new((konum - 64.0).max(alan.left()), alan.bottom() - 2.5),
                    Pos2::new(konum, alan.bottom()),
                ),
                0.0,
                tema::NEON.gamma_multiply(0.85),
            );
            tema::nokta(
                &p,
                Pos2::new(konum, alan.bottom() - 1.5),
                2.2,
                Color32::WHITE,
            );
        }
    }

    /// Sonuç listesinin üstündeki sütun başlıkları.
    fn sutun_basligi(&self, ui: &mut egui::Ui) {
        let (yanit, p) = ui.allocate_painter(Vec2::new(ui.available_width(), 24.0), Sense::hover());
        let alan = yanit.rect;
        let sutunlar = sutunlari_hesapla(alan);
        let font = tema::mono(10.0);
        p.line_segment(
            [
                Pos2::new(alan.left(), alan.bottom() - 0.5),
                Pos2::new(alan.right(), alan.bottom() - 0.5),
            ],
            tema::kontur(1.0, tema::CETVEL.gamma_multiply(0.7)),
        );
        let basliklar = [
            (
                Pos2::new(sutunlar.ad, alan.center().y),
                Align2::LEFT_CENTER,
                "DOSYA ADI",
            ),
            (
                Pos2::new(sutunlar.yol, alan.center().y),
                Align2::LEFT_CENTER,
                "KONUM",
            ),
            (
                Pos2::new(sutunlar.boyut, alan.center().y),
                Align2::RIGHT_CENTER,
                "BOYUT",
            ),
            (
                Pos2::new(sutunlar.zaman, alan.center().y),
                Align2::RIGHT_CENTER,
                "DEĞİŞTİRİLME",
            ),
        ];
        for (konum, hiza, metin) in basliklar {
            p.text(konum, hiza, metin, font.clone(), tema::METIN_3);
        }
    }

    /// Sorgu boşken görünen karşılama ekranı: büyük başlık ve örnek çipler.
    fn bos_durum(&mut self, ui: &mut egui::Ui) {
        let alan = ui.max_rect();
        let merkez = alan.center();
        let p = ui.painter();

        let baslik_y = merkez.y - 70.0;
        tema::neon_yazi(
            p,
            Pos2::new(merkez.x, baslik_y),
            Align2::CENTER_CENTER,
            tema::ISIM,
            tema::kalin(30.0),
            tema::NEON,
            0.5,
        );
        p.text(
            Pos2::new(merkez.x, baslik_y + 34.0),
            Align2::CENTER_CENTER,
            "ULTRA HIZLI DOSYA ARAMA MOTORU",
            tema::mono(11.5),
            tema::METIN_3,
        );
        p.text(
            Pos2::new(merkez.x, merkez.y + 6.0),
            Align2::CENTER_CENTER,
            "aramaya başlamak için yukarıya yazın",
            tema::mono(14.0),
            tema::METIN_2,
        );

        // örnek sorgu çipleri
        let ipuclari = ["*.pdf", "*.png", "rapor", "belgeler", "cmakelists.txt"];
        let font = tema::mono(13.0);
        let olcu = |aday: &str| -> f32 {
            ui.fonts(|f| f.layout_no_wrap(aday.to_owned(), font.clone(), tema::NEON))
                .size()
                .x
        };
        let enler: Vec<f32> = ipuclari.iter().map(|i| olcu(i) + 26.0).collect();
        let bosluk = 10.0;
        let toplam: f32 = enler.iter().sum::<f32>() + bosluk * (enler.len() as f32 - 1.0);
        let mut x = merkez.x - toplam * 0.5;
        let y = merkez.y + 44.0;
        for (sira, ipucu) in ipuclari.iter().enumerate() {
            let en = enler[sira];
            let dikdortgen = Rect::from_min_size(Pos2::new(x, y), Vec2::new(en, 32.0));
            x += en + bosluk;
            let yanit = ui.interact(dikdortgen, ui.id().with(("ipucu", sira)), Sense::click());
            let vurgu = yanit.hovered() || yanit.is_pointer_button_down_on();
            p.rect_filled(
                dikdortgen,
                Rounding::same(16.0),
                tema::NEON.gamma_multiply(if vurgu { 0.18 } else { 0.06 }),
            );
            p.rect_stroke(
                dikdortgen,
                Rounding::same(16.0),
                tema::kontur(
                    1.0,
                    tema::NEON.gamma_multiply(if vurgu { 0.9 } else { 0.35 }),
                ),
            );
            p.text(
                dikdortgen.center(),
                Align2::CENTER_CENTER,
                *ipucu,
                font.clone(),
                tema::NEON.gamma_multiply(if vurgu { 1.0 } else { 0.72 }),
            );
            if yanit.clicked() {
                self.sorgu_metni = (*ipucu).to_string();
                self.aramayi_tetikle();
            }
        }

        p.text(
            Pos2::new(merkez.x, y + 78.0),
            Align2::CENTER_CENTER,
            "sonuç satırına çift tıklayın · sağ tık ile menü",
            tema::mono(11.0),
            tema::METIN_3,
        );
    }

    /// Sorgu doluyken eşleşme olmadığında görünen ekran.
    fn sonuc_yok(&self, ui: &mut egui::Ui) {
        let alan = ui.max_rect();
        let merkez = alan.center();
        let p = ui.painter();
        tema::neon_yazi(
            p,
            Pos2::new(merkez.x, merkez.y - 14.0),
            Align2::CENTER_CENTER,
            "SONUÇ YOK",
            tema::kalin(22.0),
            tema::PEMBE,
            0.5,
        );
        let sorgu = tema::kes(
            ui,
            self.sorgu_metni.trim(),
            tema::mono(13.0),
            tema::METIN_2,
            (alan.width() - 80.0).max(120.0),
        );
        p.text(
            Pos2::new(merkez.x, merkez.y + 20.0),
            Align2::CENTER_CENTER,
            format!("«{sorgu}» için eşleşme bulunamadı"),
            tema::mono(13.0),
            tema::METIN_2,
        );
    }

    /// Alt durum çubuğu: canlı mesaj (sol) ve kısayol ipuçları (sağ).
    fn alt_cubuk(&self, ui: &mut egui::Ui, taraniyor: bool) {
        let genislik = ui.available_width();
        ui.horizontal(|ui| {
            let renk = if taraniyor {
                tema::TURUNCU
            } else {
                tema::YESIL
            };
            let (yanit, p) = ui.allocate_painter(Vec2::new(14.0, 16.0), Sense::hover());
            tema::nokta(&p, yanit.rect.center(), 3.0, renk);

            let mesaj = tema::kes(
                ui,
                &self.durum_mesaji,
                tema::mono(12.5),
                tema::METIN_2,
                (genislik - 330.0).max(90.0),
            );
            ui.label(
                egui::RichText::new(mesaj)
                    .font(tema::mono(12.5))
                    .color(tema::METIN_2),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new("sağ tık menü · ⏎ aç · çift tık konum")
                        .font(tema::mono(11.0))
                        .color(tema::METIN_3),
                );
                ui.label(
                    egui::RichText::new(format!("{} sonuç", self.sonuclar.toplam()))
                        .font(tema::mono(11.5))
                        .color(tema::NEON),
                );
            });
        });
    }

    /// Merkez panel: arka plan, başlıklar ve sonuç satırları.
    fn icerik(&mut self, ui: &mut egui::Ui, zaman: f32, taraniyor: bool) {
        let alan = ui.max_rect();
        tema::arka_plan(ui.painter(), alan, zaman, taraniyor);

        let secili_konum = self.secili;
        let mut secili_yeni = secili_konum;
        let mut eylem: Option<Eylem> = None;
        let satirlar = std::mem::take(&mut self.satirlar);
        let bos_sorgu = self.sorgu_metni.trim().is_empty();

        if bos_sorgu {
            self.bos_durum(ui);
        } else {
            self.sutun_basligi(ui);
            if satirlar.is_empty() {
                self.sonuc_yok(ui);
            } else {
                let sutunlar = sutunlari_hesapla(ui.max_rect());
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show_rows(ui, SATIR_Y, satirlar.len(), |ui, aralik| {
                        let okunan = self.indeks.read();
                        let kilit = match okunan.as_deref() {
                            Ok(kilit) => kilit,
                            Err(_) => return,
                        };
                        for i in aralik {
                            let Some(satir) = satirlar.get(i).copied() else {
                                continue;
                            };
                            let Some(veri) = satir_verisi(&self.sonuclar, &satir, kilit) else {
                                continue;
                            };
                            let konum = satir_konumu(&satir);
                            let (yanit, p) = ui.allocate_painter(
                                Vec2::new(ui.available_width(), SATIR_Y),
                                Sense::click(),
                            );
                            satiri_ciz(
                                ui,
                                &p,
                                yanit.rect,
                                &yanit,
                                &veri,
                                secili_konum == Some(konum),
                                &sutunlar,
                            );
                            if yanit.clicked() {
                                secili_yeni = Some(konum);
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
                        }
                    });
            }
        }

        self.satirlar = satirlar;
        self.secili = secili_yeni;
        if let Some(secilen) = eylem {
            self.durum_mesaji = Self::eylemi_uygula(secilen);
        }
    }
}

impl eframe::App for InEverythingApp {
    fn update(&mut self, ctx: &egui::Context, _cerceve: &mut eframe::Frame) {
        self.kanallari_yokla();

        let zaman = ctx.input(|girdi| girdi.time) as f32;
        let taraniyor = self.taraniyor.load(Ordering::Relaxed);
        ctx.request_repaint_after(Duration::from_millis(if taraniyor { 33 } else { 250 }));

        let taranan = self.tarama.sayi.load(Ordering::Relaxed);
        let ozet = TaramaOzeti {
            taranan,
            klasor: self.tarama.klasor.load(Ordering::Relaxed),
            dizin: self.tarama.dizin.load(Ordering::Relaxed),
        };
        let indeks_kayit = self.indeks_kayit_sayisi();

        egui::TopBottomPanel::top("ust_panel")
            .frame(
                egui::Frame::none()
                    .fill(tema::YUZEY)
                    .stroke(tema::kontur(1.0, tema::CETVEL))
                    .inner_margin(egui::Margin {
                        left: 22.0,
                        right: 22.0,
                        top: 14.0,
                        bottom: 11.0,
                    }),
            )
            .show(ctx, |ui| {
                self.baslik_satiri(ui);
                ui.add_space(4.0);
                self.arama_satiri(ui, taraniyor);
                ui.add_space(6.0);
                self.ust_durum_satiri(ui, zaman, taraniyor, &ozet, indeks_kayit);
                self.isin_cizgisi(ui, zaman, taraniyor);
            });

        egui::TopBottomPanel::bottom("durum_cubugu")
            .frame(
                egui::Frame::none()
                    .fill(tema::YUZEY)
                    .stroke(tema::kontur(1.0, tema::CETVEL))
                    .inner_margin(egui::Margin {
                        left: 22.0,
                        right: 22.0,
                        top: 8.0,
                        bottom: 8.0,
                    }),
            )
            .show(ctx, |ui| {
                self.alt_cubuk(ui, taraniyor);
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(tema::YUZEY))
            .show(ctx, |ui| {
                self.icerik(ui, zaman, taraniyor);
            });
    }
}

/// Verilen dikdörtgende sütunların x konumlarını hesaplar.
fn sutunlari_hesapla(alan: Rect) -> Sutunlar {
    let sag = alan.right() - SAG_PAY;
    let zaman = sag;
    let boyut = (sag - SUTUN_ARASI).max(alan.left() + 120.0);
    let ad_payi = (alan.width() * 0.34).clamp(180.0, 340.0);
    let yol = alan.left() + ad_payi;
    let yol_en = (boyut - 24.0 - yol).max(40.0);
    Sutunlar {
        ad: alan.left() + AD_X,
        yol,
        yol_en,
        boyut,
        zaman,
    }
}

/// Dosya adının uzantısına göre işareti ve vurgu rengini seçer.
fn dosya_rengi(ad: &str, klasor: bool) -> Color32 {
    if klasor {
        return tema::MOR;
    }
    let uzanti = ad
        .rsplit_once('.')
        .map(|(_, k)| k.to_ascii_lowercase())
        .unwrap_or_default();
    match uzanti.as_str() {
        "pdf" => tema::PEMBE,
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" | "tif" | "tiff" | "heic" => {
            tema::MOR
        }
        "rs" | "c" | "cpp" | "h" | "hpp" | "py" | "js" | "ts" | "tsx" | "go" | "java" | "sh"
        | "cmake" => tema::YESIL,
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "mkv" | "mp4" | "avi" | "mov" | "webm" => {
            tema::TURUNCU
        }
        "zip" | "tar" | "gz" | "xz" | "7z" | "rar" => tema::TURUNCU,
        "doc" | "docx" | "odt" | "txt" | "md" | "rtf" => tema::NEON,
        "xls" | "xlsx" | "ods" | "csv" => tema::YESIL,
        "json" | "yaml" | "yml" | "toml" | "xml" | "ini" | "conf" => tema::METIN_2,
        _ => tema::METIN_3,
    }
}

/// Bir sonucun satırını neon temayla çizer.
fn satiri_ciz(
    ui: &egui::Ui,
    p: &egui::Painter,
    alan: Rect,
    yanit: &egui::Response,
    veri: &SatirVerisi,
    secili_mi: bool,
    sutunlar: &Sutunlar,
) {
    // zemin
    let dolgu = if secili_mi {
        tema::NEON.gamma_multiply(0.15)
    } else if yanit.is_pointer_button_down_on() {
        tema::NEON.gamma_multiply(0.11)
    } else if yanit.hovered() {
        tema::NEON.gamma_multiply(0.06)
    } else {
        Color32::TRANSPARENT
    };
    if dolgu != Color32::TRANSPARENT {
        p.rect_filled(alan, 0.0, dolgu);
    }

    // sol vurgu çubuğu ve sağa doğru sönümlenen parıltı
    if secili_mi {
        p.rect_filled(
            Rect::from_min_max(
                Pos2::new(alan.left(), alan.top()),
                Pos2::new(alan.left() + 3.0, alan.bottom()),
            ),
            0.0,
            tema::NEON,
        );
        for i in 0..10 {
            p.rect_filled(
                Rect::from_min_max(
                    Pos2::new(alan.left() + 3.0 + i as f32 * 3.0, alan.top()),
                    Pos2::new(alan.left() + 6.0 + i as f32 * 3.0, alan.bottom()),
                ),
                0.0,
                tema::NEON.gamma_multiply(0.10 * (1.0 - i as f32 / 10.0)),
            );
        }
    } else if yanit.hovered() {
        p.rect_filled(
            Rect::from_min_max(
                Pos2::new(alan.left(), alan.top()),
                Pos2::new(alan.left() + 3.0, alan.bottom()),
            ),
            0.0,
            tema::MOR.gamma_multiply(0.8),
        );
    }

    // uzantı işareti
    let renk = dosya_rengi(&veri.ad, veri.klasor);
    let isaret = Rect::from_center_size(
        Pos2::new(alan.left() + ISARET_X + 5.0, alan.center().y),
        Vec2::new(10.0, 10.0),
    );
    p.rect_filled(
        isaret.expand(3.0),
        Rounding::same(4.0),
        renk.gamma_multiply(0.18),
    );
    p.rect_filled(isaret, Rounding::same(3.0), renk);

    // dosya adı
    let ad_en = (sutunlar.yol - 14.0 - (alan.left() + AD_X)).max(40.0);
    let ad = tema::kes(ui, &veri.ad, tema::kalin(14.0), tema::METIN, ad_en);
    p.text(
        Pos2::new(alan.left() + AD_X, alan.center().y),
        Align2::LEFT_CENTER,
        ad,
        tema::kalin(14.0),
        if secili_mi { tema::NEON } else { tema::METIN },
    );

    // yol
    let yol = tema::kes(
        ui,
        &veri.yol,
        tema::mono(11.5),
        tema::METIN_3,
        sutunlar.yol_en,
    );
    p.text(
        Pos2::new(sutunlar.yol, alan.center().y),
        Align2::LEFT_CENTER,
        yol,
        tema::mono(11.5),
        tema::METIN_3,
    );

    // sağa hizalı boyut ve zaman
    let boyut = if veri.klasor {
        String::from("—")
    } else {
        boyutu_bicimlendir(veri.boyut)
    };
    let zaman = if veri.klasor {
        String::from("klasör")
    } else {
        zamani_bicimlendir(veri.zaman)
    };
    p.text(
        Pos2::new(sutunlar.boyut, alan.center().y),
        Align2::RIGHT_CENTER,
        boyut,
        tema::mono(11.5),
        tema::METIN_2,
    );
    p.text(
        Pos2::new(sutunlar.zaman, alan.center().y),
        Align2::RIGHT_CENTER,
        zaman,
        tema::mono(11.0),
        tema::METIN_3,
    );

    // satır ayracı
    p.line_segment(
        [
            Pos2::new(alan.left(), alan.bottom() - 0.5),
            Pos2::new(alan.right(), alan.bottom() - 0.5),
        ],
        tema::kontur(1.0, tema::CETVEL.gamma_multiply(0.45)),
    );
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

    #[test]
    fn sutunlar_genisle_kaymaz() {
        let genis = sutunlari_hesapla(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 400.0)));
        assert_eq!(genis.ad, AD_X);
        assert_eq!(genis.yol, 340.0);
        assert!(genis.yol_en > 500.0);
        assert_eq!(genis.zaman, 1100.0 - SAG_PAY);
        assert_eq!(genis.boyut, genis.zaman - SUTUN_ARASI);

        let dar = sutunlari_hesapla(Rect::from_min_size(Pos2::ZERO, Vec2::new(520.0, 400.0)));
        assert!(dar.yol_en >= 40.0);
        assert!(dar.boyut - dar.yol > 100.0, "yol sütunu sığmalı");
    }

    #[test]
    fn uzanti_rengi_tanimli() {
        assert_eq!(dosya_rengi("a.pdf", false), tema::PEMBE);
        assert_eq!(dosya_rengi("a.PNG", false), tema::MOR);
        assert_eq!(dosya_rengi("main.rs", false), tema::YESIL);
        assert_eq!(dosya_rengi("klasor", true), tema::MOR);
        assert_eq!(dosya_rengi("bilinmiyor", false), tema::METIN_3);
    }
}
