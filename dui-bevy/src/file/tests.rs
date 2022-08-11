use super::{LossySerialFile, SerialFile};
use crate::misc::test_utils::{catch_and_error, check_logged_errors, error};
use std::borrow::Cow;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt::Display;
use std::fs::{File as IOFile};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use derive_more::Display;
use log::{info, warn};
use test_log::test;
use serde::{Deserialize, Serialize};

#[derive(Debug, Display)]
struct SmallError(String);

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Display)]
#[display(fmt = "interface = {:?}", interface)]
struct ExpectedOrder {
    interface: Vec<String>
}

enum AssociatedIOFile<T> {
    ErrorReading(&'static str),
    CheckExpected(&'static str, T),
    WriteExpected(&'static str, String, IOFile)
}

impl<T> AssociatedIOFile<T> {
    fn map<R>(self, fun: fn(T) -> R) -> AssociatedIOFile<R> {
        match self {
            AssociatedIOFile::ErrorReading(associated_name) => AssociatedIOFile::ErrorReading(associated_name),
            AssociatedIOFile::CheckExpected(associated_name, v) => AssociatedIOFile::CheckExpected(associated_name, fun(v)),
            AssociatedIOFile::WriteExpected(associated_name, name, f) => AssociatedIOFile::WriteExpected(associated_name, name, f)
        }
    }

    fn and_then<R, E: Error>(self, name: &str, fun: fn(T) -> Result<R, E>) -> AssociatedIOFile<R> {
        match self {
            AssociatedIOFile::ErrorReading(associated_name) => AssociatedIOFile::ErrorReading(associated_name),
            AssociatedIOFile::CheckExpected(associated_name, v) => match fun(v) {
                Err(err) => {
                    error!("failed to parse expected {}: {}; {}", associated_name, name, err);
                    AssociatedIOFile::ErrorReading(associated_name)
                }
                Ok(v) => AssociatedIOFile::CheckExpected(associated_name, v)
            },
            AssociatedIOFile::WriteExpected(associated_name, name, f) => AssociatedIOFile::WriteExpected(associated_name, name, f)
        }
    }
}

impl<T: Eq + Display> AssociatedIOFile<T> {
    fn check_or_replace(self, errors: &mut Vec<SmallError>, get_actual: impl FnOnce() -> T) {
        match self {
            AssociatedIOFile::ErrorReading(_) => { /* skip, already reported */ },
            AssociatedIOFile::CheckExpected(associated_name, expected) => {
                let actual = get_actual();
                if actual != expected {
                    errors.push(SmallError(format!("expected {}, input mismatch\nexpected: <\n{}\n> but was <\n{}\n>", associated_name, expected, actual).into()));
                }
            }
            AssociatedIOFile::WriteExpected(associated_name, name, mut file) => {
                let actual = get_actual();
                let actual_display = actual.to_string();
                catch_and_error!(file.write(actual_display.as_bytes()), "failed to write actual {}", associated_name);
                warn!("wrote actual {} for {} for regression", associated_name, name)
            }
        }
    }
}

#[test]
fn test_serde() {
    check_logged_errors(|| {
        let mut inputs = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        inputs.push("test-resources/duis");
        let inputs = inputs.read_dir().expect("failed to read serde inputs (readdir)");
        for input in inputs {
            // region load input boilerplate
            if let Err(err) = input {
                error!("failed to open a serde input (readdir): {}", err);
                continue;
            }
            let input = input.unwrap();
            let file_path = input.path();
            if file_path.extension() != Some(OsStr::new("dui")) {
                info!("skipping non-dui file: {}", file_path.display());
                continue;
            }
            let _file_name = input.file_name();
            let name = match _file_name.to_string_lossy() {
                Cow::Borrowed(name) => &name[..name.len() - 4],
                Cow::Owned(_) => "<not utf-8>"
            };
            let input = IOFile::options().read(true).open(input.path());
            if let Err(err) = input {
                error!("failed to open serde input (open): {}; {}", name, err);
                continue;
            }
            let mut input = input.unwrap();
            let mut input_str = String::new();
            if let Err(err) = input.read_to_string(&mut input_str) {
                error!("failed to open serde input (read): {}; {}", name, err);
                continue;
            }
            let input = input_str;
            // endregion

            // region load associated inputs boilerplate
            let mut root_path = file_path;
            root_path.pop();
            root_path.pop();

            let load_associated = |associated_name: &'static str, extension: &str| -> AssociatedIOFile<String> {
                let mut expected_path = root_path.clone();
                expected_path.push("expected_");
                let mut expected_path = expected_path.into_os_string();
                expected_path.push(associated_name);
                let mut expected_path = PathBuf::from(expected_path);
                expected_path.push(name);
                expected_path.set_extension(extension);
                match IOFile::options().read(true).open(&expected_path) {
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            match IOFile::options().create_new(true).write(true).open(expected_path) {
                                Err(err) => {
                                    error!("failed to open expected {} file (for writing): {}; {}", associated_name, name, err);
                                    AssociatedIOFile::ErrorReading(associated_name)
                                }
                                Ok(file) => AssociatedIOFile::WriteExpected(associated_name, String::from(name), file)
                            }
                        } else {
                            error!("failed to open expected {} file (for reading): {}; {}", associated_name, name, err);
                            AssociatedIOFile::ErrorReading(associated_name)
                        }
                    }
                    Ok(mut file) => {
                        let mut expected_str = String::new();
                        if let Err(err) = file.read_to_string(&mut expected_str) {
                            error!("failed to read expected {} file: {}; {}", associated_name, name, err);
                            AssociatedIOFile::ErrorReading(associated_name)
                        } else if expected_str.is_empty() {
                            match IOFile::options().truncate(true).write(true).open(expected_path) {
                                Err(err) => {
                                    error!("failed to open expected {} file (for writing): {}; {}", associated_name, name, err);
                                    AssociatedIOFile::ErrorReading(associated_name)
                                }
                                Ok(file) => AssociatedIOFile::WriteExpected(associated_name, String::from(name), file)
                            }
                        } else {
                            AssociatedIOFile::CheckExpected(associated_name, expected_str)
                        }
                    }
                }
            };
            // endregion

            let expected_order = load_associated("order", "toml")
                .and_then(name, |associated| toml::from_str::<ExpectedOrder>(&associated));
            let expected_debug = load_associated("debug", "txt");

            match test_input(&input, expected_order, expected_debug) {
                Ok(errors) => {
                    if !errors.is_empty() {
                        error!("serde input failed: {};", name);
                        for error in errors {
                            error!("- {}", error);
                        }
                    }
                }
                Err(err) => error!("serde input failed: {};\n{}", name, err),
            }
        }
    })
}

fn test_input(input: &str, expected_order: AssociatedIOFile<ExpectedOrder>, expected_debug: AssociatedIOFile<String>) -> Result<Vec<SmallError>, Box<dyn Error>> {
    let mut errors = Vec::new();

    let serial_instance = SerialFile::try_from(input)?;

    // Test round trip
    let instance_str = toml::to_string_pretty(&serial_instance)?;
    // Serializer's format is a bit different than ours
    // So we have to re-serialize to really check
    let instance2 = SerialFile::try_from(&instance_str).map_err(|err| {
        error!("* failed on round-trip second deserialize");
        err
    })?;
    if instance2 != serial_instance {
        let instance_str2 = toml::to_string_pretty(&serial_instance)?;
        errors.push(SmallError(format!("round trip, input mismatch\nexpected: <\n{}\n> but was <\n{}\n> then was <\n{}\n> (ignore non-semantic differences)", input, instance_str, instance_str2).into()));
    }

    // Test expected order
    expected_order.check_or_replace(&mut errors, || ExpectedOrder {
        interface: serial_instance.interface.keys().cloned().collect()
    });

    // Test finish serialization
    let instance = LossySerialFile::try_from(serial_instance)?;

    // Test expected debug
    expected_debug.check_or_replace(&mut errors, || format!("{:?}", instance));

    Ok(errors)
}