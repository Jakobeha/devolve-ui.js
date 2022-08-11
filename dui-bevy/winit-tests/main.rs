#![feature(future_join)]

use std::future::join;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use bevy::app::App;
use bevy::DefaultPlugins;
use devolve_ui_macros::DuiInterface;
use devolve_ui_core::dui::{DuiMeta, DuiMetaField, DuiMetaFieldKind, DuiMetaIOType, DuiMetaType, In, Out};
use devolve_ui_core::dui_impl::DuiManager;
use devolve_ui_core::dui_impl::typical_engine::TypicalDuiEngine;
use dui_basic::file::File;
use dui_basic::runtime::Runtime;
use winit_modular::exit;

#[derive(Debug, DuiInterface)]
#[repr(C)]
pub struct BallPlatformerSimpleInterface {
    x: In<f32>,
    y: In<f32>,
    radius: f32,
    keys: Out<[u32; 32]>
}

const EXPECTED_BALL_PLATFORMER_INTERFACE_META: DuiMeta = DuiMeta {
    type_: DuiMetaType::of::<BallPlatformerSimpleInterface>(),
    fields: &DuiMetaField::compute_offsets([
        DuiMetaField {
            name: "x",
            offset: 0,
            type_: DuiMetaType::of::<In<f32>>(),
            kind: DuiMetaFieldKind::Atom {
                io_type: DuiMetaIOType::In,
                inner_type: DuiMetaType::of::<f32>()
            }
        },
        DuiMetaField {
            name: "y",
            offset: 0,
            type_: DuiMetaType::of::<In<f32>>(),
            kind: DuiMetaFieldKind::Atom {
                io_type: DuiMetaIOType::In,
                inner_type: DuiMetaType::of::<f32>()
            }
        },
        DuiMetaField {
            name: "radius",
            offset: 0,
            type_: DuiMetaType::of::<f32>(),
            kind: DuiMetaFieldKind::Atom {
                io_type: DuiMetaIOType::Const,
                inner_type: DuiMetaType::of::<f32>()
            }
        },
        DuiMetaField {
            name: "keys",
            offset: 0,
            type_: DuiMetaType::of::<Out<[u32; 32]>>(),
            kind: DuiMetaFieldKind::Atom {
                io_type: DuiMetaIOType::Out,
                inner_type: DuiMetaType::of::<[u32; 32]>()
            }
        }
    ])
};

fn dui_dir() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.push("test-resources/duis");
    dir
}

fn ball_platformer_simple() {
    assert_eq!(&BallPlatformerSimpleInterface::META, &EXPECTED_BALL_PLATFORMER_INTERFACE_META);

    let mut path = dui_dir();
    path.push("ball-platformer-simple.dui");

    let interface = BallPlatformerSimpleInterface {
        x: In::new(4.0),
        y: In::new(6.0),
        radius: 1.0,
        keys: Out::new()
    };

    let dui = DuiManager::new(|| Runtime::try_new(|x| x), [Box::new(|| Ok(Box::new(TypicalDuiEngine::<File>::new())))]);
    let instance = dui.spawn(path, interface).expect("failed to spawn instance");

    let interface = instance.downcast_interface::<BallPlatformerSimpleInterface>().expect("instance couldn't downcast");
    assert_eq!(interface.radius, 1.0);
    assert_eq!(interface.x.dui_get(), Some(&4.0));
    assert_eq!(interface.y.dui_get(), Some(&6.0));

    sleep(Duration::from_secs(2));

    assert_eq!(interface.radius, 1.0);

    for i in 1..101 {
        interface.x.set(4.0 + (i as f32 * 0.03));
        interface.y.set(6.0 - (i as f32 * 0.03));
        sleep(Duration::from_millis(40));
    }

    assert_eq!(interface.radius, 1.0);
    assert_eq!(interface.x.dui_get(), Some(&7.0));
    assert_eq!(interface.y.dui_get(), Some(&3.0));
}

async fn ball_platformer_async() {
    assert_eq!(&BallPlatformerSimpleInterface::META, &EXPECTED_BALL_PLATFORMER_INTERFACE_META);

    let mut path = dui_dir();
    path.push("ball-platformer-simple.dui");

    let interface = BallPlatformerSimpleInterface {
        x: In::new(4.0),
        y: In::new(6.0),
        radius: 1.0,
        keys: Out::new()
    };

    let dui = DuiManager::new(|| Runtime::try_new(|x| x), [Box::new(|| Ok(Box::new(TypicalDuiEngine::<File>::new())))]);
    let instance = dui.spawn(path, interface).expect("failed to spawn instance");

    let interface = instance.downcast_interface::<BallPlatformerSimpleInterface>().expect("instance couldn't downcast");
    assert_eq!(interface.radius, 1.0);
    assert_eq!(interface.x.dui_get(), Some(&4.0));
    assert_eq!(interface.y.dui_get(), Some(&6.0));

    tokio::time::sleep(Duration::from_secs(2)).await;

    assert_eq!(interface.radius, 1.0);

    for i in 1..101 {
        interface.x.set(4.0 + (i as f32 * 0.03));
        interface.y.set(6.0 - (i as f32 * 0.03));
        tokio::time::sleep(Duration::from_millis(40)).await;
    }

    assert_eq!(interface.radius, 1.0);
    assert_eq!(interface.x.dui_get(), Some(&7.0));
    assert_eq!(interface.y.dui_get(), Some(&3.0));
}

fn main() {
    let _ = env_logger::init();
    App::new()
        .add_plugins(DefaultPlugins)
        .add_pl
        .run()
}