# Changelog

Tutte le modifiche rilevanti a Hasher sono documentate in questo file.
Il formato segue [Keep a Changelog](https://keepachangelog.com/it/1.1.0/) e il progetto adotta il
[Versionamento Semantico](https://semver.org/lang/it/).

## [0.1.0] - 2026-10-09

Prima versione.

### Aggiunto

- Calcolo di 22 algoritmi in un'unica lettura del file, in parallelo: CRC32, CRC64, xxHash64,
  xxHash3-128, MD2, MD4, MD5, SHA-1, SHA-224, SHA-256, SHA-384, SHA-512, SHA3-256, SHA3-512,
  RIPEMD-128/160/256/320, BLAKE2b-512, BLAKE2s-256, BLAKE3 ed ED2K (convenzione eMule).
- Vista a file singolo con metadati (percorso, dimensione, date, attributi e permessi) e pulsante
  *copia* per ogni hash.
- Vista a lista per più file e cartelle (ricorsione configurabile), righe espandibili, avanzamento
  globale con velocità e tempo stimato, annullamento.
- Drag & drop dal sistema operativo, con indicazione visiva della zona di rilascio e confronto con
  Shift/Ctrl.
- Verifica: hash atteso incollato (riconoscimento dell'algoritmo dalla lunghezza), file di checksum
  (`.md5`, `.sha1`, `.sha256`, `.sha512`, `.sfv`, formati GNU e BSD) e confronto tra due file.
- Esportazione in TXT, CSV, JSON, XLSX, DOCX e file di checksum.
- Interfaccia in italiano, inglese, francese, tedesco e spagnolo.
- Tema chiaro e scuro automatico (segue il sistema) con scelta manuale.
- Impostazioni portable in `hasher.settings.json` accanto all'eseguibile, con ripiego nella
  cartella di configurazione dell'utente.
- Guida, scorciatoie da tastiera e informazioni su licenze e versione.
- Pacchetti portable per Linux (`.tar.gz`), Windows (`.zip`) e macOS (`Hasher.app` universale in
  `.zip`, non firmata), con CI e rilascio automatico su GitHub Actions.
