# Algorithms

Hasher computes **22 algorithms** in a single pass over each file. Nine are active by default; the others are enabled in [[Settings]] or from the quick selector in the top bar.

## Catalogue

"Default" marks the algorithms selected out of the box. *Hex chars* is the length of the text you see, copy and paste.

| Algorithm | Group | Digest | Hex chars | Class | Default | Notes |
|---|---|---|---|---|---|---|
| CRC32 | Checksum | 32 bit | 8 | Checksum | yes | CRC-32/ISO-HDLC as in zlib, ZIP, PNG; shown big-endian like an SFV file. `"123456789"` → `cbf43926` |
| CRC64 | Checksum | 64 bit | 16 | Checksum | | **CRC-64/XZ** (ECMA-182 polynomial, reflected, init and xor-out all ones) as in `xz` and 7-Zip. `"123456789"` → `995dc9bbdf1939fa` |
| xxHash64 | Checksum | 64 bit | 16 | Checksum | | Seed 0, big-endian (canonical) output as printed by `xxh64sum` |
| xxHash3-128 | Checksum | 128 bit | 32 | Checksum | | XXH3 128-bit, seed 0, default secret, big-endian like `xxh128sum` |
| MD2 | MD | 128 bit | 32 | Legacy | | RFC 1319. Extremely slow and strictly serial: flagged **slow**, off by default |
| MD4 | MD | 128 bit | 32 | Legacy | yes | RFC 1320. Broken; included because ED2K is built on it and for old catalogues |
| MD5 | MD | 128 bit | 32 | Legacy | yes | RFC 1321. Collisions are easy to forge; fine against accidental corruption |
| SHA-1 | SHA | 160 bit | 40 | Legacy | yes | Practical collisions exist; still published by many projects |
| SHA-224 | SHA | 224 bit | 56 | Cryptographic | | SHA-2 family (FIPS 180-4) |
| SHA-256 | SHA | 256 bit | 64 | Cryptographic | yes | The most widely published hash |
| SHA-384 | SHA | 384 bit | 96 | Cryptographic | | SHA-2 family |
| SHA-512 | SHA | 512 bit | 128 | Cryptographic | yes | SHA-2 family; fast on 64-bit CPUs |
| SHA3-256 | SHA | 256 bit | 64 | Cryptographic | | Keccak (FIPS 202) |
| SHA3-512 | SHA | 512 bit | 128 | Cryptographic | | Keccak (FIPS 202); slower than SHA-2 in software |
| RIPEMD-128 | RIPEMD | 128 bit | 32 | Legacy | yes | Short digest, historical |
| RIPEMD-160 | RIPEMD | 160 bit | 40 | Cryptographic | yes | No practical attack known; used in some protocols |
| RIPEMD-256 | RIPEMD | 256 bit | 64 | Cryptographic* | | Wider output of RIPEMD-128 (*same security level as the 128-bit version*) |
| RIPEMD-320 | RIPEMD | 320 bit | 80 | Cryptographic* | | Wider output of RIPEMD-160 (same security level as the 160-bit version) |
| BLAKE2b-512 | BLAKE | 512 bit | 128 | Cryptographic | | RFC 7693, unkeyed; what `b2sum` prints |
| BLAKE2s-256 | BLAKE | 256 bit | 64 | Cryptographic | | RFC 7693, unkeyed |
| BLAKE3 | BLAKE | 256 bit | 64 | Cryptographic | | Default 32-byte output, unkeyed; same as `b3sum` |
| ED2K | P2P | 128 bit | 32 | Network id | yes | eDonkey/eMule file hash, built on MD4 (see below) |

The groups are the headings shown in the algorithm selectors (CHECKSUM, MD, SHA, RIPEMD, BLAKE, P2P). Which class to trust for what is explained at the end of [[Verifying Hashes]].

## Conventions worth knowing

### ED2K (eMule convention)

The file is split into **9,728,000-byte chunks**.

- A file **smaller than one chunk**: ED2K = MD4 of the data. An empty file gives `31d6cfe0d16ae931b73c59d7e0c089c0`.
- A larger file: ED2K = MD4 of the concatenation of the MD4 digests of all chunks (the last one may be shorter).
- **Exact multiple rule**: when the size is an exact multiple of 9,728,000 bytes (including exactly one chunk), the MD4 of an *empty* chunk is appended to the list before the final MD4. This is what eMule does and what Hasher implements; it is covered by a dedicated test at the chunk boundaries.

The 32 hex characters are the hash eMule shows and puts in links, `ed2k://|file|<name>|<size in bytes>|<hash>|/`. Hasher computes only this root hash (no AICH, no per-part list) and does not build links.

### Byte order of numeric checksums

CRC32, CRC64, xxHash64 and xxHash3-128 are printed as **big-endian** hexadecimal, the canonical text form used by SFV files, `xz`/7-Zip and `xxhsum`. A tool that prints a little-endian dump will show the bytes reversed; compare the value, not the byte order.

### Other notes

- **RIPEMD**: the original RIPEMD-128/160/256/320 family. The 256- and 320-bit versions only widen the output; they do not raise the security level.
- **SHA-3**: only the 256- and 512-bit variants are offered.
- **BLAKE2/BLAKE3**: plain hashing mode with the default output length; no keys, no tree parameters.

## Speed

Typical throughput when one algorithm is selected, measured with the project's engine on the development machine in a **debug (dev-profile) build**; release builds (`opt-level = 3`, fat LTO) are faster.

| Algorithm | Throughput |
|---|---|
| MD2 | ≈ 7 MB/s |
| SHA3-512 | ≈ 38 MB/s |
| SHA-256 | ≈ 150 MB/s |
| MD5 | ≈ 400 MB/s |
| BLAKE3 | ≈ 850 MB/s |
| CRC32 | ≈ 1,180 MB/s |

The figures are indicative and depend on the CPU; the reference point is the ratio between algorithms. With fast algorithms on a hard disk or a network share the storage is the bottleneck; on a fast SSD it is the slowest selected algorithm (MD2 above all).

### MD2 is special

MD2 is intrinsically serial and processes only about **7-10 MB/s**. Because all selected algorithms are updated together on every block, MD2 would slow the whole computation to that speed, so it is **not active by default** and carries a *slow* badge (with a tooltip) wherever it can be ticked. Enable it only when you need it.

### One pass, parallel updates

For each file Hasher reads 1 MiB blocks. Every block is fed to **all selected algorithms in parallel** (one `rayon` task per algorithm; for blocks smaller than 64 KiB or a single algorithm the update is sequential). The file is read once, however many algorithms are selected, and the time per block is roughly that of the **slowest selected algorithm** as long as you have enough CPU cores. In addition, several files are processed concurrently (automatic: min(cores, 4)). See [[Architecture]] for the details.

## Cross-checking with other tools

Results agree with the standard tools; this is how the test suite verifies them:

| Algorithm | Command |
|---|---|
| MD5, SHA-1, SHA-2 | `md5sum`, `sha1sum`, `sha224sum` ... `sha512sum` |
| BLAKE2b-512 / BLAKE3 | `b2sum` / `b3sum` |
| CRC32 | `python3 -c "import zlib,sys;print('%08x'%zlib.crc32(open(sys.argv[1],'rb').read()))" file` |
| xxHash | `xxh64sum`, `xxh128sum` |
| MD4 | `openssl dgst -md4 file` (OpenSSL 3 also needs `-provider legacy -provider default`) |
| RIPEMD, ED2K, CRC32 | `rhash` (for example `rhash --ed2k file`) |
