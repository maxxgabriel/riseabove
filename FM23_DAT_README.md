# FM23 `.dat` reader

`fm23_dat_tool.py` is a read-only extractor for the database files shipped with Football Manager 2023.

The files are proprietary `tad.` containers containing independent Zstandard-compressed frames. The tool:

- validates and reports frame offsets and sizes;
- decompresses every frame into `block_XXXXX.bin` files;
- writes `manifest.json` and a best-effort `strings.json`.
- preserves files without Zstandard frames as `raw.bin`.

It does not modify the game installation and does not repack files. The extracted blocks are useful for format analysis, backups, and building a later parser. They are not directly loadable by FM23 because the game expects the original container and its internal schemas.

## Usage (PowerShell)

```powershell
python .\fm23_dat_tool.py inspect "C:\Football Manager 2023\data\database\db\2300\2300_fm\people_db.dat"

python .\fm23_dat_tool.py extract `
  "C:\Football Manager 2023\data\database\db\2300\2300_fm\people_db.dat" `
  ".\fm23_extracted\people_db"
```

The tool uses the `libzstd.dll` commonly installed with Git for Windows. If it is elsewhere, set `ZSTD_DLL` to the full DLL path before running it.

## What the files are

The companion `2300_fm.xml` identifies this as the official Football Manager 2023 database, version 23.0.0. The main tables include `people_db.dat`, `server_db.dat`, `client_db.dat`, language data, and historical-data/index files. The index files are especially useful for reconstructing record boundaries; the decompressed blocks alone are not yet a human-readable database export.
