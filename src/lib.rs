//! InEverything: ultra hızlı dosya arama ve yönetim uygulaması.
//!
//! Kütüphane kökü; ikili hedef (`src/main.rs`) buradaki `app::calistir`
//! işlevini çağırır. Birim testleri her modülün içinde, uçtan uca testler
//! `tests/` dizinindedir.

pub mod actions;
pub mod app;
pub mod config;
pub mod depo;
pub mod indexer;
pub mod model;
pub mod search;
pub mod tema;
