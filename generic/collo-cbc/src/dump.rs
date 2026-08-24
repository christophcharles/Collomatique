// A faithful, name-free export of the numbers that reach the solver, and the
// reader that feeds them back in.
//
// By the time a problem gets to `collo_cbc_solve` it is pure numbers: the
// `ProblemDesc` fields, an optional MIP start and a log level — nothing else is
// set. Dumping those is therefore a complete reproducer of a solve that
// misbehaves in the field, and it carries no names of students, teachers or
// subjects.
//
// The format is line-oriented text: one section per line, the section key then
// its values separated by whitespace. Rust's `f64` `Display` is
// shortest-round-trip and `str::parse::<f64>` inverts it exactly; `inf` and
// `-inf` round-trip too, so plain decimal text is lossless here.
//
// MPS was the obvious alternative and was rejected on purpose: it cannot carry
// a MIP start at all, the objective *sense* does not round-trip through it
// cleanly (writers emit coefficients as-is and readers assume minimize), and it
// would need file IO on the C++ side. Faithfulness is the point here; interop
// is not.

use std::fmt::Display;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::str::FromStr;

use crate::ProblemDesc;

const MODEL_HEADER: &str = "collo-cbc-model";
const MIP_START_HEADER: &str = "collo-cbc-mipstart";
const FORMAT_VERSION: i32 = 1;

fn invalid(msg: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

fn write_scalar<W: Write>(w: &mut W, key: &str, value: i32) -> io::Result<()> {
    writeln!(w, "{key} {value}")
}

fn write_array<W: Write, T: Display>(w: &mut W, key: &str, values: &[T]) -> io::Result<()> {
    write!(w, "{key}")?;
    for v in values {
        write!(w, " {v}")?;
    }
    writeln!(w)
}

/// Line-at-a-time reader that reports the line number in every error, so a
/// truncated or hand-edited dump says where it went wrong.
struct SectionReader<R: BufRead> {
    inner: R,
    line_no: usize,
}

impl<R: BufRead> SectionReader<R> {
    fn new(inner: R) -> Self {
        SectionReader { inner, line_no: 0 }
    }

    fn take_line(&mut self) -> io::Result<String> {
        let mut line = String::new();
        if self.inner.read_line(&mut line)? == 0 {
            return Err(invalid(format!(
                "line {}: unexpected end of file",
                self.line_no + 1
            )));
        }
        self.line_no += 1;
        while line.ends_with('\n') || line.ends_with('\r') {
            line.pop();
        }
        Ok(line)
    }

    /// Read the next line, check it starts with `key`, and return the rest.
    fn section(&mut self, key: &str) -> io::Result<String> {
        let line = self.take_line()?;
        let (found, rest) = match line.find(char::is_whitespace) {
            Some(i) => (&line[..i], line[i..].trim()),
            None => (line.as_str(), ""),
        };
        if found != key {
            return Err(invalid(format!(
                "line {}: expected section `{key}`, found `{found}`",
                self.line_no
            )));
        }
        Ok(rest.to_string())
    }

    fn scalar<T: FromStr>(&mut self, key: &str) -> io::Result<T> {
        let rest = self.section(key)?;
        rest.parse::<T>().map_err(|_| {
            invalid(format!(
                "line {}: `{key}` expects a single number, found `{rest}`",
                self.line_no
            ))
        })
    }

    fn array<T: FromStr>(&mut self, key: &str, expected_len: usize) -> io::Result<Vec<T>> {
        let rest = self.section(key)?;
        let mut values = Vec::with_capacity(expected_len);
        for token in rest.split_whitespace() {
            let value = token.parse::<T>().map_err(|_| {
                invalid(format!(
                    "line {}: `{key}` contains a value that is not a number: `{token}`",
                    self.line_no
                ))
            })?;
            values.push(value);
        }
        if values.len() != expected_len {
            return Err(invalid(format!(
                "line {}: `{key}` has {} values, expected {expected_len}",
                self.line_no,
                values.len()
            )));
        }
        Ok(values)
    }

    /// Check the `<name> <version>` first line of a dump.
    fn header(&mut self, name: &str) -> io::Result<()> {
        let version: i32 = self.scalar(name)?;
        if version != FORMAT_VERSION {
            return Err(invalid(format!(
                "line {}: `{name}` format version {version} is not supported \
                 (this build reads version {FORMAT_VERSION})",
                self.line_no
            )));
        }
        Ok(())
    }
}

impl ProblemDesc {
    /// Write the problem to `path`, creating or truncating it.
    pub fn write_to(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        self.write(&mut writer)?;
        writer.flush()
    }

    pub fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        writeln!(w, "{MODEL_HEADER} {FORMAT_VERSION}")?;
        write_scalar(w, "num_cols", self.num_cols)?;
        write_scalar(w, "num_rows", self.num_rows)?;
        write_scalar(w, "obj_sense", self.obj_sense)?;
        write_array(w, "col_lb", &self.col_lb)?;
        write_array(w, "col_ub", &self.col_ub)?;
        write_array(w, "obj_coeffs", &self.obj_coeffs)?;
        write_array(w, "is_integer", &self.is_integer)?;
        write_array(w, "mat_start", &self.mat_start)?;
        write_array(w, "mat_index", &self.mat_index)?;
        write_array(w, "mat_value", &self.mat_value)?;
        write_array(w, "row_lb", &self.row_lb)?;
        write_array(w, "row_ub", &self.row_ub)
    }

    /// Read back a problem written by [`ProblemDesc::write_to`].
    pub fn read_from(path: impl AsRef<Path>) -> io::Result<ProblemDesc> {
        let file = File::open(path)?;
        ProblemDesc::read(file)
    }

    pub fn read<R: Read>(reader: R) -> io::Result<ProblemDesc> {
        let mut r = SectionReader::new(BufReader::new(reader));
        r.header(MODEL_HEADER)?;

        let num_cols: i32 = r.scalar("num_cols")?;
        let num_rows: i32 = r.scalar("num_rows")?;
        if num_cols < 0 || num_rows < 0 {
            return Err(invalid(format!(
                "num_cols ({num_cols}) and num_rows ({num_rows}) must not be negative"
            )));
        }
        let cols = num_cols as usize;
        let rows = num_rows as usize;
        let obj_sense: i32 = r.scalar("obj_sense")?;

        let col_lb = r.array("col_lb", cols)?;
        let col_ub = r.array("col_ub", cols)?;
        let obj_coeffs = r.array("obj_coeffs", cols)?;
        let is_integer = r.array("is_integer", cols)?;
        // The CSC start array carries one extra entry: the end of the last column.
        let mat_start: Vec<i32> = r.array("mat_start", cols + 1)?;
        let nnz = *mat_start.last().unwrap_or(&0);
        if nnz < 0 {
            return Err(invalid(format!(
                "mat_start ends at {nnz}, which is not a valid non-zero count"
            )));
        }
        let mat_index: Vec<i32> = r.array("mat_index", nnz as usize)?;
        let mat_value = r.array("mat_value", nnz as usize)?;
        let row_lb = r.array("row_lb", rows)?;
        let row_ub = r.array("row_ub", rows)?;

        if let Some(bad) = mat_index.iter().find(|&&i| i < 0 || i >= num_rows) {
            return Err(invalid(format!(
                "mat_index contains row {bad}, outside 0..{num_rows}"
            )));
        }

        Ok(ProblemDesc {
            num_cols,
            num_rows,
            obj_sense,
            col_lb,
            col_ub,
            obj_coeffs,
            is_integer,
            mat_start,
            mat_index,
            mat_value,
            row_lb,
            row_ub,
        })
    }
}

/// Write a MIP start (one value per original column) to `path`.
pub fn write_mip_start(path: impl AsRef<Path>, values: &[f64]) -> io::Result<()> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);
    writeln!(w, "{MIP_START_HEADER} {FORMAT_VERSION}")?;
    write_scalar(&mut w, "num_cols", values.len() as i32)?;
    write_array(&mut w, "values", values)?;
    w.flush()
}

/// Read back a MIP start written by [`write_mip_start`].
pub fn read_mip_start(path: impl AsRef<Path>) -> io::Result<Vec<f64>> {
    let file = File::open(path)?;
    let mut r = SectionReader::new(BufReader::new(file));
    r.header(MIP_START_HEADER)?;
    let num_cols: i32 = r.scalar("num_cols")?;
    if num_cols < 0 {
        return Err(invalid(format!(
            "num_cols ({num_cols}) must not be negative"
        )));
    }
    r.array("values", num_cols as usize)
}
