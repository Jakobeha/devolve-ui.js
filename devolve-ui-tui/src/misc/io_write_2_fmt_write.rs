use std::{io, fmt};

/// Adapts `io::Write` to `fmt::Write`.
/// Does not flush after every write, you must call `flush` manually.
pub struct IoWrite2FmtWrite<'a, W: io::Write>(&'a mut W);

impl <'a, W: io::Write> IoWrite2FmtWrite<'a, W> {
    pub fn with<F: FnOnce(&mut IoWrite2FmtWrite<W>) -> Result<R, fmt::Error>, R>(w: &'a mut W, f: F) -> Result<R, io::Error> {
        f(&mut IoWrite2FmtWrite(w)).map_err(|fmt_err| io::Error::new(io::ErrorKind::Other, fmt_err))
    }

    pub fn new(inner: &'a mut W) -> IoWrite2FmtWrite<'a, W> {
        IoWrite2FmtWrite(inner)
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl <'a, W: io::Write> fmt::Write for IoWrite2FmtWrite<'a, W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write!(self.0, "{}", s).map_err(|_| fmt::Error)
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        write!(self.0, "{}", c).map_err(|_| fmt::Error)
    }

    fn write_fmt(&mut self, args: fmt::Arguments) -> fmt::Result {
        self.0.write_fmt(args).map_err(|_e| fmt::Error)
    }
}