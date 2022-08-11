mod measurement;
mod interface;
mod var;
mod serialize_with_global_vars;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::fmt::{Display, Formatter};
use bytemuck::cast_slice_mut;
pub use measurement::*;
pub use interface::*;
pub use var::*;

use serde::{Deserialize, Serialize};
use devolve_ui_core::dui_impl::DuiRead;
use derive_more::{Display, Error, From};
use devolve_ui_core::dui::{DuiMetaFieldShmemReader, ShmemReaderError, DuiInstanceKey, DuiInterface, DuiMeta};
use join_lazy_fmt::Join;
use slicevec::SliceVec;
use devolve_ui_core::dui_impl::typical_engine::TypicalDuiFile;
use crate::gpu::{FileGpuAdapter, GpuRenderContext, Vertex};
use crate::runtime::Runtime;

// region type defs

/// Lossy representation of a .dui file which we can actually render from.
#[derive(Debug)]
pub struct File {
    pub interface: Interface,
    pub views: Vec<View>,
    gpu: FileGpuAdapter,
    input_readers: HashMap<DuiInstanceKey, Vec<Option<DuiMetaFieldShmemReader<f64>>>>
}

/// Intermediate representation which is still deserialized (aka no runtime info)
/// but can't be serialized back.
#[derive(Debug, Deserialize)]
#[serde(try_from = "SerialFile")]
pub(super) struct LossySerialFile {
    interface: Interface,
    views: Vec<View>,
}

/// Lossless representation of a .dui file
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SerialFile {
    #[serde(with = "serialize_with_global_vars::set")]
    pub interface: Interface,
    pub common: HashMap<String, PartialView>,
    // This annotation has to be on the last field which contains Measurements
    #[serde(with = "serialize_with_global_vars::clear")]
    pub views: Vec<PartialView>
}

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Rect {
        key: Option<String>,
        bounds: Bounds,
        color: Color
    },
    Circle {
        key: Option<String>,
        bounds: Bounds,
        color: Color
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PartialView {
    pub key: Option<String>,
    pub bounds: Option<Bounds>,
    #[serde(rename = "type")]
    pub type_: Option<ViewType>,
    pub color: Option<Color>,
    pub inherits: Option<String>
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Color {
    Red,
    Blue
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewType {
    #[display(fmt = "rect")]
    Rect,
    #[display(fmt = "circle")]
    Circle
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "[Measurement; 4]", into = "[Measurement; 4]")]
pub struct Bounds {
    pub left: Measurement,
    pub top: Measurement,
    pub right: Measurement,
    pub bottom: Measurement,
}
// endregion

// region dui-file
#[derive(Debug, Display, Error, From)]
pub enum ConfigureError {}

#[derive(Debug, Display, Error, From)]
pub enum OpenError {
    IO(std::io::Error),
    Deserialize(toml::de::Error)
}


#[derive(Debug, Display, Error, From)]
pub enum BeforeSpawnError {
    InterfaceMismatch(InterfaceMismatchError),
    #[display(fmt = "failed to setup reader for {}: {}", field_name, error)]
    ReaderSetup {
        field_name: &'static str,
        #[error(source)]
        error: ShmemReaderError
    },
    #[display(fmt = "field reads an output: {} reads {}", field, var_index)]
    ReadsOutput {
        field: Measurement,
        var_index: VarIndex
    }
}

#[derive(Debug, Display, Error)]
#[display(fmt = "interface mismatch: {}\nContext: <\n{}\n> vs <\n{}\n>", message, expected, actual)]
pub struct InterfaceMismatchError {
    message: String,
    expected: DuiMeta,
    actual: Interface
}

#[derive(Debug, Display, Error, From)]
pub enum RenderError {}

impl TypicalDuiFile for File {
    type Shared = ();
    type Runtime = Runtime;
    type ConfigureError = ConfigureError;
    type OpenError = OpenError;
    type BeforeSpawnError = BeforeSpawnError;
    type RenderError = RenderError;

    fn prefix() -> &'static str {
        "# dui-basic"
    }

    fn configure(_runtime: &mut Runtime) -> Result<Self::Shared, ConfigureError> {
        Ok(())
    }

    unsafe fn open(runtime: &mut GpuRenderContext<'_>, _shared: &(), data: &mut DuiRead) -> Result<Self, OpenError> {
        let mut string = String::new();
        data.read_to_string(&mut string)?;
        let file = File::try_new(&string, runtime)?;
        Ok(file)
    }

    unsafe fn before_spawn(&mut self, _runtime: &mut GpuRenderContext<'_>, _shared: &(), key: DuiInstanceKey, io: &dyn DuiInterface) -> Result<(), BeforeSpawnError> {
        self.check_interface(io).map_err(BeforeSpawnError::from)?;
        self.initialize_input_readers(key, io)?;
        self.check_render(key)
    }

    unsafe fn render(&mut self, runtime: &mut GpuRenderContext<'_>, _shared: &(), key: DuiInstanceKey, io: &dyn DuiInterface) -> Result<(), RenderError> {
        if cfg!(debug_assertions) {
            self.check_interface(io).expect("interface mismatch after before_spawn")
        }

        let input_readers = self.input_readers.get(&key).expect("instance rendered before it was configured");
        self.gpu.render(runtime, |gpu| {
            let mut vertex_buffer_ = gpu.vertex_buffer.slice(..).get_mapped_range_mut();
            let mut index_buffer_ = gpu.index_buffer.slice(..).get_mapped_range_mut();
            let mut vertex_buffer = SliceVec::new(cast_slice_mut(&mut *vertex_buffer_));
            let mut index_buffer = SliceVec::new(cast_slice_mut(&mut *index_buffer_));
            let mut vertex_index = 0;
            for view in self.views.iter() {
                match view {
                    View::Rect { key: _key, bounds, color } => {
                        let left = Self::eval(input_readers, bounds.left, io) as f32;
                        let top = Self::eval(input_readers, bounds.top, io) as f32;
                        let right = Self::eval(input_readers, bounds.right, io) as f32;
                        let bottom = Self::eval(input_readers, bounds.bottom, io) as f32;
                        let color = color.to_rgba();
                        vertex_buffer.push(Vertex {
                            position: [left, top],
                            color
                        }).unwrap();
                        vertex_buffer.push(Vertex {
                            position: [right, top],
                            color
                        }).unwrap();
                        vertex_buffer.push(Vertex {
                            position: [right, bottom],
                            color
                        }).unwrap();
                        vertex_buffer.push(Vertex {
                            position: [left, bottom],
                            color
                        }).unwrap();
                        index_buffer.push(vertex_index as u16).unwrap();
                        index_buffer.push((vertex_index + 1) as u16).unwrap();
                        index_buffer.push((vertex_index + 2) as u16).unwrap();
                        index_buffer.push((vertex_index + 2) as u16).unwrap();
                        index_buffer.push((vertex_index + 3) as u16).unwrap();
                        index_buffer.push(vertex_index as u16).unwrap();
                        vertex_index += 4;
                    }
                    View::Circle { key: _key, bounds, color } => {
                        let left = Self::eval(input_readers, bounds.left, io) as f32;
                        let top = Self::eval(input_readers, bounds.top, io) as f32;
                        let right = Self::eval(input_readers, bounds.right, io) as f32;
                        let bottom = Self::eval(input_readers, bounds.bottom, io) as f32;
                        let color = color.to_rgba();
                        let x_radius = (right - left) / 2.0;
                        let y_radius = (top - bottom) / 2.0;
                        let center = [left + x_radius, bottom + y_radius];
                        vertex_buffer.push(Vertex {
                            position: center,
                            color
                        }).unwrap();
                        for i in 0..24 {
                            let angle = i as f32 * std::f32::consts::PI / 12.0;
                            let x = center[0] + x_radius * angle.cos();
                            let y = center[1] + y_radius * angle.sin();
                            vertex_buffer.push(Vertex {
                                position: [x, y],
                                color
                            }).unwrap();
                        }
                        for i in 0..24 {
                            index_buffer.push((vertex_index + i) as u16).unwrap();
                            index_buffer.push((vertex_index + i + 1) as u16).unwrap();
                            index_buffer.push((vertex_index + i + 2) as u16).unwrap();
                        }
                        vertex_index += 25;
                    }
                }
            }
        });

        Ok(())
    }
}

struct FileGpuInfo {
    num_vertices: u16,
    num_indices: u32
}

impl File {
    fn try_new(serial: &str, runtime: &mut GpuRenderContext<'_>) -> Result<Self, toml::de::Error> {
        let serial = LossySerialFile::try_from(serial)?;
        let FileGpuInfo { num_vertices, num_indices } = Self::get_gpu_info(&serial);
        Ok(File {
            interface: serial.interface,
            views: serial.views,
            gpu: FileGpuAdapter::new(runtime.gpu_adapter(), num_vertices, num_indices),
            input_readers: HashMap::new()
        })
    }

    fn get_gpu_info(serial: &LossySerialFile) -> FileGpuInfo {
        FileGpuInfo {
            num_vertices: serial.views.iter().map(|view| match view {
                View::Rect { .. } => 4,
                View::Circle { .. } => 25
            }).sum(),
            num_indices: serial.views.iter().map(|view| match view {
                View::Rect { .. } => 6,
                View::Circle { .. } => 72
            }).sum()
        }
    }

    fn check_interface(&mut self, io: &dyn DuiInterface) -> Result<(), InterfaceMismatchError> {
        let meta = io.meta();
        let fields = &meta.fields;
        let err_mismatch = |message| {
            InterfaceMismatchError {
                message,
                expected: meta.clone(),
                actual: self.interface.clone()
            }
        };

        if fields.len() != self.interface.len() {
            return Err(err_mismatch(format!("wrong num fields")));
        }
        let fields = fields.iter().zip(self.interface.iter());
        for (real_field, our_field) in fields {
            if real_field.name != our_field.name {
                return Err(err_mismatch(format!("field name expected {} got {}", real_field.name, our_field.name)));
            }
            if our_field.value.type_() != real_field.kind {
                return Err(err_mismatch(format!("field type expected {} got {}", real_field.type_, our_field.value.type_())));
            }
        }

        Ok(())
    }

    fn initialize_input_readers(&mut self, key: DuiInstanceKey, io: &dyn DuiInterface) -> Result<(), BeforeSpawnError> {
        debug_assert!(!self.input_readers.contains_key(&key), "already initialized input readers for instance {}", key);
        let meta = io.meta();
        self.input_readers.insert(key, meta.fields.iter().map(|field| {
            field.shmem_reader()
                // If we have a reader, good
                .map(Some)
                // If this was an output field, we just put None. Otherwise...
                .or_else(|error| if matches!(error, ShmemReaderError::Output) { Ok(None) } else { Err(error) })
                // propogate the error outside of this function
                .map_err(|error| BeforeSpawnError::ReaderSetup {
                    field_name: field.name,
                    error
                })
        }).try_collect()?);
        Ok(())
    }

    fn check_render(&self, key: DuiInstanceKey) -> Result<(), BeforeSpawnError> {
        let input_readers = self.input_readers.get(&key).expect("input readers must be created before check_render");
        for view in self.views.iter() {
            match view {
                View::Rect { key: _key, bounds, color: _color } => {
                    Self::check(input_readers, bounds.left)?;
                    Self::check(input_readers, bounds.top)?;
                    Self::check(input_readers, bounds.right)?;
                    Self::check(input_readers, bounds.bottom)?;
                }
                View::Circle { key: _key, bounds, color: _color } => {
                    Self::check(input_readers, bounds.left)?;
                    Self::check(input_readers, bounds.top)?;
                    Self::check(input_readers, bounds.right)?;
                    Self::check(input_readers, bounds.bottom)?;
                }
            }
        }

        Ok(())
    }

    fn check(input_readers: &Vec<Option<DuiMetaFieldShmemReader<f64>>>, field: Measurement) -> Result<(), BeforeSpawnError> {
        field.check(|var_index| {
            if input_readers[var_index.0].is_none() {
                Err(BeforeSpawnError::ReadsOutput { field, var_index })
            } else {
                Ok(())
            }
        })
    }

    fn eval(input_readers: &Vec<Option<DuiMetaFieldShmemReader<f64>>>, field: Measurement, io: &dyn DuiInterface) -> f64 {
        field.eval(|var_index| {
            let input_reader = input_readers[var_index.0].as_ref().expect("this is an output (should've been checked at configure stage");
            unsafe { input_reader.read(io).into_owned() }
        })
    }
}

impl Color {
    fn to_rgba(self) -> [f32; 4] {
        match self {
            Color::Red => [1.0, 0.0, 0.0, 1.0],
            Color::Blue => [0.0, 0.0, 1.0, 1.0],
        }
    }
}
// endregion

// region serde
#[derive(Debug, Error)]
pub struct CompleteViewError {
    key: Option<String>,
    details: CompleteViewErrorDetails
}

impl Display for CompleteViewError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(key) = &self.key {
            write!(f, "in {}", key)?;
        }
        write!(f, "{}", self.details)
    }
}

#[derive(Debug, Display, Error)]
pub enum CompleteViewErrorDetails {
    #[display(fmt = "inherited view not found: {}", _0)]
    InheritedViewNotFound(#[error(not(source))] String),
    #[display(fmt = "view is missing a type")]
    MissingType,
    #[display(fmt = "recursively inherits {}", _0)]
    RecursivelyInherits(#[error(not(source))] String),
    #[display(fmt = "incomplete {}, missing fields {}", type_, "\", \".join(missing_fields)")]
    MissingFields {
        type_: ViewType,
        missing_fields: Vec<&'static str>
    }
}

impl TryFrom<SerialFile> for LossySerialFile {
    type Error = CompleteViewError;

    fn try_from(serial: SerialFile) -> Result<Self, Self::Error> {
        Ok(LossySerialFile {
            interface: serial.interface,
            views: serial.views.into_iter().map(|view| view.complete(&serial.common)).try_collect::<Vec<View>>()?,
        })
    }
}

impl TryFrom<PartialView> for View {
    type Error = CompleteViewError;

    fn try_from(partial: PartialView) -> Result<Self, Self::Error> {
        if partial.type_.is_none() {
            return partial.err(CompleteViewErrorDetails::MissingType)
        }
        let type_ = partial.type_.unwrap();

        Ok(match type_ {
            ViewType::Rect => {
                let mut missing_fields = Vec::new();
                if partial.bounds.is_none() {
                    missing_fields.push("bounds")
                }
                if partial.color.is_none() {
                    missing_fields.push("color")
                }
                if !missing_fields.is_empty() {
                    return partial.err(CompleteViewErrorDetails::MissingFields {
                        type_,
                        missing_fields
                    })
                }

                View::Rect {
                    key: partial.key,
                    bounds: partial.bounds.unwrap(),
                    color: partial.color.unwrap()
                }
            }
            ViewType::Circle => {
                let mut missing_fields = Vec::new();
                if partial.bounds.is_none() {
                    missing_fields.push("bounds")
                }
                if partial.color.is_none() {
                    missing_fields.push("color")
                }
                if !missing_fields.is_empty() {
                    return partial.err(CompleteViewErrorDetails::MissingFields {
                        type_,
                        missing_fields
                    })
                }

                View::Circle {
                    key: partial.key,
                    bounds: partial.bounds.unwrap(),
                    color: partial.color.unwrap()
                }
            }
        })
    }
}

impl PartialView {
    pub fn complete(mut self, common: &HashMap<String, PartialView>) -> Result<View, CompleteViewError> {
        // We want to use take because we want to replace inherits with None after each call.
        // It will be replaced again if the inherited field itself inherits another
        let mut already_inherits = HashSet::new();
        while let Some(inherits) = self.inherits.take() {
            let inherited = match common.get(&inherits) {
                None => return self.err(CompleteViewErrorDetails::InheritedViewNotFound(inherits)),
                Some(inherited) => inherited
            };
            self.inherit_from(inherited);

            if already_inherits.contains(&inherits) {
                return self.err(CompleteViewErrorDetails::RecursivelyInherits(inherits))
            }
            already_inherits.insert(inherits);
        }

        self.try_into()
    }

    /// Useful helper to create error results with this view's key
    pub fn err<T>(self, details: CompleteViewErrorDetails) -> Result<T, CompleteViewError> {
        Err(CompleteViewError {
            key: self.key,
            details
        })
    }

    /// Adds all missing fields with those fields from [inherited].
    pub fn inherit_from(&mut self, inherited: &PartialView) {
        Self::inherit_field(&mut self.type_, &inherited.type_);
        Self::inherit_field(&mut self.bounds, &inherited.bounds);
        Self::inherit_field(&mut self.color, &inherited.color);
        Self::inherit_field(&mut self.inherits, &inherited.inherits);
    }

    fn inherit_field<T: Clone>(field: &mut Option<T>, inherited: &Option<T>) {
        if field.is_none() {
            field.clone_from(inherited)
        }
    }
}

impl<'a> TryFrom<&'a String> for LossySerialFile {
    type Error = toml::de::Error;

    fn try_from(str: &'a String) -> Result<Self, Self::Error> {
        toml::from_str(str)
    }
}

impl<'a> TryFrom<&'a str> for LossySerialFile {
    type Error = toml::de::Error;

    fn try_from(str: &'a str) -> Result<Self, Self::Error> {
        toml::from_str(str)
    }
}

impl<'a> TryFrom<&'a [u8]> for LossySerialFile {
    type Error = toml::de::Error;

    fn try_from(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        toml::from_slice(bytes)
    }
}

impl<'a> TryFrom<&'a String> for SerialFile {
    type Error = toml::de::Error;

    fn try_from(str: &'a String) -> Result<Self, Self::Error> {
        toml::from_str(str)
    }
}

impl<'a> TryFrom<&'a str> for SerialFile {
    type Error = toml::de::Error;

    fn try_from(str: &'a str) -> Result<Self, Self::Error> {
        toml::from_str(str)
    }
}

impl<'a> TryFrom<&'a [u8]> for SerialFile {
    type Error = toml::de::Error;

    fn try_from(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        toml::from_slice(bytes)
    }
}

impl From<[Measurement; 4]> for Bounds {
    fn from(array: [Measurement; 4]) -> Self {
        Bounds {
            left: array[0],
            top: array[1],
            right: array[2],
            bottom: array[3]
        }
    }
}

impl Into<[Measurement; 4]> for Bounds {
    fn into(self) -> [Measurement; 4] {
        [self.left, self.top, self.right, self.bottom]
    }
}
// endregion