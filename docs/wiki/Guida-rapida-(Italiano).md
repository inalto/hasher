# Guida rapida (Italiano)

**Hasher** calcola e verifica gli hash di uno o più file. È *portable* (un solo eseguibile, nessun installer), funziona su **Windows, macOS e Linux** ed è sviluppato in Rust da **Martini Multimedia s.a.s.** Le altre pagine di questa wiki sono in inglese; l'interfaccia del programma è disponibile in italiano e segue la lingua del sistema.

![Hasher con una lista di file, tema scuro](images/list-dark.png)

## Installazione

Scarica il pacchetto dalla pagina [Releases](https://github.com/inalto/hasher/releases):

- **Windows**: decomprimi lo zip ed esegui `hasher.exe`. L'eseguibile non è firmato: se compare *"Windows ha protetto il PC"* scegli *Ulteriori informazioni → Esegui comunque*.
- **macOS**: decomprimi lo zip e sposta `Hasher.app` dove vuoi. L'app non è firmata né notarizzata; al primo avvio esegui nel Terminale `xattr -dr com.apple.quarantine Hasher.app`, oppure clic destro → *Apri*. Serve macOS 10.15 o successivo (Intel e Apple Silicon).
- **Linux**: `tar xzf hasher-<versione>-linux-x86_64.tar.gz`, poi `./hasher`. Servono GTK 3, X11 o Wayland e OpenGL (presenti su qualsiasi desktop). Per la voce nel menu delle applicazioni vedi [[Installation]].

Per aggiornare basta sostituire l'eseguibile; per disinstallare si cancella la cartella (e, se vuoi, il file `hasher.settings.json`).

## Calcolare gli hash: trascina e rilascia

1. **Trascina** uno o più file (o intere cartelle) sulla finestra, oppure usa **File → Apri file…** (`Ctrl+O`) o **Apri cartella…** (`Ctrl+Shift+O`). Si possono anche passare i percorsi come argomenti: `hasher file1 cartella2`.
2. Con **un solo file** compare una scheda con nome, percorso, dimensione (leggibile e in byte), date di creazione e modifica, attributi e permessi, più una riga per ogni hash con il pulsante **copia**. **Copia tutti** copia l'intero elenco.
3. Con **più file o cartelle** compare una tabella: scegli nell'intestazione l'algoritmo mostrato nella colonna, filtra per nome, fai clic su una riga per espanderla e vedere tutti gli hash. La barra di stato mostra avanzamento, velocità e tempo rimanente; **Annulla** (o `Esc`) interrompe il calcolo e **Riprendi** ricalcola i file annullati.
4. Il pulsante con il numero di algoritmi (in alto a destra) permette di scegliere quali calcolare; **Ricalcola** rifà il calcolo sugli stessi file.

Di default sono attivi CRC32, MD4, MD5, SHA-1, SHA-256, SHA-512, RIPEMD-128, RIPEMD-160 ed ED2K. Gli algoritmi disponibili sono 22 (anche CRC64, xxHash, SHA-224/384, SHA3, RIPEMD-256/320, BLAKE2, BLAKE3 e MD2, che è molto lento e non è attivo di default). Vedi [[Algorithms]].

## Verificare un file

- **Hash incollato**: premi **Ctrl+V** e incolla l'hash pubblicato dall'autore. Maiuscole/minuscole, spazi, due punti e trattini vengono ignorati e l'algoritmo è riconosciuto dalla lunghezza. Il risultato è *Corrisponde* (verde) oppure *Nessuna corrispondenza* (rosso). Il confronto avviene solo con gli algoritmi calcolati: se serve, attivali e premi *Ricalcola*.
- **File di checksum**: trascina un file `.md5`, `.sha1`, `.sha256`, `.sha512`, `.sfv`… (formato GNU `hash  nome` o BSD `SHA256 (nome) = hash`). Hasher verifica tutti i file elencati, cercandoli nella cartella del file di checksum, e mostra **OK**, **FALLITO** o **MANCANTE** per ciascuno.
- **Confronto tra due file**: `Ctrl`+clic su due righe, oppure rilascia due file tenendo premuto `Ctrl`, oppure rilascia un file tenendo premuto `Shift` per confrontarlo con l'ultimo. Il pannello indica, algoritmo per algoritmo, se i file sono *identici* o *diversi*.

Dettagli ed esempi: [[Verifying Hashes]].

## Esportare i risultati

**File → Esporta…** (`Ctrl+E`) salva i risultati in **TXT**, **CSV** (UTF-8 con BOM, separatore virgola), **JSON**, **Excel (XLSX)**, **Word (DOCX)** oppure come **file di checksum** (`.sha256`, `.md5`, `.sfv`… in formato GNU, riutilizzabile con Hasher o con `sha256sum -c`). Se hai selezionato alcune righe puoi esportare solo quelle. Dettagli in [[Exporting]].

## Impostazioni

Apri le impostazioni con l'icona a ingranaggio o `Ctrl+,`:

- **Lingua** (automatica, italiano, inglese, francese, tedesco, spagnolo) e **tema** (automatico, chiaro, scuro);
- **algoritmi predefiniti**, **esadecimale maiuscolo**, esplorazione **ricorsiva** delle cartelle, link simbolici, file nascosti, **animazioni**;
- **file in parallelo** (automatico = il minore tra core della CPU e 4).

Le impostazioni sono salvate in `hasher.settings.json` **accanto all'eseguibile** (ideale per una chiavetta USB) se la cartella è scrivibile, altrimenti nella cartella di configurazione dell'utente (`%APPDATA%\MartiniMultimedia\Hasher\`, `~/Library/Application Support/MartiniMultimedia/Hasher/` o `~/.config/MartiniMultimedia/Hasher/`). La variabile d'ambiente `HASHER_SETTINGS_PATH` indica un percorso diverso. Nota: francese, tedesco e spagnolo sono selezionabili ma, al momento, le traduzioni sono incomplete e i testi mancanti appaiono in inglese. Vedi [[Settings]].

## Scorciatoie principali

Su macOS `Ctrl` corrisponde a `Cmd`.

| Tasti | Azione |
|---|---|
| `Ctrl+O` / `Ctrl+Shift+O` | Apri file / Apri cartella |
| `Ctrl+E` | Esporta i risultati |
| `Ctrl+V` | Incolla un hash da verificare |
| `Ctrl+L` | Svuota l'elenco |
| `Ctrl+,` | Impostazioni |
| `F1` | Guida |
| `Esc` | Annulla il calcolo o chiudi la finestra |
| `Canc` | Rimuovi i file selezionati |
| `Ctrl`+clic / `Shift`+clic | Seleziona più file (due file: confronto) / un intervallo |
| `Shift` + rilascio | Confronta il file rilasciato con l'ultimo |
| `Ctrl` + rilascio | Confronta i due file rilasciati |

## Problemi frequenti

- **Il calcolo è lento**: MD2 e, in misura minore, SHA3-512 rallentano tutto; deseleziona gli algoritmi che non ti servono. Su dischi meccanici imposta *File in parallelo* a 1.
- **La data di creazione è "n/d"**: il file system non la registra; è normale su alcuni sistemi Linux.
- **La finestra non si apre su Linux**: verifica GTK 3 e OpenGL; prova `LIBGL_ALWAYS_SOFTWARE=1 ./hasher`.
- **Un file `.sha256` trascinato da solo non viene calcolato**: viene trattato come file di checksum; trascinalo insieme a un altro file per calcolarne l'hash.

Altre risposte: [[FAQ and Troubleshooting]] (in inglese). Segnala problemi su <https://github.com/inalto/hasher/issues>.

*© Martini Multimedia s.a.s.*
