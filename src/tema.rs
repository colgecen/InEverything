//! Futuristik arayüz teması: renk paleti, fontlar, gradyan ve neon çizimleri.
//!
//! Düzeni `app` modülü kurar; renk, tipografi ve parıltı efektleri buradan
//! gelir. Her çizim işlevi safdır, egui'nin `Painter` tipiyle çalışır.

use std::sync::{Arc, OnceLock};

use eframe::egui::{self, Align2, Color32, FontFamily, FontId, Pos2, Rect, Rounding, Stroke, Vec2};

/// Arka planın üstteki (hafif daha açık) tonu.
pub const ARKA_1: Color32 = Color32::from_rgb(8, 12, 22);
/// Arka planın alttaki koyu tonu.
pub const ARKA_2: Color32 = Color32::from_rgb(3, 5, 10);
/// Panel ve başlık zemini.
pub const YUZEY: Color32 = Color32::from_rgb(9, 14, 25);
/// Arama kutusunun zemini.
pub const YUZEY_2: Color32 = Color32::from_rgb(13, 20, 35);
/// Kenar çizgileri.
pub const CETVEL: Color32 = Color32::from_rgb(25, 40, 66);
/// Ana neon rengi (camgöbeği).
pub const NEON: Color32 = Color32::from_rgb(0, 229, 255);
/// İkincil neon (mor).
pub const MOR: Color32 = Color32::from_rgb(140, 92, 255);
/// Vurgu rengi (pembe).
pub const PEMBE: Color32 = Color32::from_rgb(255, 77, 165);
/// Klasör ve başarı vurgusu (yeşil).
pub const YESIL: Color32 = Color32::from_rgb(0, 255, 170);
/// Tarama ve uyarı vurgusu (turuncu).
pub const TURUNCU: Color32 = Color32::from_rgb(255, 176, 32);
/// Ana metin.
pub const METIN: Color32 = Color32::from_rgb(226, 236, 250);
/// İkincil metin.
pub const METIN_2: Color32 = Color32::from_rgb(134, 154, 184);
/// Üçüncül metin (yol, sütun başlığı, ipucu).
pub const METIN_3: Color32 = Color32::from_rgb(74, 94, 122);

/// Uygulama adının harf aralıklı yazımı (başlık ve boş ekran için).
pub const ISIM: &str = "I N E V E R Y T H I N G";

/// Kalın yazı ailesinin adı; `font_tanimlari` ile kaydedilir.
const KALIN_AILE: &str = "mono-kalin";
/// Kalın yazı verisinin `FontDefinitions` içindeki anahtarı.
const KALIN_ANAHTAR: &str = "hack-bold";

/// Normal ağırlıklı monospace font (egui varsayılanı olarak Hack gömülüdür).
pub fn mono(boyut: f32) -> FontId {
    FontId::new(boyut, FontFamily::Monospace)
}

/// Kalın monospace font (başlıklar, dosya adları, düğme etiketleri).
///
/// Sistemde kalın font bulunamazsa varsayılan monospace ailesine düşer.
pub fn kalin(boyut: f32) -> FontId {
    FontId::new(boyut, FontFamily::Name(KALIN_AILE.into()))
}

/// Fontları ve stili bağlama uygular; uygulama başlarken bir kez çağrılır.
pub fn uygula(ctx: &egui::Context) {
    ctx.set_fonts(font_tanimlari());
    ctx.set_style(stil());
}

/// Gömülü uygulama logosu (`assets/InEverything.jpg`).
///
/// İkiliye gömülür; kurulu ikili ve AppImage hangi dizinde olursa olsun
/// aynı logo açılışta bulunur. Dosyayı değiştirmek için kaynak görüntüyü
/// güncelleyip yeniden derlemek yeterlidir.
const LOGO_GORUNTU: &[u8] = include_bytes!("../assets/InEverything.jpg");

/// Logonun kenar uzunluğu; pencere ikonu için 4'ün katı olmalı.
const LOGO_OLCU: u32 = 256;

/// Logonun çözülmüş hâli (256×256 RGBA); ilk çağrıda bir kez çözülür.
static LOGO: OnceLock<Option<Arc<egui::IconData>>> = OnceLock::new();

/// Uygulama logosunun pencere/masaüstü ikonu olarak kullanılacak hâli.
///
/// Görüntü çözülemezse (`None`) uygulama varsayılan ikonla açılır.
pub fn uygulama_logosu() -> Option<&'static Arc<egui::IconData>> {
    LOGO.get_or_init(|| {
        let gorsel = image::load_from_memory(LOGO_GORUNTU).ok()?;
        let piksel = gorsel
            .resize_exact(LOGO_OLCU, LOGO_OLCU, image::imageops::FilterType::Lanczos3)
            .to_rgba8();
        Some(Arc::new(egui::IconData {
            rgba: piksel.into_raw(),
            width: LOGO_OLCU,
            height: LOGO_OLCU,
        }))
    })
    .as_ref()
}

/// Font tanımlarını kurar.
///
/// egui gömülü olarak yalnızca Hack'in normal ağırlığını taşır; kalın
/// ağırlık sistemden okunur, bulunamazsa gömülü aile yedeğe devredilir
/// (tanımsız bir aile adı egui'nin açılışta paniğe düşmesine yol açar).
fn font_tanimlari() -> egui::FontDefinitions {
    let mut tanimlar = egui::FontDefinitions::default();
    let yedek = tanimlar
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();

    let veri = font_verisi(&[
        "/usr/share/fonts/source-foundry-hack-fonts/Hack-Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "C:/Windows/Fonts/consolab.ttf",
    ]);
    let mut kalin_aile = yedek;
    if let Some(veri) = veri {
        tanimlar.font_data.insert(KALIN_ANAHTAR.to_owned(), veri);
        kalin_aile.insert(0, KALIN_ANAHTAR.to_owned());
    }
    tanimlar
        .families
        .insert(FontFamily::Name(KALIN_AILE.into()), kalin_aile);
    tanimlar
}

/// Verilen yollardan ilk okunabiliyen fontu yükler.
fn font_verisi(yollar: &[&str]) -> Option<egui::FontData> {
    yollar
        .iter()
        .find_map(|yol| std::fs::read(yol).ok())
        .map(egui::FontData::from_owned)
}

/// Uygulamanın stilini (boşluklar, kaydırma çubuğu, görünümler) üretir.
fn stil() -> egui::Style {
    let mut s = egui::Style {
        visuals: gorunum(),
        ..egui::Style::default()
    };
    s.spacing.item_spacing = Vec2::new(10.0, 6.0);
    s.spacing.button_padding = Vec2::new(14.0, 7.0);
    s.spacing.window_margin = egui::Margin {
        left: 14.0,
        right: 14.0,
        top: 12.0,
        bottom: 12.0,
    };
    s.spacing.scroll.floating = false;
    s.spacing.scroll.bar_width = 9.0;
    s.spacing.scroll.handle_min_length = 26.0;
    s.spacing.scroll.foreground_color = true;
    s.spacing.scroll.dormant_handle_opacity = 0.30;
    s.spacing.scroll.active_handle_opacity = 0.85;
    s.spacing.scroll.dormant_background_opacity = 0.0;
    s.spacing.scroll.active_background_opacity = 0.0;
    s.spacing.scroll.interact_background_opacity = 0.0;
    s.animation_time = 0.12;
    s
}

/// Koyu paletli temel görünümleri (renkler, widget katmanları) üretir.
fn gorunum() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.dark_mode = true;
    v.override_text_color = Some(METIN);
    v.panel_fill = YUZEY;
    v.window_fill = YUZEY;
    v.window_rounding = Rounding::same(16.0);
    v.window_stroke = kontur(1.0, CETVEL);
    v.window_shadow = egui::Shadow::NONE;
    v.menu_rounding = Rounding::same(12.0);
    v.popup_shadow = egui::Shadow::NONE;
    v.extreme_bg_color = ARKA_2;
    v.code_bg_color = ARKA_2;
    v.faint_bg_color = Color32::from_rgb(12, 18, 31);
    v.hyperlink_color = NEON;
    v.warn_fg_color = TURUNCU;
    v.error_fg_color = PEMBE;
    v.selection.bg_fill = NEON.gamma_multiply(0.35);
    v.selection.stroke = kontur(1.0, NEON.gamma_multiply(0.70));
    v.text_cursor.stroke = kontur(2.0, NEON);
    v.text_cursor.preview = false;
    v.button_frame = true;

    let dolu = Color32::from_rgb(11, 17, 30);
    v.widgets.noninteractive.bg_fill = dolu;
    v.widgets.noninteractive.weak_bg_fill = dolu;
    v.widgets.noninteractive.bg_stroke = kontur(1.0, CETVEL);
    v.widgets.noninteractive.rounding = Rounding::same(10.0);
    v.widgets.noninteractive.fg_stroke = kontur(1.0, METIN_2);
    v.widgets.noninteractive.expansion = 0.0;

    v.widgets.inactive.bg_fill = NEON.gamma_multiply(0.07);
    v.widgets.inactive.weak_bg_fill = YUZEY_2;
    v.widgets.inactive.bg_stroke = kontur(1.0, NEON.gamma_multiply(0.30));
    v.widgets.inactive.rounding = Rounding::same(10.0);
    v.widgets.inactive.fg_stroke = kontur(1.0, METIN);
    v.widgets.inactive.expansion = 0.0;

    v.widgets.hovered.bg_fill = NEON.gamma_multiply(0.18);
    v.widgets.hovered.weak_bg_fill = NEON.gamma_multiply(0.12);
    v.widgets.hovered.bg_stroke = kontur(1.4, NEON.gamma_multiply(0.75));
    v.widgets.hovered.rounding = Rounding::same(10.0);
    v.widgets.hovered.fg_stroke = kontur(1.5, METIN);
    v.widgets.hovered.expansion = 0.0;

    v.widgets.active.bg_fill = NEON.gamma_multiply(0.32);
    v.widgets.active.weak_bg_fill = NEON.gamma_multiply(0.24);
    v.widgets.active.bg_stroke = kontur(1.6, NEON);
    v.widgets.active.rounding = Rounding::same(10.0);
    v.widgets.active.fg_stroke = kontur(1.6, METIN);
    v.widgets.active.expansion = 0.0;

    v.widgets.open = v.widgets.hovered;
    v
}

/// İki rengi verilen oran kadar karıştırır.
fn karisim(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let (a, b) = (a.to_array(), b.to_array());
    let mut c = [0u8; 4];
    for (i, out) in c.iter_mut().enumerate() {
        let delta = f32::from(b[i]) - f32::from(a[i]);
        *out = (f32::from(a[i]) + delta * t).round() as u8;
    }
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

/// Verilen kalınlıkta düz kontur üretir.
///
/// `Stroke::new` genişliği `impl Into<f32>` olarak aldığı için doğrudan
/// sayılmaz değer yazmak gelecekte hata olacak; bu sarmalayıcı tipi sabitler.
pub fn kontur(genislik: f32, renk: Color32) -> Stroke {
    Stroke::new(genislik, renk)
}

/// Gradyan zemin, ızgara ve üst neon yıkamasını çizer.
///
/// Tarama sürerken ayrıca soldan sağa koşan bir tarama çizgisi eklenir.
pub fn arka_plan(p: &egui::Painter, alan: Rect, zaman: f32, taraniyor: bool) {
    if alan.width() <= 2.0 || alan.height() <= 2.0 {
        return;
    }

    // 1) dikey gradyan
    let adim = 8.0;
    let mut y = alan.top();
    while y < alan.bottom() {
        let t = (y - alan.top()) / alan.height();
        let alt = (y + adim).min(alan.bottom());
        p.rect_filled(
            Rect::from_min_max(Pos2::new(alan.left(), y), Pos2::new(alan.right(), alt)),
            0.0,
            karisim(ARKA_1, ARKA_2, t),
        );
        y = alt;
    }

    // 2) ince ızgara
    const IZGARA: f32 = 36.0;
    let cizgi = Color32::from_rgba_unmultiplied(64, 140, 190, 15);
    let mut x = alan.left() + alan.width() % IZGARA * 0.5;
    while x < alan.right() {
        p.line_segment(
            [Pos2::new(x, alan.top()), Pos2::new(x, alan.bottom())],
            kontur(1.0, cizgi),
        );
        x += IZGARA;
    }
    let mut y = alan.top();
    while y < alan.bottom() {
        p.line_segment(
            [Pos2::new(alan.left(), y), Pos2::new(alan.right(), y)],
            kontur(1.0, cizgi),
        );
        y += IZGARA;
    }

    // 3) üstteki yumuşak neon yıkaması
    let serit = 44.0 / 8.0;
    for i in 0..8 {
        let ust = alan.top() + i as f32 * serit;
        p.rect_filled(
            Rect::from_min_max(
                Pos2::new(alan.left(), ust),
                Pos2::new(alan.right(), ust + serit),
            ),
            0.0,
            NEON.gamma_multiply(0.045 * (1.0 - i as f32 / 8.0)),
        );
    }

    // 4) tarama çizgisi
    if taraniyor {
        let konum = alan.left() + (zaman * 0.5) % 1.0 * alan.width();
        let iz = Rect::from_min_max(
            Pos2::new((konum - 70.0).max(alan.left()), alan.top()),
            Pos2::new(konum, alan.bottom()),
        );
        p.rect_filled(iz, 0.0, NEON.gamma_multiply(0.05));
        p.line_segment(
            [
                Pos2::new(konum, alan.top()),
                Pos2::new(konum, alan.bottom()),
            ],
            kontur(1.5, NEON.gamma_multiply(0.55)),
        );
    }
}

/// Dikdörtgenin çevresine katmanlı bir neon parıltısı çizer.
///
/// `yogunluk` 0.0 ile 1.0 arasında; parıltının parlaklığını ve genişliğini ayarlar.
pub fn neon_cerceve(p: &egui::Painter, alan: Rect, yuvarlak: f32, renk: Color32, yogunluk: f32) {
    let yogunluk = yogunluk.clamp(0.0, 1.0);
    if yogunluk <= 0.01 {
        return;
    }
    let dis = alan.expand(1.0);
    for (genislik, alfa) in [(12.0, 0.05), (7.0, 0.09), (4.0, 0.15), (2.0, 0.26)] {
        p.rect_stroke(
            dis.expand(genislik * yogunluk),
            Rounding::same(yuvarlak + genislik * yogunluk),
            kontur(2.0, renk.gamma_multiply(alfa * yogunluk)),
        );
    }
    p.rect_stroke(
        dis,
        Rounding::same(yuvarlak),
        kontur(1.4, renk.gamma_multiply(0.55 + 0.45 * yogunluk)),
    );
}

/// Neon metin çizer: dört yöne düşük alfa kopya atar, üstüne asıl metni basar.
pub fn neon_yazi(
    p: &egui::Painter,
    konum: Pos2,
    hiza: Align2,
    metin: &str,
    font: FontId,
    renk: Color32,
    parlaklik: f32,
) {
    let parlaklik = parlaklik.clamp(0.0, 1.0);
    for (dx, dy) in [(-1.6, 0.0), (1.6, 0.0), (0.0, -1.6), (0.0, 1.6)] {
        p.text(
            konum + Vec2::new(dx, dy),
            hiza,
            metin,
            font.clone(),
            renk.gamma_multiply(0.16 * parlaklik),
        );
    }
    p.text(konum, hiza, metin, font, renk);
}

/// Işıklı bir nokta çizer (durum imleci, logo çekirdeği).
pub fn nokta(p: &egui::Painter, merkez: Pos2, yaricap: f32, renk: Color32) {
    for (bolen, alfa) in [
        (7.0, 0.06),
        (5.0, 0.10),
        (3.2, 0.16),
        (2.0, 0.30),
        (1.0, 1.0),
    ] {
        p.circle_filled(merkez, yaricap * bolen, renk.gamma_multiply(alfa));
    }
}

/// İki nokta arasında katmanlı parlak bir çizgi çizer.
pub fn cizgi(p: &egui::Painter, p1: Pos2, p2: Pos2, renk: Color32, yogunluk: f32) {
    let yogunluk = yogunluk.clamp(0.0, 1.0);
    for (genislik, alfa) in [(6.0, 0.07), (3.0, 0.16), (1.4, 1.0)] {
        p.line_segment(
            [p1, p2],
            kontur(genislik, renk.gamma_multiply(alfa * yogunluk)),
        );
    }
}

/// Metni verilen genişliğe sığdırır; sığmazsa karakter karakter kısaltır.
pub fn kes(ui: &egui::Ui, metin: &str, font: FontId, renk: Color32, en: f32) -> String {
    if en <= 10.0 {
        return String::new();
    }
    let olc = |aday: &str| -> f32 {
        ui.fonts(|f| f.layout_no_wrap(aday.to_owned(), font.clone(), renk))
            .size()
            .x
    };
    if olc(metin) <= en {
        return metin.to_string();
    }

    let harf: Vec<char> = metin.chars().collect();
    let mut alt = 0usize;
    let mut ust = harf.len();
    while alt < ust {
        let orta = (alt + ust).div_ceil(2);
        let aday: String = harf[..orta].iter().collect::<String>() + "…";
        if olc(&aday) <= en {
            alt = orta;
        } else {
            ust = orta - 1;
        }
    }
    if alt == 0 {
        return String::new();
    }
    let mut sonuc: String = harf[..alt].iter().collect();
    sonuc.push('…');
    sonuc
}

/// Sağa doğru dizilmiş, yuvarlak uçlu bilgi çipi çizer.
///
/// `sag` çizimden sonra çipin sol kenarına (artı boşluk) güncellenir.
#[allow(clippy::too_many_arguments)]
pub fn cip_sagdan(
    ui: &egui::Ui,
    p: &egui::Painter,
    sag: &mut f32,
    dikey: f32,
    metin: &str,
    renk: Color32,
    yukseklik: f32,
) {
    let font = mono(11.0);
    let genislik = ui
        .fonts(|f| f.layout_no_wrap(metin.to_owned(), font.clone(), renk))
        .size()
        .x;
    let en = genislik + 22.0;
    *sag -= en;
    let alan = Rect::from_min_size(
        Pos2::new(*sag, dikey - yukseklik * 0.5),
        Vec2::new(en, yukseklik),
    );
    p.rect_filled(
        alan,
        Rounding::same(yukseklik * 0.5),
        renk.gamma_multiply(0.09),
    );
    p.rect_stroke(
        alan,
        Rounding::same(yukseklik * 0.5),
        kontur(1.0, renk.gamma_multiply(0.45)),
    );
    p.text(
        alan.center(),
        Align2::CENTER_CENTER,
        metin,
        font,
        renk.gamma_multiply(0.95),
    );
    *sag -= 8.0;
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn karisim_iki_ucunu_korumaz() {
        assert_eq!(karisim(ARKA_1, ARKA_2, 0.0), ARKA_1);
        assert_eq!(karisim(ARKA_1, ARKA_2, 1.0), ARKA_2);
        let orta = karisim(Color32::BLACK, Color32::WHITE, 0.5);
        assert_eq!(orta, Color32::from_rgb(128, 128, 128));
    }

    /// Gömülü logo çözülemezse pencere/kapak ikonu boş kalır; bunu kapı
    /// testleri erken yakalar.
    #[test]
    fn gomulu_logo_cozulur() {
        let logo = uygulama_logosu().expect("gömülü logo çözülemiyor");
        assert_eq!(logo.width, LOGO_OLCU);
        assert_eq!(logo.height, LOGO_OLCU);
        assert_eq!(logo.rgba.len(), (LOGO_OLCU * LOGO_OLCU * 4) as usize);
    }

    #[test]
    fn stil_koyu_ve_neon_vurgulu() {
        let s = stil();
        assert!(s.visuals.dark_mode);
        assert_eq!(s.visuals.panel_fill, YUZEY);
        assert_eq!(s.visuals.hyperlink_color, NEON);
        assert_eq!(s.spacing.scroll.bar_width, 9.0);
        assert!(!s.spacing.scroll.floating);
    }

    #[test]
    fn kalin_aile_tanimda_kayitli() {
        let t = font_tanimlari();
        let aile = t
            .families
            .get(&FontFamily::Name(KALIN_AILE.into()))
            .expect("kalın aile kayıtlı olmalı");
        assert!(!aile.is_empty());
        // Kayıtlı her anahtar font verisinde karşılık bulmalı.
        for ad in aile {
            assert!(t.font_data.contains_key(ad), "eksik font verisi: {ad}");
        }
    }
}
