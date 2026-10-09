//! Hasher — calcolo e verifica di hash per file.
//! Martini Multimedia s.a.s.
//!
//! Crate library: `backend` contiene motore, metadati, impostazioni ed esportazione;
//! `ui` contiene l'interfaccia egui. Il binario (`main.rs`) è un involucro sottile.

pub mod backend;
pub mod ui;

pub const APP_NAME: &str = "Hasher";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const COMPANY: &str = "Martini Multimedia s.a.s.";
pub const COMPANY_URL: &str = "https://www.martini-multimedia.net";

rust_i18n::i18n!("locales", fallback = "en");
