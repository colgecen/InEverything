//! Ölçüm: tarama, indeks yazma/okuma ve arama süreleri.
//!
//! ```sh
//! cargo run --release --example bench -- /home /usr
//! ```

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use ineverything::depo::Indeks;
use ineverything::indexer::{self, TaramaDurumu};

fn main() {
    let kokler: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let kokler = if kokler.is_empty() {
        vec![PathBuf::from("/")]
    } else {
        kokler
    };
    let haric: Vec<PathBuf> = indexer::VARSAYILAN_HARIC
        .iter()
        .map(PathBuf::from)
        .collect();

    let durum = TaramaDurumu::default();
    let iptal = Arc::new(AtomicBool::new(false));

    println!("kökler: {kokler:?}");

    let t = Instant::now();
    let indeks = indexer::indeks_tara(&kokler, &haric, &durum, &iptal, 1);
    let tarama = t.elapsed();
    println!(
        "tarama:        {tarama:>10.2?}  ({} kayıt, {} dosya, {} klasör, {} dizin)",
        indeks.kayit_sayisi(),
        durum.sayi.load(Ordering::Relaxed),
        durum.klasor.load(Ordering::Relaxed),
        durum.dizin.load(Ordering::Relaxed),
    );

    let yol = PathBuf::from("target/bench-indeks.bin");
    let t = Instant::now();
    indeks.yaz(&yol).expect("yaz");
    let yazma = t.elapsed();
    let dosya_boyutu = std::fs::metadata(&yol).map(|m| m.len()).unwrap_or(0);

    let t = Instant::now();
    let okunan = Indeks::yukle(&yol).expect("oku");
    let okuma = t.elapsed();
    println!(
        "yazma:         {yazma:>10.2?}  ({:.1} MB)",
        dosya_boyutu as f64 / 1e6
    );
    println!("okuma:         {okuma:>10.2?}  (açılışta bu kadar sürer)");
    println!("bellek:        {:>10} bayt", okunan.boyut());

    for sorgu in ["rapor", "*.pdf", "belgeler", "cmakelists", "zxyzbulunamaz"] {
        // İlk çağrı sayfaları ısıtır; ikinci ölçüm gerçek arama süresidir.
        let _ = okunan.ara(sorgu, 10_000);
        let t = Instant::now();
        let sonuc = okunan.ara(sorgu, 10_000);
        let sure = t.elapsed();
        println!("arama {sorgu:<24} {sure:>10.2?}  ({} sonuç)", sonuc.len());
    }
}
