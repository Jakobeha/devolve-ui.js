use std::fmt::{Display, Formatter};
use std::ops::Add;
use std::ops::AddAssign;

/// Identifies a `VComponent` among its siblings.
/// Needed because the siblings may change and we need to remember the component and check if it was deleted.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct VNodeKey(&'static str, usize, Option<String>);

/// A path segment in `VNodePath` (see `VNodePath`)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub enum VNodePathSegment {
    ComponentChild,
    ViewChildWithKey(VNodeKey),
    ViewChildWithIndex(usize),
}

/// The location of a `VNode` in the node tree.
/// Primarily used to let components listen to events emitted by the root:
/// events may be emitted at any time and we don't have a mutable reference to a component at any time.
/// However, we do have a reference to the renderer at any time, which allows us to get the mutable component reference
/// from its path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct VNodePath(Vec<VNodePathSegment>);

// region boilerplate
impl VNodePath {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn iter(&self) -> impl Iterator<Item = &VNodePathSegment> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut VNodePathSegment> {
        self.0.iter_mut()
    }
}

impl IntoIterator for VNodePath {
    type Item = VNodePathSegment;
    type IntoIter = std::vec::IntoIter<VNodePathSegment>;

    fn into_iter(self) -> std::vec::IntoIter<VNodePathSegment> {
        self.0.into_iter()
    }
}

impl Display for VNodeKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let VNodeKey(static_, index, arbitrary) = self;
        // if !static_.is_empty()
        write!(f, "{}", static_)?;
        if *index > 0 {
            write!(f, "{}", index - 1)?;
        }
        if let Some(arbitrary) = arbitrary.as_ref() {
            write!(f, "{}", arbitrary)?;
        }
        Ok(())
    }
}

impl Display for VNodePathSegment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VNodePathSegment::ComponentChild => write!(f, "0"),
            VNodePathSegment::ViewChildWithKey(key) => write!(f, "{}", key),
            VNodePathSegment::ViewChildWithIndex(index) => write!(f, "{}", index)
        }
    }
}

impl Display for VNodePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.0.iter();
        match iter.next() {
            None => write!(f, "<empty>")?,
            Some(first) => {
                write!(f, "{}", first)?;
                for next in iter {
                    write!(f, ".{}", next)?;
                }
            }
        }
        Ok(())
    }
}

impl Add<VNodePathSegment> for VNodePath {
    type Output = VNodePath;

    fn add(mut self, rhs: VNodePathSegment) -> Self::Output {
        self.0.push(rhs);
        self
    }
}

impl Add<VNodeKey> for VNodePath {
    type Output = VNodePath;

    fn add(mut self, rhs: VNodeKey) -> Self::Output {
        self.0.push(VNodePathSegment::ViewChildWithKey(rhs));
        self
    }
}

impl AddAssign<VNodePathSegment> for VNodePath {
    fn add_assign(&mut self, rhs: VNodePathSegment) {
        self.0.push(rhs);
    }
}

impl AddAssign<VNodeKey> for VNodePath {
    fn add_assign(&mut self, rhs: VNodeKey) {
        self.0.push(VNodePathSegment::ViewChildWithKey(rhs))
    }
}

impl From<VNodeKey> for VNodePathSegment {
    fn from(key: VNodeKey) -> Self {
        VNodePathSegment::ViewChildWithKey(key)
    }
}

impl VNodeKey {
    pub fn new(static_: &'static str, index: usize, arbitrary: Option<String>) -> Self {
        VNodeKey(static_, index, arbitrary)
    }
}

impl Default for VNodeKey {
    fn default() -> Self {
        VNodeKey("", 0, None)
    }
}

impl From<()> for VNodeKey {
    fn from((): ()) -> Self {
        VNodeKey("", 0, None)
    }
}

impl From<&'static str> for VNodeKey {
    fn from(str: &'static str) -> Self {
        VNodeKey(str, 0, None)
    }
}

impl From<usize> for VNodeKey {
    fn from(index: usize) -> Self {
        VNodeKey("", index + 1, None)
    }
}

impl From<(&'static str, usize)> for VNodeKey {
    fn from((str, index): (&'static str, usize)) -> Self {
        VNodeKey(str, index, None)
    }
}

impl From<String> for VNodeKey {
    fn from(string: String) -> Self {
        VNodeKey("", 0, Some(string))
    }
}
// endregion