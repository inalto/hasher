# Hasher — Piano di sviluppo

Applicazione **portable** per calcolare e verificare gli hash di uno o più file.
Sviluppata da **Martini Multimedia s.a.s.** Multipiattaforma: Linux, Windows, macOS.

Data piano: 9 ottobre 2026.

---

## 1. Decisione architetturale: Rust puro con egui/eframe

La richiesta iniziale era Rust + Tauri. Durante la ricognizione dell'ambiente è emerso un vincolo
bloccante:

| Requisito Tauri 2 | Disponibile su AlmaLinux 9 |
|---|---|
| webkit2gtk-4.1 (libsoup3) | **No** — solo webkit2gtk-4.0 (libsoup2) |
| libsoup3-devel | **No** — solo libsoup 2.72 |

Tauri 2 non è quindi compilabile nativamente sulla macchina di sviluppo. Le alternative erano
Tauri 1 (in manutenzione, sconsigliato per un nuovo progetto), compilare dentro un container
Ubuntu (loop di sviluppo lento) oppure un'interfaccia nativa interamente in Rust. Il committente
ha autorizzato l'opzione Rust puro, che porta anche vantaggi concreti per un'app *portable*:

- **Un solo eseguibile statico**, nessun runtime WebView da pretendere sul sistema di destinazione.
- Nessuna toolchain Node/npm; una sola lingua, un solo `cargo build`.
- Cross-compilazione più semplice (il frontend non è una pagina web da impacchettare).

**Framework scelto: `egui` + `eframe` 0.36.** Motivazioni rispetto a iced e Slint:

- API stabile e molto documentata; supporto nativo a drag & drop di file dal sistema operativo
  (`hovered_files` / `dropped_files`), tema chiaro/scuro che segue automaticamente il sistema,
  spinner e barre di avanzamento animate incluse, animazioni fluide con `animate_value_with_time`.
- Rendering con OpenGL (`glow`): leggero da compilare, funziona anche con Mesa software (Xvfb)
  per i test grafici automatizzati su macchina senza display.
- Font e icone personalizzati embeddati nel binario (Inter, JetBrains Mono, Phosphor Icons).
- Slint non espone ancora in modo affidabile il drop di file dal sistema; iced ha API instabili
  tra versioni e animazioni più laboriose.

---

## 2. Funzionalità

### 2.1 Calcolo hash
Algoritmi (tutti in **un'unica lettura del file**, calcolati in parallelo):

| Gruppo | Algoritmi |
|---|---|
| Checksum | CRC32, CRC64 (XZ/ECMA) , xxHash64, xxHash3-128 |
| MD | MD2, MD4, MD5 |
| SHA | SHA-1, SHA-224, SHA-256, SHA-384, SHA-512, SHA3-256, SHA3-512 |
| RIPEMD | RIPEMD-128, RIPEMD-160, RIPEMD-256, RIPEMD-320 |
| BLAKE | BLAKE2b-512, BLAKE2s-256, BLAKE3 |
| P2P | ED2K (convenzione eMule: blocchi da 9.728.000 byte) |

Quelli della vecchia applicazione di riferimento (CRC32, MD4, MD5, SHA1, SHA256, SHA512,
RMD128, RMD160, ED2K) sono **attivi di default**; gli altri sono attivabili nelle impostazioni.
**MD2 è disponibile ma non attivo di default**: è un algoritmo intrinsecamente seriale che le misure
sul motore limitano a circa 7–10 MB/s, contro i 150 MB/s di SHA-256 e gli 800+ di BLAKE3; tenerlo
attivo rallenterebbe l'intero calcolo a quella velocità. Nell'interfaccia è contrassegnato come "lento".

### 2.2 File singolo
Scheda con: nome, percorso completo, dimensione (leggibile + byte esatti), data di creazione,
data di modifica, attributi (sola lettura, nascosto, sistema, archivio, link simbolico;
permessi POSIX su Linux/macOS), poi una riga per ogni hash con pulsante *copia* e indicatore di
verifica.

### 2.3 Più file / cartelle
Tabella con nome, dimensione, data di modifica, stato (in coda / in corso con barra di
avanzamento / completato / errore) e hash dell'algoritmo selezionato nella colonna; ogni riga è
espandibile per vedere tutti gli hash. Barra di stato globale con avanzamento complessivo,
velocità (MB/s) e tempo stimato. Pulsante *Annulla*. Le cartelle trascinate vengono esplorate
(ricorsione configurabile).

### 2.4 Verifica
- **Incolla un hash atteso**: l'app normalizza il testo (spazi, due punti, maiuscole), riconosce
  l'algoritmo dalla lunghezza e mostra ✓ *Corrisponde (SHA-256)* oppure ✗ *Nessuna
  corrispondenza*. In lista evidenzia il file che corrisponde.
- **File di checksum** (`.md5`, `.sha1`, `.sha256`, `.sha512`, `.sfv`, formato GNU coreutils
  `hash  nome` o BSD `ALGO (nome) = hash`): trascinandolo l'app verifica tutti i file elencati
  e mostra OK / FALLITO / MANCANTE per ciascuno.
- **Confronto tra due file**: selezionando due righe (o con Shift/Ctrl al drop, come nella
  vecchia app) l'app evidenzia se sono identici per ogni algoritmo.

### 2.5 Esportazione
Formati: **TXT** (rapporto leggibile), **CSV**, **JSON**, **XLSX** (Excel, con intestazioni
formattate e larghezze colonna), **DOCX** (Word, tabella per file), **file checksum**
(`.sha256`, `.md5`, … in formato GNU, riutilizzabile per la verifica). Dialogo di salvataggio
nativo. XLSX e DOCX sostituiscono i formati legacy XLS/DOC (apribili da Excel/Word e LibreOffice).

### 2.6 Impostazioni
Lingua (automatica / italiano / inglese / francese / tedesco / spagnolo), tema (automatico /
chiaro / scuro), algoritmi attivi di default, esadecimale maiuscolo o minuscolo, ricorsione
nelle cartelle, numero di file elaborati in parallelo.
**Persistenza portable**: `hasher.settings.json` accanto all'eseguibile se la cartella è
scrivibile, altrimenti nella cartella di configurazione utente.

### 2.7 Aiuto
Menu *Aiuto* con: *Guida* (uso, drag & drop, verifica, confronto, esportazione), *Scorciatoie da
tastiera*, *Informazioni* (logo Martini Multimedia, versione, licenze dei componenti).

### 2.8 Esperienza d'uso
- Drag & drop come interazione principale; la zona di rilascio pulsa quando un file è sopra la
  finestra.
- Animazioni d'attesa: spinner sulle righe in calcolo, barre con effetto *shimmer*, comparsa
  progressiva dei risultati, contatori animati.
- Dark mode automatica dal sistema, con override manuale.
- Toast di conferma ("Copiato", "Esportato in …").
- Scorciatoie: `Ctrl+O` apri file, `Ctrl+Shift+O` apri cartella, `Ctrl+E` esporta, `Ctrl+V`
  incolla hash da verificare, `Ctrl+,` impostazioni, `F1` guida, `Esc` annulla/chiudi.

---

## 3. Identità visiva

Colori estratti da `logo.svg`:

| Ruolo | Colore | Uso |
|---|---|---|
| Navy | `#151F35` | testo primario (chiaro), sfondo (scuro) |
| Blu primario | `#144781` | pulsanti, barre di avanzamento, accenti |
| Azzurro acciaio | `#5B85A1` | accenti secondari, bordi attivi |

Palette derivata:

| Token | Tema chiaro | Tema scuro |
|---|---|---|
| sfondo | `#F4F7FB` | `#0E1526` |
| superficie / card | `#FFFFFF` | `#172238` |
| superficie elevata | `#EAF0F7` | `#1F2C47` |
| testo | `#151F35` | `#E6ECF4` |
| testo attenuato | `#5B6B82` | `#9AAAC2` |
| primario | `#144781` | `#4F86C6` |
| accento | `#5B85A1` | `#7FA6C0` |
| successo | `#1F8A5B` | `#4CC38A` |
| errore | `#C8433F` | `#E5635F` |
| avviso | `#B9780A` | `#F2B035` |

Tipografia: **Inter** (interfaccia), **JetBrains Mono** (hash). Icone: Phosphor.
Icona applicazione: il marchio "M" del logo su tessera arrotondata chiara (`assets/icons/`),
generata in PNG 16–1024, ICO (Windows) e ICNS (macOS, in CI).

---

## 4. Architettura del codice

```
hasher/
├── Cargo.toml                 dipendenze, profilo release (LTO, strip)
├── build.rs                   icona e metadati .exe su Windows (winresource)
├── assets/
│   ├── brand/  logo.svg, mark.svg
│   ├── icons/  icon.svg, icon-*.png, icon.ico
│   └── fonts/  Inter-*.ttf, JetBrainsMono-*.ttf + licenze
├── locales/   it.yml en.yml fr.yml de.yml es.yml   (rust-i18n, compilati nel binario)
├── src/
│   ├── main.rs                avvio eframe, icona finestra, flag --screenshot per i test
│   ├── backend/
│   │   ├── mod.rs
│   │   ├── algo.rs            enum Algo, nomi, lunghezze digest, costruzione hasher
│   │   ├── hasher.rs          trait StreamHasher + implementazioni (incl. ED2K)
│   │   ├── engine.rs          motore: thread pool, lettura unica, progresso, annullamento
│   │   ├── checksum_file.rs   parser file .md5/.sha256/.sfv (GNU + BSD)
│   │   ├── fileinfo.rs        metadati file (dimensione, date, attributi per OS)
│   │   ├── settings.rs        struct Settings + persistenza portable
│   │   └── export/            txt_fmt.rs csv_fmt.rs json_fmt.rs xlsx_fmt.rs docx_fmt.rs checksum_fmt.rs
│   └── ui/
│       ├── mod.rs             HasherApp (eframe::App), stato, gestione eventi
│       ├── theme.rs           palette, Visuals chiaro/scuro, font, spaziature
│       ├── anim.rs            helper animazioni (shimmer, pulse, fade, contatori)
│       ├── widgets.rs         riga hash, badge, pulsante copia, toast
│       ├── topbar.rs          logo, menu File/Strumenti/Aiuto, tema, impostazioni
│       ├── dropzone.rs        stato vuoto e overlay di trascinamento
│       ├── single.rs          vista file singolo
│       ├── list.rs            tabella multi-file con righe espandibili
│       ├── verify.rs          pannello verifica / confronto
│       ├── settings.rs        finestra impostazioni
│       ├── help.rs            finestra guida / scorciatoie / informazioni
│       └── statusbar.rs       avanzamento globale, velocità, credito
├── tests/                     test d'integrazione (vettori noti, export)
├── scripts/                   bundle macOS (.app), pacchetti Linux/Windows, screenshot Xvfb, dipendenze dev
└── .github/workflows/         ci.yml (fmt/clippy/test + build), release.yml (3 piattaforme), wiki.yml
```

### 4.1 Contratti condivisi (`src/backend/`)
Definiti **prima** di lanciare gli agenti, in modo che i moduli siano sviluppabili in parallelo:

- `Algo` — enum con `ALL`, `DEFAULT`, `name()`, `digest_len()`, `group()`, `candidates_for_hex_len()`.
- `trait StreamHasher { fn update(&mut self, &[u8]); fn finalize(self: Box<Self>) -> Vec<u8>; }`
  (indipendente dalle versioni del crate `digest`, così BLAKE3/CRC/xxHash si integrano uniformemente).
- `FileInfo`, `FileResult { id, info, digests: BTreeMap<Algo, Vec<u8>>, elapsed }` (esadecimale su richiesta).
- `EngineEvent { Discovered, Started, Progress, Finished, Failed, AllDone }` via `crossbeam-channel`.
- `Settings` (serde) con `load()` / `save()`.
- `ExportFormat` + `fn export(path, format, &[FileResult], &ExportOptions) -> Result<()>`.

### 4.2 Motore di hashing
- Per ogni file: un thread legge a blocchi di 1 MiB; per ogni blocco gli hasher attivi vengono
  aggiornati **in parallelo** con `rayon` (`par_iter_mut`, solo con più hasher e blocchi ≥ 64 KiB),
  poi il contatore atomico dei byte avanza.
- Più file in parallelo (numero configurabile, default = min(core, 4)).
- Annullamento cooperativo con `AtomicBool`; l'interfaccia richiede repaint ogni 50 ms mentre
  il motore lavora.
- ED2K: MD4 su blocchi da 9.728.000 byte; se il file supera un blocco, MD4 della concatenazione
  dei digest; se la dimensione è un multiplo esatto si aggiunge il digest del blocco vuoto
  (convenzione eMule).

### 4.3 Internazionalizzazione
`rust-i18n` con file YAML per lingua compilati nel binario; `t!("chiave")` nell'interfaccia;
rilevamento lingua di sistema con `sys-locale`; fallback inglese.

### 4.4 Portabilità e build
- Linux: binario dinamico verso glibc/X11/Wayland/OpenGL/GTK3 (dialoghi) — tutte librerie presenti
  su qualsiasi desktop.
- Windows: `.exe` singolo (`windows_subsystem = "windows"`, icona e metadati via `winresource`),
  nessun installer necessario.
- macOS: bundle `Hasher.app` generato da script (Info.plist + ICNS), zip per la distribuzione.
- GitHub Actions: matrice ubuntu/windows/macos; artefatti `hasher-linux-x86_64.tar.gz`,
  `hasher-windows-x86_64.zip`, `hasher-macos-universal.zip`.
- Profilo release: `opt-level=3`, `lto="fat"`, `codegen-units=1`, `strip=true`.

---

## 5. Ripartizione del lavoro tra agenti

| Fase | Modulo | Agente | Motivo |
|---|---|---|---|
| 0 | Piano, asset (icone, font), contratti condivisi, `Cargo.toml`, prima build | coordinatore | definisce le interfacce |
| 1A | Motore hash: `algo.rs`, `hasher.rs`, `engine.rs`, `checksum_file.rs` + test con vettori noti | **Opus** | concorrenza, correttezza crittografica, ED2K |
| 1B | `fileinfo.rs` (attributi per OS) + `settings.rs` (persistenza portable) | **Sonnet** | logica lineare ben specificata |
| 1C | `export/` (TXT, CSV, JSON, XLSX, DOCX, checksum) + test | **Sonnet** | uso di librerie documentate |
| 1D | Interfaccia egui completa: tema, animazioni, viste, impostazioni, guida, hook screenshot | **Opus** | la parte più complessa e visiva |
| 1E | CI GitHub Actions, `build.rs`, script bundle macOS, README | **Sonnet** | configurazione standard |
| 2 | Integrazione, build, correzione errori di compilazione | coordinatore (+ Opus se serve) | |
| 3 | Traduzioni complete fr/de/es a partire da it/en | **Sonnet** | |
| 4 | Test grafico headless (Xvfb + Mesa) con screenshot, revisione visiva, rifiniture | coordinatore + **Opus** | |
| 5 | `cargo clippy`, `cargo test`, revisione del codice, build release Linux | coordinatore | |

Regole operative per gli agenti: non eseguire mai `cargo build --release`; usare
`CARGO_BUILD_JOBS=2` e serializzare i `cargo check` con `flock` (memoria limitata, ~2 GB
liberi); consultare le API reali nel registro locale `~/.cargo/registry/src` invece di andare a
memoria.

---

## 6. Criteri di accettazione

1. Trascinando un file compaiono metadati e tutti gli hash di default; i valori coincidono con
   `sha256sum`, `md5sum`, `rhash` (CRC32, ED2K, RIPEMD).
2. Trascinando una cartella con 100 file la tabella si popola progressivamente, con barra
   globale, velocità e annullamento funzionante.
3. Incollando un hash corretto appare la conferma verde con il nome dell'algoritmo; con uno
   sbagliato appare l'errore rosso.
4. Trascinando un file `.sha256` i file elencati vengono verificati con esito per riga.
5. Esportazione in TXT/CSV/JSON/XLSX/DOCX apribile rispettivamente in editor, LibreOffice Calc,
   Excel, Word.
6. Cambio lingua immediato tra le 5 lingue; tema che segue il sistema e override manuale.
7. Impostazioni salvate accanto all'eseguibile e ricaricate all'avvio.
8. Guida e Informazioni accessibili da menu e con `F1`.
9. `cargo test` e `cargo clippy -- -D warnings` puliti; CI verde su tre piattaforme.

---

## 7. Rischi e mitigazioni

| Rischio | Mitigazione |
|---|---|
| Memoria limitata in compilazione | build serializzate, `jobs=2`, backend `glow` (più leggero di wgpu) |
| Dialoghi file su Linux senza GTK3 | GTK3 è presente su ogni desktop; in alternativa feature `xdg-portal` di `rfd` |
| Versioni crate RustCrypto non allineate | trait `StreamHasher` proprio, nessuna dipendenza dal trait `Digest` esterno |
| Convenzione ED2K sui multipli esatti | documentata (eMule), test dedicato |
| Nessun display per i test | Xvfb + Mesa llvmpipe, flag `--screenshot` che salva un PNG ed esce |
| Build Windows/macOS non eseguibili in locale | GitHub Actions; cross-compile Windows opzionale con `cargo-zigbuild` |
