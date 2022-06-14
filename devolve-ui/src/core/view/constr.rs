//! Utilities to create terse constructors for your custom views,
//! since creating `VView`s manually is very verbose.

use crate::core::component::node::VNode;
use crate::core::view::layout::bounds::{Bounds, LayoutPosition, Measurement};
use crate::core::view::view::{VView, VViewData};

/// Arguments for a less verbose constructor
#[derive(Debug)]
pub struct VViewConstrArgs {
    pub layout: LayoutPosition,
    pub x: Measurement,
    pub y: Measurement,
    pub z: i32,
    pub width: Option<Measurement>,
    pub height: Option<Measurement>,
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub is_visible: bool,
}

impl Default for VViewConstrArgs {
    fn default() -> Self {
        VViewConstrArgs {
            layout: Bounds::default().layout,
            x: Bounds::default().x,
            y: Bounds::default().y,
            z: Bounds::default().z,
            width: Bounds::default().width,
            height: Bounds::default().height,
            anchor_x: Bounds::default().anchor_x,
            anchor_y: Bounds::default().anchor_y,
            is_visible: true
        }
    }
}

/// Create a less verbose constructor for a specific type of view
pub fn constr_view<ViewData: VViewData>(
    view_args: VViewConstrArgs,
    view_data: ViewData
) -> VNode<ViewData> {
    VNode::View(Box::new(VView::new(
        Bounds {
            layout: view_args.layout,
            x: view_args.x,
            y: view_args.y,
            z: view_args.z,
            width: view_args.width,
            height: view_args.height,
            anchor_x: view_args.anchor_x,
            anchor_y: view_args.anchor_y
        },
        view_args.is_visible,
        view_data
    )))
}

/// Create a less verbose constructor for a specific type of view.
///
/// Pro tip: In IntelliJ you must have `_make_view` imported in scope in order for this to expand in IDE.
///
/// ## Examples
///
/// ```rust
/// use devolve_ui::core::view::constr::{_make_view, make_view};
/// use devolve_ui::core::view::color::Color;
/// use devolve_ui::view_data::attrs::{BorderStyle, DividerDirection, DividerStyle, TextWrapMode};
/// use devolve_ui::view_data::tui::terminal_image::{HandleAspectRatio, Source};
/// use devolve_ui::view_data::tui::tui::TuiViewData;
///
/// make_view!(pub text, TextConstrArgs {
///     color: Option<Color>,
///     wrap_mode: TextWrapMode
/// } [ text: String ], TuiViewData::Text);
///
/// text!({
///   layout: LayoutPosition::xy(LayoutPosition1D::LocalAbsolute)
/// }, {
///   wrap_mode: TextWrapMode::Word
/// }, "Hello World!");
///
/// make_view!(pub ptext, PTextConstrArgs {
///     color: Option<Color>
/// } [ text: String ], TuiViewData::Text { wrap_mode: TextWrapMode::Word });
///
/// ptext!({
///   layout: LayoutPosition::xy(LayoutPosition1D::LocalAbsolute)
/// }, {}, "Hello World!");
///
/// make_view!(pub color, [ color: Color ], TuiViewData::Color);
///
/// color!({}, Color::RED);
///
/// make_view!(pub border, BorderConstrArgs {
///     color: Option<Color>
/// } [ style: BorderStyle ], TuiViewData::Border);
///
/// border!({}, { color: Some(Color::YELLOW) }, BorderStyle::Ascii);
///
/// make_view!(pub hdivider, HDividerConstrArgs {
///     color: Option<Color>
/// } [ style: DividerStyle ], TuiViewData::Divider { direction: DividerDirection::Horizontal });
///
/// make_view!(pub vdivider, VDividerConstrArgs {
///     color: Option<Color>
/// } [ style: DividerStyle ], TuiViewData::Divider { direction: DividerDirection::Vertical });
///
/// hdivider!({}, {}, DividerStyle::Double);
/// vdivider!({}, {}, DividerStyle::Double);
///
/// make_view!(pub source, SourceConstrArgs {
///     handle_aspect_ratio: HandleAspectRatio
/// } [ source: Source ], TuiViewData::Source);
///
/// source!({
///   anchor_x: 0.5f32,
///   anchor_y: 0.5f32
/// }, {}, Source::File("/some/path.png"));
/// ```
pub macro make_view(
    $vis:vis $name:ident,
    $( $ConstrArgs:ident { $( $field_id:ident : $field_ty:ty ),* } )?
    $([ $( $required_field:ident : $required_field_ty:ty ),* ])?,
    $ViewData:ident :: $ViewDataEnum:ident
    $({ $( $field_id2:ident : $field_value:expr ),* })?
) {
    _make_view!(
        ($) @
        $vis $name,
        $( $ConstrArgs { $( $field_id : $field_ty ),* } )?
        $([ $( $required_field : $required_field_ty ),* ])?,
        $ViewData :: $ViewDataEnum
        $({ $( $field_id2 : $field_value ),* })?
    );
}

/// Create a less verbose constructor for a specific type of view
pub macro _make_view(
    ($d:tt) @
    $vis:vis $name:ident,
    $( $ConstrArgs:ident { $( $field_id:ident : $field_ty:ty ),* } )?
    $([ $( $required_field:ident : $required_field_ty:ty ),* ])?,
    $ViewData:ident :: $ViewDataEnum:ident
    $({ $( $field_id2:ident : $field_value:expr ),* })?
) {
    $(
        #[derive(Default)]
        $vis struct $ConstrArgs {
            $( pub $field_id : $field_ty ),*
        }
    )?

    $vis fn $name(
        view_args: VViewConstrArgs
        $( ,
            #[allow(unused_variables)]
            data_args: $ConstrArgs
        )?
        $( , $( $required_field : $required_field_ty ),* )?
    ) -> VNode<$ViewData> {
        constr_view(view_args, $ViewData::$ViewDataEnum {
            $( $( $field_id : data_args.$field_id, )* )?
            $( $( $required_field, )* )?
            $( $( $field_id2 : $field_value, )* )?
        })
    }

    $vis macro $name(
        { $d ( $d view_field:ident : $d view_value:expr ),* },
        $( $d ( ignore $ConstrArgs )? { $d ( $d data_field:ident : $d data_value:expr ),* } )?
        $( , $( $d $required_field:expr ),* )?
    ) {
        $name(
            VViewConstrArgs {
                $d ( $d view_field : $d view_value, )*
                ..VViewConstrArgs::default()
            }
            $( ,
                $ConstrArgs {
                    $d ( $d data_field: $d data_value, )*
                    ..$ConstrArgs::default()
                }
            )?
            $( , $( $d $required_field ),* )?
        )
    }
}