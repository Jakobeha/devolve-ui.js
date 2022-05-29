use std::{io, fmt};
use replace_with::replace_with_or_abort_and_return;

/// Adapts `io::Write` to `fmt::Write`.
/// Does not flush after every write, you must call `flush` manually.
pub struct IoWrite2FmtWrite<W: io::Write>(W);

impl <W: io::Write> IoWrite2FmtWrite<W> {
    pub fn on<F: FnOnce(&mut IoWrite2FmtWrite<W>) -> R, R>(w: &mut W, f: F) -> R {
        replace_with_or_abort_and_return(w, |w| {
            let mut this = IoWrite2FmtWrite(w);
            let result = f(&mut this);
            (result, this.unwrap())
        })
    }

    pub fn new(inner: W) -> IoWrite2FmtWrite<W> {
        IoWrite2FmtWrite(inner)
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }

    pub fn unwrap(self) -> W {
        self.0
    }
}

impl <W: io::Write> fmt::Write for IoWrite2FmtWrite<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_all(s.as_bytes()).map_err(|_e| fmt::Error)
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.0.write_all(c.encode_utf8(&mut [0; 4]).as_bytes()).map_err(|_e| fmt::Error)
    }

    fn write_fmt(&mut self, args: fmt::Arguments) -> fmt::Result {
        self.0.write_fmt(args).map_err(|_e| fmt::Error)
    }
}