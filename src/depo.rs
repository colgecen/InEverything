//! Kalıcı dosya indeksi: tek parça ikili dosya + `mmap` ile okuma.
//!
//! Everything'in yaptığı gibi her açılışta disk taranmaz. Tarama bir kez
//! yapılır, sonuç `indeks.bin` dosyasına yazılır; sonraki açılışlarda dosya
//! belleğe **eşlenir** (`mmap`). Eşleme yalnızca birkaç mikrosaniye sürer,
//! dosya sayfa sayfa okunur ve işletim sistemi gerektiğinde sayfaları geri
//! yükleyebilir — bu yüzden açılış, diskin 600 MB'lık indeks dosyasının
//! boyutundan bağımsızdır.
//!
//! Dosya düzeni (küçük uç dönüşlü):
//!
//! ```text
//! 0                     başlık (64 bayt)
//! 64                    kayıtlar (kayit_sayisi * 40 bayt)
//! 64 + kayıt*40         metin bloğu (özgün yol + küçük harfli kopya)
//! ```
//!
//! Her kayıt metin bloğuna iki konum (özgün yol, küçük harfli kopya),
//! `off/len` çiftleri ve dosya adının yol içindeki başlangıcını işaret eder;
//! böylece arama sırasında hiçbir `String`/`PathBuf` tahsisi, UTF-8
//! doğrulaması ya da ayırıcı araması yapılmaz.

use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use memchr::memmem;
use memmap2::{Mmap, MmapOptions};

use crate::model::SearchQuery;

/// Dosya imzası (`FFIX`).
const IMAZA: &[u8; 4] = b"FFIX";
/// Okuma biçimi sürümü. Biçim değişince (ad ofseti alanı dâhil) artar.
pub const SURUM: u32 = 2;
/// Başlık boyutu (bayt).
pub const BASLIK_UZUNLUGU: usize = 64;
/// Tek kaydın boyutu (bayt).
pub const KAYIT_UZUNLUGU: usize = 40;
/// `Kayit.bayrak` içindeki "bu bir klasör" biti.
pub const BAYRAK_KLASOR: u32 = 1;
/// Zaman damgası "bilinmiyor" değeri.
const ZAMAN_YOK: i64 = -1;

const OFSET_KAYIT_SAYISI: usize = 8;
const OFSET_METIN_UZUNLUGU: usize = 16;
const OFSET_TARAMA_ZAMANI: usize = 24;
const OFSET_TARAMA_SURESI: usize = 32;
const OFSET_AYAR_HASH: usize = 40;
const OFSET_KOK_SAYISI: usize = 48;

// Kayıt içi alan ofsetleri.
const K_BOYUT: usize = 0;
const K_ZAMAN: usize = 8;
const K_YOL_OFF: usize = 16;
const K_YOL_LEN: usize = 20;
const K_KUCUK_OFF: usize = 24;
const K_KUCUK_LEN: usize = 28;
const K_BAYRAK: usize = 32;
const K_AD_OFF: usize = 36;

/// Indeks verisinin kaynağı: tarama sırasında bellekte, diskteyken `mmap`.
enum Ham {
    Bellek(Vec<u8>),
    Diskte(Mmap),
}

impl Ham {
    fn as_slice(&self) -> &[u8] {
        match self {
            Ham::Bellek(v) => v,
            Ham::Diskte(m) => m,
        }
    }
}

impl Deref for Ham {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

/// Aynı bayt dizisinde iki bağımsız kopya tutan yazıcı.
///
/// Tarama sırasında her iş parçacığı kendi yazıcısını doldurur, sonra
/// `birles` ile tek bir blok hâline getirilir. Bileşimde yalnızca metin
/// ofsetleri kaydırılır, kayıtlar bayt bayt kopyalanmaz.
pub struct KayitYazici {
    kayitlar: Vec<u8>,
    metin: Vec<u8>,
    sayi: usize,
    klasor: usize,
    dosya: usize,
}

impl Default for KayitYazici {
    fn default() -> Self {
        Self::yeni()
    }
}

impl KayitYazici {
    /// Boş yazıcı (yaklaşık 1 milyon kayıtlık tamponla başlar).
    pub fn yeni() -> Self {
        Self {
            kayitlar: Vec::with_capacity(1 << 16),
            metin: Vec::with_capacity(1 << 20),
            sayi: 0,
            klasor: 0,
            dosya: 0,
        }
    }

    /// Tek bir kayıt ekler. `yol` mutlak ve bayt olarak geçerli olmalı.
    pub fn ekle(&mut self, yol: &[u8], boyut: u64, degistirilme: Option<SystemTime>, klasor: bool) {
        let yol_off = self.metin.len() as u32;
        self.metin.extend_from_slice(yol);
        let kucuk_off = self.metin.len() as u32;
        kucuk_harf_yaz(yol, &mut self.metin);

        let bayrak = if klasor { BAYRAK_KLASOR } else { 0 };
        // Adın yol içindeki başlangıcı: ayırıcılar tek bayt olduğu için
        // her iki kopyada da aynı konumdadır.
        let ad_off = yol
            .iter()
            .rposition(|b| *b == b'/' || *b == b'\\')
            .map(|i| i + 1)
            .unwrap_or(0) as u32;
        self.kayitlar.extend_from_slice(&boyut.to_le_bytes());
        self.kayitlar
            .extend_from_slice(&zaman_kodla(degistirilme).to_le_bytes());
        self.kayitlar.extend_from_slice(&yol_off.to_le_bytes());
        self.kayitlar
            .extend_from_slice(&(yol.len() as u32).to_le_bytes());
        self.kayitlar.extend_from_slice(&kucuk_off.to_le_bytes());
        self.kayitlar
            .extend_from_slice(&(yol.len() as u32).to_le_bytes());
        self.kayitlar.extend_from_slice(&bayrak.to_le_bytes());
        self.kayitlar.extend_from_slice(&ad_off.to_le_bytes());

        self.sayi += 1;
        if klasor {
            self.klasor += 1;
        } else {
            self.dosya += 1;
        }
    }

    /// Başka bir yazıcının kayıtlarını kendisine ekler.
    pub fn birles(&mut self, diger: KayitYazici) {
        if diger.sayi == 0 {
            return;
        }
        let taban = self.metin.len() as u32;
        let mut kayitlar = diger.kayitlar;
        for kayit in kayitlar.chunks_exact_mut(KAYIT_UZUNLUGU) {
            let yol_off = u32_okut(kayit, K_YOL_OFF).saturating_add(taban);
            let kucuk_off = u32_okut(kayit, K_KUCUK_OFF).saturating_add(taban);
            yaz_u32(kayit, K_YOL_OFF, yol_off);
            yaz_u32(kayit, K_KUCUK_OFF, kucuk_off);
        }
        self.metin.extend_from_slice(&diger.metin);
        self.kayitlar.extend_from_slice(&kayitlar);
        self.sayi += diger.sayi;
        self.klasor += diger.klasor;
        self.dosya += diger.dosya;
    }

    /// Yazıcıyı tek bir bellek bloğuna dönüştürür (başlık dahil).
    pub fn indeks_uret(self, ayar_hash: u64, kok_sayisi: u32, tarama_suresi_ms: u64) -> Indeks {
        let Self {
            kayitlar,
            metin,
            sayi,
            ..
        } = self;
        let mut ham = Vec::with_capacity(BASLIK_UZUNLUGU + kayitlar.len() + metin.len());
        baslik_yaz(
            &mut ham,
            sayi as u64,
            metin.len() as u64,
            ayar_hash,
            kok_sayisi,
            tarama_suresi_ms,
        );
        ham.extend_from_slice(&kayitlar);
        ham.extend_from_slice(&metin);
        Indeks {
            ham: Ham::Bellek(ham),
            kayit_sayisi: sayi,
            metin_baslangic: BASLIK_UZUNLUGU + sayi * KAYIT_UZUNLUGU,
            tarama_zamani: sistem_saniyesi(),
            tarama_suresi_ms,
            ayar_hash,
            kok_sayisi,
        }
    }

    /// Yazılan kayıt sayısı.
    pub fn kayit_sayisi(&self) -> usize {
        self.sayi
    }
}

/// Bellek içi indeks: başlık + kayıtlar + metin bloğu tek kaynaktadır.
///
/// Kaynak ya tarama sırasında oluşturulan bellek tamponudur ya da diskteki
/// indeks dosyasının `mmap` eşlemesidir.
pub struct Indeks {
    ham: Ham,
    kayit_sayisi: usize,
    metin_baslangic: usize,
    /// Indeksin tarandığı an (unix saniye).
    pub tarama_zamani: i64,
    /// Taramayı süren (ms).
    pub tarama_suresi_ms: u64,
    /// Kök/hariç listelerinden hesaplanan imza; değişirse indeks bayat.
    pub ayar_hash: u64,
    /// Taranan kök sayısı.
    pub kok_sayisi: u32,
}

/// Arama döngüsünün kullandığı kayıt görünümü: ham baytlar, `Copy`.
///
/// [`Kayit`]ün aksine UTF-8 doğrulaması yapmaz ve `Path`/`String` üretmez;
/// bu yüzden milyonlarca kaydı tararken tahsis maliyeti yoktur.
#[derive(Clone, Copy)]
struct HamKayit<'a> {
    kucuk_yol: &'a [u8],
    kucuk_ad: &'a [u8],
    #[allow(dead_code)]
    klasor: bool,
}

/// Tek bir kaydın bellek içi görünümü; alanlar `indeks` bloğunu paylaşır.
#[derive(Debug, Clone, Copy)]
pub struct Kayit<'a> {
    /// Kaydın indeksteki sırası.
    pub konum: usize,
    /// Özgün yol (UTF-8 varsayılır).
    pub yol: &'a str,
    /// Küçük harfli yol kopyası.
    pub kucuk_yol: &'a str,
    /// Dosya/klasör adı (yolun son bölümü).
    pub ad: &'a str,
    /// Küçük harfli ad.
    pub kucuk_ad: &'a str,
    /// Bayt cinsinden boyut.
    pub boyut: u64,
    /// Son değiştirilme zamanı.
    pub degistirilme: Option<SystemTime>,
    /// `true` ise kayıt bir klasörü temsil eder.
    pub klasor_mu: bool,
}

impl Indeks {
    /// Kayıt sayısı.
    pub fn kayit_sayisi(&self) -> usize {
        self.kayit_sayisi
    }

    /// Blok boyutu (bayt).
    pub fn boyut(&self) -> usize {
        self.ham.len()
    }

    /// İndeksin bayat olup olmadığını söyler.
    ///
    /// `ayar_hash` değiştiyse (kök/haric listesi düzenlendi) ya da tarama
    /// yaşı `en_fazla_yas_saat`'i aştıysa `true` döner.
    pub fn bayat_mi(&self, ayar_hash: u64, en_fazla_yas_saat: u64) -> bool {
        if self.kayit_sayisi == 0 || self.ayar_hash != ayar_hash {
            return true;
        }
        if en_fazla_yas_saat == 0 {
            return false;
        }
        let yas = sistem_saniyesi() - self.tarama_zamani;
        yas < 0 || (yas as u64) > en_fazla_yas_saat * 3600
    }

    /// Belirtilen sıradaki kaydı döndürür; sıra geçersizse `None`.
    pub fn kayit(&self, konum: usize) -> Option<Kayit<'_>> {
        let (kayit, ham) = self.ham_kayit_araligi(konum)?;
        let yol_off = u32_okut(kayit, K_YOL_OFF) as usize;
        let yol_len = u32_okut(kayit, K_YOL_LEN) as usize;
        let kucuk_off = u32_okut(kayit, K_KUCUK_OFF) as usize;
        let kucuk_len = u32_okut(kayit, K_KUCUK_LEN) as usize;
        let ad_off = u32_okut(kayit, K_AD_OFF) as usize;
        let metin = ham.get(self.metin_baslangic..)?;

        let yol = std::str::from_utf8(metin.get(yol_off..yol_off + yol_len)?).ok()?;
        let kucuk_yol = std::str::from_utf8(metin.get(kucuk_off..kucuk_off + kucuk_len)?).ok()?;

        Some(Kayit {
            konum,
            ad: yol.get(ad_off..)?,
            kucuk_ad: kucuk_yol.get(ad_off..)?,
            yol,
            kucuk_yol,
            boyut: u64_okut(kayit, K_BOYUT),
            degistirilme: zaman_coz(u64_okut(kayit, K_ZAMAN) as i64),
            klasor_mu: u32_okut(kayit, K_BAYRAK) & BAYRAK_KLASOR != 0,
        })
    }

    /// Kaydın (bayt aralığı, tam blok) çiftini verir.
    fn ham_kayit_araligi(&self, konum: usize) -> Option<(&[u8], &[u8])> {
        if konum >= self.kayit_sayisi {
            return None;
        }
        let taban = BASLIK_UZUNLUGU + konum * KAYIT_UZUNLUGU;
        let kayit = self.ham.get(taban..taban + KAYIT_UZUNLUGU)?;
        Some((kayit, self.ham.as_slice()))
    }

    /// Arama döngüsünün kullandığı tahsissiz, UTF-8'siz kayıt görünümü.
    fn ham_kayit(&self, konum: usize) -> Option<HamKayit<'_>> {
        let (kayit, ham) = self.ham_kayit_araligi(konum)?;
        let kucuk_off = u32_okut(kayit, K_KUCUK_OFF) as usize;
        let kucuk_len = u32_okut(kayit, K_KUCUK_LEN) as usize;
        let ad_off = u32_okut(kayit, K_AD_OFF) as usize;
        let metin = ham.get(self.metin_baslangic..)?;
        let kucuk_yol = metin.get(kucuk_off..kucuk_off + kucuk_len)?;
        Some(HamKayit {
            kucuk_yol,
            kucuk_ad: kucuk_yol.get(ad_off..)?,
            klasor: u32_okut(kayit, K_BAYRAK) & BAYRAK_KLASOR != 0,
        })
    }

    /// Kaydın yolunu `PathBuf` olarak verir (eylemler için).
    pub fn yol(&self, konum: usize) -> Option<PathBuf> {
        self.kayit(konum).map(|k| PathBuf::from(k.yol))
    }

    /// Sorguyu indekste arar, en iyi `en_fazla` kaydın konumlarını döndürür.
    ///
    /// Eşleşme küçük harfli *ad* üzerinde puanlanır; ad eşleşmezse yolun
    /// kendisi üzerinde aranır (Everything davranışı). Sonuçlar puana, sonra
    /// ada göre sıralanır. Tahsis yalnızca sonuç vektöründe yapılır.
    pub fn ara(&self, ham_sorgu: &str, en_fazla: usize) -> Vec<usize> {
        if self.kayit_sayisi == 0 || en_fazla == 0 {
            return Vec::new();
        }
        let sorgu = SearchQuery::cozumle(ham_sorgu);
        if sorgu.bos_mu() {
            return Vec::new();
        }
        let igne = igneyi_ayikla(&sorgu);
        let igne = igne.trim();
        let filtre = sorgu.uzanti_filtresi.as_deref().map(str::as_bytes);
        if igne.is_empty() && filtre.is_none() {
            return Vec::new();
        }

        if igne.is_empty() {
            // Yalnızca uzantı filtresi: eşleşen ilk kayıtlar yeterlidir.
            let adaylar = self.eslesenleri_topla(|kayit| {
                if !filtre.map_or(true, |f| uzanti_eslesir(kayit.kucuk_ad, f)) {
                    return None;
                }
                Some(0u8)
            });
            return sirala_ve_kirp(self, adaylar, en_fazla);
        }

        // Sorgu iğnesini küçük harfe indir; dosya adları da küçük harfle
        // saklandığı için doğrudan bayt araması yapılabilir.
        let igne = kucuk_harf(igne);
        let igne = igne.as_bytes();
        let arayici = memmem::Finder::new(igne);
        let adaylar = self.eslesenleri_topla(|kayit| {
            if !filtre.map_or(true, |f| uzanti_eslesir(kayit.kucuk_ad, f)) {
                return None;
            }
            if kayit.kucuk_ad == igne {
                return Some(0u8);
            }
            if kayit.kucuk_ad.starts_with(igne) {
                return Some(1u8);
            }
            if arayici.find(kayit.kucuk_ad).is_some() {
                return Some(2u8);
            }
            if arayici.find(kayit.kucuk_yol).is_some() {
                return Some(3u8);
            }
            None
        });
        sirala_ve_kirp(self, adaylar, en_fazla)
    }

    /// Kayıtları paralel tarayıp closure'ın `Some` döndürdüğü her kaydı
    /// (skor, konum) çifti olarak toplar.
    ///
    /// Döngü kayıtları bayt bayt gezer: tek bir UTF-8 doğrulaması, tek bir
    /// ayırıcı araması ve tek bir `String` tahsisi yoktur. Sonuç parçacıklar
    /// arasında sıralı değildir; sıralama [`Indeks::ara`] içinde en sonda
    /// yapılır.
    fn eslesenleri_topla<F>(&self, kriter: F) -> Vec<(u8, u32)>
    where
        F: Fn(HamKayit<'_>) -> Option<u8> + Send + Sync,
    {
        use rayon::prelude::*;
        let is_parcacik = (1 + rayon::current_num_threads() * 4).max(1);
        let adim = self.kayit_sayisi.div_ceil(is_parcacik);
        (0..self.kayit_sayisi)
            .into_par_iter()
            .step_by(adim)
            .map(|baslangic| {
                let mut liste = Vec::new();
                for konum in baslangic..(baslangic + adim).min(self.kayit_sayisi) {
                    if let Some(kayit) = self.ham_kayit(konum) {
                        if let Some(skor) = kriter(kayit) {
                            liste.push((skor, konum as u32));
                        }
                    }
                }
                liste
            })
            .reduce_with(|mut a, mut b| {
                a.append(&mut b);
                a
            })
            .unwrap_or_default()
    }

    /// İndeksi verilen yola yazar (atomik: geçici dosya + rename).
    pub fn yaz(&self, yol: &Path) -> Result<()> {
        use std::io::Write;
        if let Some(ust) = yol.parent() {
            std::fs::create_dir_all(ust)
                .with_context(|| format!("klasör açılamadı: {}", ust.display()))?;
        }
        let gecici = yol.with_extension("bin.tmp");
        {
            let dosya = std::fs::File::create(&gecici)
                .with_context(|| format!("indeks yazılamadı: {}", gecici.display()))?;
            let mut tampon = std::io::BufWriter::new(dosya);
            tampon.write_all(&self.ham)?;
            tampon.flush()?;
        }
        std::fs::rename(&gecici, yol)
            .with_context(|| format!("indeks taşınamadı: {}", yol.display()))?;
        Ok(())
    }

    /// Verilen yoldan indeks dosyasını `mmap` ile eşler ve doğrular.
    ///
    /// Eşleme anında biter; dosya gereksinim duyuldukça sayfa sayfa
    /// okunur. Bozuk ya da eksik dosya hata döndürür, çağıran taraf
    /// yeniden taramalıdır.
    pub fn yukle(yol: &Path) -> Result<Self> {
        let dosya = std::fs::File::open(yol)
            .with_context(|| format!("indeks açılamadı: {}", yol.display()))?;
        let uzunluk = dosya
            .metadata()
            .with_context(|| format!("indeks bilgisi alınamadı: {}", yol.display()))?
            .len();
        if uzunluk < BASLIK_UZUNLUGU as u64 {
            bail!("indeks boş ya da çok küçük");
        }
        // Güvenlik: eşleme dosyaya yazmaz. Indeks yazılırken dosya önce
        // `.tmp` adıyla oluşturulup `rename` edilir; bu, girdi yeniden
        // yazılsa bile eski eşlemenin geçerliliğini korur (POSIX'te eski
        // inode, Windows'ta eski dosya tanımı açık kalır).
        let mmap = unsafe { MmapOptions::new().map(&dosya) }
            .with_context(|| format!("indeks eşlenemedi: {}", yol.display()))?;
        Self::dogrula(Ham::Diskte(mmap)).with_context(|| format!("indeks bozuk: {}", yol.display()))
    }

    /// Ham kaynağı doğrulayıp `Indeks` yapar.
    fn dogrula(ham: Ham) -> Result<Self> {
        if ham.len() < BASLIK_UZUNLUGU {
            bail!("indeks başlığı eksik");
        }
        if &ham[0..4] != IMAZA {
            bail!("indeks imzası hatalı");
        }
        let surum = u32_okut(&ham, 4);
        if surum != SURUM {
            bail!("indeks sürümü desteklenmiyor: {surum}");
        }
        let kayit_sayisi = u64_okut(&ham, OFSET_KAYIT_SAYISI);
        let metin_uzunlugu = u64_okut(&ham, OFSET_METIN_UZUNLUGU);
        let beklenen =
            BASLIK_UZUNLUGU as u64 + kayit_sayisi * KAYIT_UZUNLUGU as u64 + metin_uzunlugu;
        if beklenen != ham.len() as u64 {
            bail!(
                "indeks boyutu tutarsız (beklenen {beklenen}, bulunan {})",
                ham.len()
            );
        }
        let kayit_sayisi = kayit_sayisi as usize;
        let indeks = Self {
            tarama_zamani: i64_okut(&ham, OFSET_TARAMA_ZAMANI),
            tarama_suresi_ms: u64_okut(&ham, OFSET_TARAMA_SURESI),
            ayar_hash: u64_okut(&ham, OFSET_AYAR_HASH),
            kok_sayisi: u32_okut(&ham, OFSET_KOK_SAYISI),
            metin_baslangic: BASLIK_UZUNLUGU + kayit_sayisi * KAYIT_UZUNLUGU,
            kayit_sayisi,
            ham,
        };
        indeks.ornekleri_dogrula()?;
        Ok(indeks)
    }

    /// Bazı kayıtların ofsetlerinin metin bloğuna düştüğünü doğrular.
    ///
    /// `kayit()` ofsetleri zaten sınırlar içinde arar; burada yalnızca
    /// başlık/kayıt sayısı hesabının kaydırmadığı doğrulanır.
    fn ornekleri_dogrula(&self) -> Result<()> {
        if self.kayit_sayisi == 0 {
            return Ok(());
        }
        for konum in [0, self.kayit_sayisi / 2, self.kayit_sayisi - 1] {
            if self.kayit(konum).is_none() {
                bail!("{konum}. kayıt okunamadı");
            }
        }
        Ok(())
    }
}

/// Sorgudaki aranacak iğne metni (`*.pdf rapor` -> `rapor`).
fn igneyi_ayikla(sorgu: &SearchQuery) -> String {
    if sorgu.uzanti_filtresi.is_none() {
        return sorgu.kucuk.clone();
    }
    let mut parcalar = sorgu.kucuk.split_whitespace();
    let _ = parcalar.next();
    parcalar.collect::<Vec<_>>().join(" ")
}

/// Skorlanmış eşleşmeleri sıralayıp ilk `en_fazla` konumu döndürür.
fn sirala_ve_kirp(indeks: &Indeks, mut adaylar: Vec<(u8, u32)>, en_fazla: usize) -> Vec<usize> {
    if adaylar.len() <= en_fazla {
        adaylar.sort_unstable();
        return adaylar
            .into_iter()
            .map(|(_, konum)| konum as usize)
            .collect();
    }
    let sira = |a: &(u8, u32), b: &(u8, u32)| {
        a.0.cmp(&b.0)
            .then_with(|| ad_karsilastir(indeks, a.1 as usize, b.1 as usize))
            .then_with(|| a.1.cmp(&b.1))
    };
    // Yalnızca ilk `en_fazla` kayda ihtiyaç var: kısmi sıralama ile
    // yüz binlerce eşleşmeyi gereksiz yere sıralamayı önlüyoruz.
    if adaylar.len() > 1 {
        adaylar.select_nth_unstable_by(en_fazla - 1, sira);
    }
    adaylar.truncate(en_fazla);
    adaylar.sort_unstable_by(sira);
    adaylar
        .into_iter()
        .map(|(_, konum)| konum as usize)
        .collect()
}

/// Aynı skordaki iki kaydı ada göre sıralar (tahsissiz, bayt bazlı).
fn ad_karsilastir(indeks: &Indeks, a: usize, b: usize) -> std::cmp::Ordering {
    match (indeks.ham_kayit(a), indeks.ham_kayit(b)) {
        (Some(x), Some(y)) => x.kucuk_ad.cmp(y.kucuk_ad),
        _ => std::cmp::Ordering::Equal,
    }
}

/// `*.pdf` filtresi için küçük harfli ada göre uzantı eşleşmesi.
fn uzanti_eslesir(kucuk_ad: &[u8], uzanti: &[u8]) -> bool {
    kucuk_ad
        .iter()
        .rposition(|b| *b == b'.')
        .is_some_and(|i| kucuk_ad.get(i + 1..) == Some(uzanti))
}

/// Yola ASCII küçük harf uygular; diğer baytlar olduğu gibi kopyalanır.
fn kucuk_harf_yaz(yol: &[u8], hedef: &mut Vec<u8>) {
    for &b in yol {
        hedef.push(b.to_ascii_lowercase());
    }
}

/// Metni küçük harfe indirir (ASCII dışı için `char::to_lowercase`).
fn kucuk_harf(metin: &str) -> String {
    if metin.is_ascii() {
        return metin.to_ascii_lowercase();
    }
    metin.to_lowercase()
}

/// Unix saniye cinsinden şimdi.
fn sistem_saniyesi() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// `SystemTime` -> unix saniye (`None` ise [ZAMAN_YOK]).
fn zaman_kodla(zaman: Option<SystemTime>) -> i64 {
    zaman
        .and_then(|z| z.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(ZAMAN_YOK)
}

/// unix saniye -> `SystemTime`.
fn zaman_coz(kod: i64) -> Option<SystemTime> {
    if kod < 0 {
        return None;
    }
    UNIX_EPOCH.checked_add(Duration::from_secs(kod as u64))
}

fn baslik_yaz(
    hedef: &mut Vec<u8>,
    kayit_sayisi: u64,
    metin_uzunlugu: u64,
    ayar_hash: u64,
    kok_sayisi: u32,
    tarama_suresi_ms: u64,
) {
    hedef.extend_from_slice(IMAZA);
    hedef.extend_from_slice(&SURUM.to_le_bytes());
    hedef.extend_from_slice(&kayit_sayisi.to_le_bytes());
    hedef.extend_from_slice(&metin_uzunlugu.to_le_bytes());
    hedef.extend_from_slice(&sistem_saniyesi().to_le_bytes());
    hedef.extend_from_slice(&tarama_suresi_ms.to_le_bytes());
    hedef.extend_from_slice(&ayar_hash.to_le_bytes());
    hedef.extend_from_slice(&kok_sayisi.to_le_bytes());
    hedef.resize(BASLIK_UZUNLUGU, 0);
}

fn u32_okut(ham: &[u8], ofset: usize) -> u32 {
    ham.get(ofset..ofset + 4)
        .map(|k| u32::from_le_bytes([k[0], k[1], k[2], k[3]]))
        .unwrap_or(0)
}

fn u64_okut(ham: &[u8], ofset: usize) -> u64 {
    ham.get(ofset..ofset + 8)
        .map(|k| u64::from_le_bytes([k[0], k[1], k[2], k[3], k[4], k[5], k[6], k[7]]))
        .unwrap_or(0)
}

fn i64_okut(ham: &[u8], ofset: usize) -> i64 {
    u64_okut(ham, ofset) as i64
}

fn yaz_u32(ham: &mut [u8], ofset: usize, deger: u32) {
    if let Some(k) = ham.get_mut(ofset..ofset + 4) {
        k.copy_from_slice(&deger.to_le_bytes());
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn ornek_yazici() -> KayitYazici {
        let mut yazici = KayitYazici::yeni();
        yazici.ekle(b"/home/kullanici/RAPOR.PDF", 100, None, false);
        yazici.ekle(b"/home/kullanici/foto.png", 200, None, false);
        yazici.ekle(b"/home/kullanici/arsiv", 0, None, true);
        yazici
    }

    #[test]
    fn yaz_oku_gidis_donusu_kayipsizdir() {
        let dizin = tempfile::tempdir().expect("gecici dizin");
        let yol = dizin.path().join("indeks.bin");
        let indeks = ornek_yazici().indeks_uret(42, 2, 1234);
        indeks.yaz(&yol).expect("yaz");
        let okunan = Indeks::yukle(&yol).expect("oku");

        assert_eq!(okunan.kayit_sayisi(), 3);
        assert_eq!(okunan.ayar_hash, 42);
        assert_eq!(okunan.kok_sayisi, 2);
        assert_eq!(okunan.tarama_suresi_ms, 1234);
        assert_eq!(okunan.ham.as_slice(), indeks.ham.as_slice());

        let ilk = okunan.kayit(0).expect("ilk kayit");
        assert_eq!(ilk.yol, "/home/kullanici/RAPOR.PDF");
        assert_eq!(ilk.kucuk_yol, "/home/kullanici/rapor.pdf");
        assert_eq!(ilk.ad, "RAPOR.PDF");
        assert_eq!(ilk.kucuk_ad, "rapor.pdf");
        assert_eq!(ilk.boyut, 100);
        assert!(!ilk.klasor_mu);
        assert!(ilk.degistirilme.is_none());

        let klasor = okunan.kayit(2).expect("klasor kaydi");
        assert!(klasor.klasor_mu);
        assert_eq!(klasor.ad, "arsiv");
    }

    #[test]
    fn birlestirme_ofsetleri_kaydirmaz() {
        let mut bir = KayitYazici::yeni();
        bir.ekle(b"/a/bir.pdf", 1, None, false);
        let mut iki = KayitYazici::yeni();
        iki.ekle(b"/b/iki.pdf", 2, None, false);
        iki.ekle(b"/b/uc.pdf", 3, None, false);
        bir.birles(iki);
        let indeks = bir.indeks_uret(1, 1, 0);

        assert_eq!(indeks.kayit_sayisi(), 3);
        assert_eq!(indeks.kayit(1).expect("bir").yol, "/b/iki.pdf");
        assert_eq!(indeks.kayit(1).expect("bir").kucuk_yol, "/b/iki.pdf");
        assert_eq!(indeks.kayit(2).expect("iki").yol, "/b/uc.pdf");
        assert_eq!(indeks.kayit(2).expect("iki").kucuk_ad, "uc.pdf");
    }

    #[test]
    fn arama_kucuk_harf_duyarsizdir() {
        let indeks = ornek_yazici().indeks_uret(1, 1, 0);
        let sonuc = indeks.ara("rapor", 10);
        assert_eq!(sonuc.len(), 1);
        assert_eq!(indeks.kayit(sonuc[0]).expect("kayit").ad, "RAPOR.PDF");
    }

    #[test]
    fn arama_yol_parcasini_da_bulur() {
        let indeks = ornek_yazici().indeks_uret(1, 1, 0);
        let sonuc = indeks.ara("arsiv", 10);
        assert!(!sonuc.is_empty());
    }

    #[test]
    fn arama_uzanti_filtresi_daraltir() {
        let indeks = ornek_yazici().indeks_uret(1, 1, 0);
        let pdf = indeks.ara("*.pdf", 10);
        assert_eq!(pdf.len(), 1);
        let birlik = indeks.ara("*.pdf rapor", 10);
        assert_eq!(birlik, pdf);
        let bos = indeks.ara("*.pdf foto", 10);
        assert!(bos.is_empty());
    }

    #[test]
    fn arama_sonu_limiti_uygulanir() {
        let mut yazici = KayitYazici::yeni();
        for i in 0..100 {
            yazici.ekle(format!("/x/dosya{i}.txt").as_bytes(), 1, None, false);
        }
        let indeks = yazici.indeks_uret(1, 1, 0);
        assert_eq!(indeks.ara("dosya", 10).len(), 10);
        assert_eq!(indeks.ara("dosya", 1000).len(), 100);
    }

    #[test]
    fn siralama_once_tam_ad_olusu() {
        let mut yazici = KayitYazici::yeni();
        yazici.ekle(b"/x/rapor.pdf", 1, None, false);
        yazici.ekle(b"/x/rapor", 1, None, false);
        yazici.ekle(b"/x/raporu.txt", 1, None, false);
        let indeks = yazici.indeks_uret(1, 1, 0);
        let sonuc = indeks.ara("rapor", 10);
        assert_eq!(sonuc[0], 1, "tam ad eşleşmesi önce gelmeli");
    }

    #[test]
    fn bozuk_indeks_reddedilir() {
        let dizin = tempfile::tempdir().expect("gecici dizin");
        let yol = dizin.path().join("indeks.bin");

        std::fs::write(&yol, b"kisa").expect("yaz");
        assert!(Indeks::yukle(&yol).is_err());

        let indeks = ornek_yazici().indeks_uret(1, 1, 0);
        indeks.yaz(&yol).expect("yaz");
        let mut veri = std::fs::read(&yol).expect("oku");
        veri[0] = b'X';
        std::fs::write(&yol, &veri).expect("yaz");
        assert!(Indeks::yukle(&yol).is_err());

        let indeks = ornek_yazici().indeks_uret(1, 1, 0);
        indeks.yaz(&yol).expect("yaz");
        let mut veri = std::fs::read(&yol).expect("oku");
        veri[4] = 99;
        std::fs::write(&yol, &veri).expect("yaz");
        assert!(Indeks::yukle(&yol).is_err());
    }

    #[test]
    fn bayatlik_kurali_calisir() {
        let indeks = ornek_yazici().indeks_uret(7, 1, 0);
        assert!(!indeks.bayat_mi(7, 24), "aynı ayar hash'i taze sayılır");
        assert!(
            !indeks.bayat_mi(7, 0),
            "yaş kuralı kapalıyken bayat sayılmaz"
        );
        assert!(indeks.bayat_mi(8, 0), "ayar hash'i değiştiyse bayat");
    }

    #[test]
    fn ad_konumu_ayiriciya_gore_bulunur() {
        let mut yazici = KayitYazici::yeni();
        yazici.ekle(b"/a/b/c.pdf", 1, None, false);
        yazici.ekle(b"C:\\a\\b.pdf", 1, None, false);
        yazici.ekle(b"dosya.pdf", 1, None, false);
        yazici.ekle(b"kok", 0, None, true);
        let indeks = yazici.indeks_uret(1, 1, 0);

        let adlar: Vec<&str> = (0..4)
            .filter_map(|i| indeks.kayit(i))
            .map(|k| k.ad)
            .collect();
        assert_eq!(adlar, vec!["c.pdf", "b.pdf", "dosya.pdf", "kok"]);

        // Küçük harfli kopya da aynı konumdan başlamalı.
        let kucuk: Vec<&str> = (0..4)
            .filter_map(|i| indeks.kayit(i))
            .map(|k| k.kucuk_ad)
            .collect();
        assert_eq!(kucuk, vec!["c.pdf", "b.pdf", "dosya.pdf", "kok"]);
    }

    #[test]
    fn zaman_kodlama_kaybi_zaman_yok_yapar() {
        assert_eq!(zaman_kodla(None), ZAMAN_YOK);
        let an = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        assert_eq!(zaman_kodla(Some(an)), 1_700_000_000);
        assert_eq!(zaman_coz(ZAMAN_YOK), None);
        assert_eq!(zaman_coz(1_700_000_000), Some(an));
    }
}
