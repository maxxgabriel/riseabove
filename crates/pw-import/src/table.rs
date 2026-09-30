//! CSV reading shared by every adapter: header lookup by name, streaming rows, UTF-8 or Windows-1252 text,
//! and validated parsing of dates and numbers (a bad value is `None`, never a default).

use std::borrow::Cow;
use std::io::Read;
use std::path::Path;

use pw_core::Date;
use pw_core::date::days_in_month;
use rustc_hash::FxHashMap;

use crate::ImportError;

#[derive(Clone, Copy)]
pub struct Opts {
    pub delimiter: u8,
    /// The file is Windows-1252, not UTF-8.
    pub cp1252: bool,
}

impl Default for Opts {
    fn default() -> Self {
        Self { delimiter: b',', cp1252: false }
    }
}

type Cols = FxHashMap<String, usize>;

pub struct Table {
    pub file: String,
    cols: Cols,
    rdr: csv::Reader<Box<dyn Read>>,
    /// Fields that had to be decoded lossily (invalid UTF-8).
    pub lossy_fields: u32,
}

/// One row, addressed by column name. Missing columns and empty cells read as `None` / `""`.
pub struct Rec<'a> {
    cols: &'a Cols,
    r: &'a csv::ByteRecord,
    lossy: &'a std::cell::Cell<u32>,
}

/// Windows-1252 to text. Bytes 0x80-0x9F differ from Latin-1; undefined ones become U+FFFD.
pub fn decode_cp1252(bytes: &[u8]) -> String {
    const HIGH: [char; 32] = [
        '€', '\u{fffd}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{fffd}', 'Ž', '\u{fffd}', '\u{fffd}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{fffd}', 'ž', 'Ÿ',
    ];
    bytes
        .iter()
        .map(|&b| match b {
            0x80..=0x9f => HIGH[usize::from(b - 0x80)],
            _ => char::from(b),
        })
        .collect()
}

impl Table {
    /// `None` when the file does not exist.
    pub fn open(dir: &Path, name: &str, opts: Opts) -> Result<Option<Table>, ImportError> {
        let path = dir.join(name);
        if !path.exists() {
            return Ok(None);
        }
        let io = |e: std::io::Error| ImportError::Io { file: name.into(), source: e };
        let src: Box<dyn Read> = if opts.cp1252 { Box::new(std::io::Cursor::new(decode_cp1252(&std::fs::read(&path).map_err(io)?).into_bytes())) } else { Box::new(std::io::BufReader::with_capacity(1 << 20, std::fs::File::open(&path).map_err(io)?)) };
        Self::from_reader(name, src, opts)
    }

    #[cfg(test)]
    pub fn from_text(name: &str, text: &str, opts: Opts) -> Result<Table, ImportError> {
        Self::from_reader(name, Box::new(std::io::Cursor::new(text.as_bytes().to_vec())), opts).map(|t| t.expect("always some"))
    }

    fn from_reader(name: &str, src: Box<dyn Read>, opts: Opts) -> Result<Option<Table>, ImportError> {
        let err = |source| ImportError::Csv { file: name.into(), source };
        let mut rdr = csv::ReaderBuilder::new().flexible(true).delimiter(opts.delimiter).trim(csv::Trim::All).from_reader(src);
        let cols = rdr.byte_headers().map_err(err)?.iter().enumerate().map(|(i, h)| (String::from_utf8_lossy(h).trim().trim_start_matches('\u{feff}').to_ascii_lowercase(), i)).collect();
        Ok(Some(Table { file: name.into(), cols, rdr, lossy_fields: 0 }))
    }

    /// Every row in file order with its 1-based data row number. Malformed rows are counted, not fatal.
    pub fn for_each(&mut self, mut f: impl FnMut(usize, &Rec)) -> Result<u32, ImportError> {
        let lossy = std::cell::Cell::new(0u32);
        let mut malformed = 0u32;
        let mut row = 0usize;
        let mut rec = csv::ByteRecord::new();
        loop {
            match self.rdr.read_byte_record(&mut rec) {
                Ok(true) => {
                    row += 1;
                    f(row, &Rec { cols: &self.cols, r: &rec, lossy: &lossy });
                }
                Ok(false) => break,
                Err(source) if source.is_io_error() => return Err(ImportError::Csv { file: self.file.clone(), source }),
                Err(_) => {
                    row += 1;
                    malformed += 1;
                }
            }
        }
        self.lossy_fields = lossy.get();
        Ok(malformed)
    }
}

impl Rec<'_> {
    pub fn s(&self, col: &str) -> Cow<'_, str> {
        let Some(bytes) = self.cols.get(col).and_then(|&i| self.r.get(i)) else { return Cow::Borrowed("") };
        match std::str::from_utf8(bytes) {
            Ok(s) => Cow::Borrowed(s.trim()),
            Err(_) => {
                self.lossy.set(self.lossy.get() + 1);
                Cow::Owned(String::from_utf8_lossy(bytes).trim().to_string())
            }
        }
    }

    /// Present and non-empty text.
    pub fn text(&self, col: &str) -> Option<String> {
        let s = self.s(col);
        if s.is_empty() { None } else { Some(s.into_owned()) }
    }

    pub fn f64(&self, col: &str) -> Option<f64> {
        let s = self.s(col).replace([',', '_', '\u{a0}'], "");
        s.parse::<f64>().ok().filter(|v| v.is_finite())
    }

    pub fn int(&self, col: &str) -> Option<i64> {
        self.f64(col).filter(|v| v.abs() < 9.0e15).map(|v| v.round() as i64)
    }

    pub fn num<T: std::str::FromStr>(&self, col: &str) -> Option<T> {
        let s = self.s(col).replace([',', '_'], "");
        if s.is_empty() { None } else { s.parse().ok() }
    }

    pub fn date(&self, col: &str) -> Option<Date> {
        parse_date(&self.s(col))
    }

    /// Whether the cell holds text that is not a valid date (as opposed to being empty).
    pub fn bad_date(&self, col: &str) -> bool {
        let s = self.s(col);
        !s.is_empty() && parse_date(&s).is_none()
    }
}

/// `YYYY-MM-DD` (a trailing time is ignored) or `DD/MM/YYYY`, with real calendar validation.
pub fn parse_date(s: &str) -> Option<Date> {
    let s = s.trim().split([' ', 'T']).next()?;
    let parts: Vec<&str> = s.split(['-', '/', '.']).collect();
    let [a, b, c] = parts[..] else { return None };
    let (y, m, d): (i32, u32, u32) = if a.len() == 4 { (a.parse().ok()?, b.parse().ok()?, c.parse().ok()?) } else { (c.parse().ok()?, b.parse().ok()?, a.parse().ok()?) };
    ((1850..=2200).contains(&y) && (1..=12).contains(&m) && d >= 1 && d <= days_in_month(y, m)).then(|| Date::from_ymd(y, m, d))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_validated_not_wrapped() {
        assert_eq!(parse_date("1978-06-09 00:00:00"), Some(Date::from_ymd(1978, 6, 9)));
        assert_eq!(parse_date("09/06/1978"), Some(Date::from_ymd(1978, 6, 9)));
        assert_eq!(parse_date("2023-02-29"), None, "not a leap year");
        assert_eq!(parse_date("2024-02-29"), Some(Date::from_ymd(2024, 2, 29)));
        assert_eq!(parse_date("2024-13-01"), None);
        assert_eq!(parse_date("2024-00-10"), None);
        assert_eq!(parse_date("0000-00-00"), None);
        assert_eq!(parse_date("soon"), None);
        assert_eq!(parse_date(""), None);
    }

    #[test]
    fn cp1252_decodes_accents_and_specials() {
        assert_eq!(decode_cp1252(b"Br\xe9sil \x80 \x93x\x94"), "Brésil € “x”");
        assert_eq!(decode_cp1252(b"1\xa0830"), "1\u{a0}830");
    }

    #[test]
    fn rows_are_read_by_column_name_with_bom_delimiter_and_quotes() {
        let text = "\u{feff}\"Name\";\"Age\";\"Wage\"\n\"Aarab, Hamza\";\"28\";\"1\u{a0}830\"\n\"Ok\";\"x\";\"\"\n";
        let mut t = Table::from_text("s.csv", text, Opts { delimiter: b';', cp1252: false }).unwrap();
        let mut rows = Vec::new();
        t.for_each(|n, r| rows.push((n, r.s("name").to_string(), r.num::<u8>("age"), r.int("wage")))).unwrap();
        assert_eq!(rows, vec![(1, "Aarab, Hamza".into(), Some(28), Some(1830)), (2, "Ok".into(), None, None)]);
    }

    #[test]
    fn invalid_utf8_is_counted_and_never_panics() {
        let mut t = Table::open(&write_temp("bad.csv", b"name\nAl\xe9x\nBo\n"), "bad.csv", Opts::default()).unwrap().unwrap();
        let mut names = Vec::new();
        t.for_each(|_, r| names.push(r.s("name").to_string())).unwrap();
        assert_eq!(names.len(), 2);
        assert_eq!(t.lossy_fields, 1);
        assert!(names[0].contains('\u{fffd}'));
    }

    fn write_temp(name: &str, data: &[u8]) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("pw-import-table-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(name), data).unwrap();
        d
    }
}
