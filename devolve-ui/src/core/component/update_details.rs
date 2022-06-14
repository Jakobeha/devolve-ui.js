use std::fmt::{Display, Formatter};
use std::borrow::Cow;

#[derive(Debug)]
pub struct RecursiveUpdateStack(Vec<RecursiveUpdateFrame>);

impl RecursiveUpdateStack {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    fn last_open(&mut self) -> Option<&mut RecursiveUpdateFrame> {
        self.0.last_mut().filter(|frame| frame.is_open)
    }

    fn last_open_or_make(&mut self) -> &mut RecursiveUpdateFrame {
        if !self.0.last_mut().is_some_and(|frame| frame.is_open) {
            self.0.push(RecursiveUpdateFrame::new())
        }
        self.last_open().unwrap()
    }

    pub fn add_to_last(&mut self, name: Cow<'static, str>) {
        self.last_open_or_make().add(name);
    }

    pub fn close_last(&mut self) {
        if let Some(last) = self.last_open() {
            last.close();
        }
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl Display for RecursiveUpdateStack {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for frame in self.0.iter() {
            write!(f, "{}\n", frame)?;
        }
        write!(f, "---")?;
        Ok(())
    }
}

#[derive(Debug)]
struct RecursiveUpdateFrame {
    simultaneous: Vec<Cow<'static, str>>,
    is_open: bool
}

impl RecursiveUpdateFrame {
    pub fn new() -> Self {
        Self {
            simultaneous: Vec::new(),
            is_open: true
        }
    }

    pub fn close(&mut self) {
        assert!(self.is_open, "already closed");
        self.is_open = false;
    }

    pub fn add(&mut self, name: Cow<'static, str>) {
        assert!(self.is_open, "closed");
        self.simultaneous.push(name);
    }
}

impl Display for RecursiveUpdateFrame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.simultaneous.join(", "))
    }
}
