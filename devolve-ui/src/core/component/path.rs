//! Identifies where a component is in the UI.
//! Specifically, given a `VComponentRoot` and a `VNodePath`, you can get the corresponding
//! component with `Renderer::with_component`. The component may not exist in which case
//! `with_component` returns `None`.

use std::fmt::{Debug, Display, Formatter, Write};
use std::ops::Add;
use std::ops::AddAssign;
use std::rc::Weak;
use std::mem::MaybeUninit;
use arrayvec::{ArrayString, CapacityError};
use crate::core::component::component::VComponent;
use crate::core::component::root::VComponentRoot;
use crate::core::view::view::VViewData;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

/// Identifies a `VComponent` among its siblings.
/// Needed because the siblings may change and we need to remember the component and check if it was deleted.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", repr(transparent))]
pub struct VComponentKey(ArrayString<{ VComponentKey::SIZE }>);

impl VComponentKey {
    pub const SIZE: usize = 16;
}

/// The location of a `VNode` in the node tree.
/// Primarily used to let components listen to events emitted by the root:
/// events may be emitted at any time and we don't have a mutable reference to a component at any time.
/// However, we do have a reference to the renderer at any time, which allows us to get the mutable component reference
/// from its path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VComponentPath(Vec<VComponentKey>);

// region ref
#[derive(Clone)]
pub struct VComponentRef<ViewData: VViewData> {
    pub(super) renderer: Weak<dyn VComponentRoot<ViewData = ViewData>>,
    pub(super) path: VComponentPath
}

impl <ViewData: VViewData> VComponentRef<ViewData> {
    pub fn with<R>(&self, fun: impl FnOnce(Option<&mut Box<VComponent<ViewData>>>) -> R) -> R {
        match self.renderer.upgrade() {
            None => fun(None),
            Some(renderer) => {
                // We can't return values in renderer's `with` because it's a trait object
                let mut return_value: MaybeUninit<R> = MaybeUninit::uninit();
                renderer.with_component(&self.path, |component| {
                    return_value.write(fun(component));
                });
                unsafe { return_value.assume_init() }
            }
        }
    }

    pub fn try_with<R>(&self, fun: impl FnOnce(&mut Box<VComponent<ViewData>>) -> R) -> Option<R> {
        self.with(|component| {
            component.map(fun)
        })
    }
}
// endregion

// region boilerplate
impl VComponentPath {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn iter(&self) -> impl Iterator<Item = &VComponentKey> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut VComponentKey> {
        self.0.iter_mut()
    }
}

impl IntoIterator for VComponentPath {
    type Item = VComponentKey;
    type IntoIter = std::vec::IntoIter<VComponentKey>;

    fn into_iter(self) -> std::vec::IntoIter<VComponentKey> {
        self.0.into_iter()
    }
}

impl Display for VComponentKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for VComponentPath {
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

impl Add<VComponentKey> for VComponentPath {
    type Output = VComponentPath;

    fn add(mut self, rhs: VComponentKey) -> Self::Output {
        self.0.push(rhs);
        self
    }
}

impl AddAssign<VComponentKey> for VComponentPath {
    fn add_assign(&mut self, rhs: VComponentKey) {
        self.0.push(rhs)
    }
}

impl VComponentKey {
    pub fn new(data: ArrayString<{ VComponentKey::SIZE }>) -> Self {
        VComponentKey(data)
    }
}

impl From<()> for VComponentKey {
    fn from((): ()) -> Self {
        VComponentKey(ArrayString::new())
    }
}

impl <'a> TryFrom<&'a str> for VComponentKey {
    type Error = CapacityError<&'a str>;

    fn try_from(str: &'a str) -> Result<Self, Self::Error> {
        Ok(VComponentKey(ArrayString::try_from(str)?))
    }
}

impl From<usize> for VComponentKey {
    fn from(index: usize) -> Self {
        let mut str = ArrayString::new();
        write!(str, "{}", index).expect("didn't expect usize not to fit in VComponentKey");
        VComponentKey(str)
    }
}

impl <'a> TryFrom<(&'a str, usize)> for VComponentKey {
    type Error = CapacityError<(&'a str, usize)>;

    fn try_from((key, index): (&'a str, usize)) -> Result<Self, Self::Error> {
        let mut str = ArrayString::new();
        write!(str, "{}{}", key, index).map_err(|_err| CapacityError::new((key, index)))?;
        Ok(VComponentKey(str))
    }
}

impl TryFrom<String> for VComponentKey {
    type Error = CapacityError<String>;

    fn try_from(string: String) -> Result<Self, Self::Error> {
        let str = ArrayString::from(&string).map_err(|_err| ());
        Ok(VComponentKey(str.map_err(|()| CapacityError::new(string))?))
    }
}

impl <ViewData: VViewData> Debug for VComponentRef<ViewData> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VComponentRef")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl <ViewData: VViewData> PartialEq for VComponentRef<ViewData> {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }

    fn ne(&self, other: &Self) -> bool {
        self.path != other.path
    }
}
// endregion