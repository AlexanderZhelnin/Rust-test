// use axum::Json;
use serde::{Deserialize, Serialize};
use serde_repr::*;
// use utoipa::ToSchema;
use std::sync::Arc;

use serde::ser::{SerializeSeq, Serializer};

use crate::arena::arena_slice::ArenaSlice;

/// Типы графических образов (GrTypeEnum)
#[derive(Debug, Clone, Copy, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum GrType {
    Empty = 0,
    Line = 1,
    Polygon = 2,
    Block = 3,
    Spline = 4,
    Bezier = 5,
    Ellipse = 6,
    Pie = 7,
    Path = 8,
    FillPath = 9,
    Text = 10,
    Visual = 12,
    Circle = 13,
}

/// Стиль границы (BorderStyleEnum)
#[derive(Debug, Clone, Copy, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum BorderStyle {
    Solid = 0,
    Dash = 1,
    Transparent = 2,
    Textured = 3,
}

/// Стиль заливки (FillStyleEnum)
#[derive(Debug, Clone, Copy, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum FillStyle {
    Solid = 1,
    Transparent = 2,
    LinearGradient = 3,
    Textured = 4,
    Hatch = 5,
}

/// Ориентация градиента (GradientStyle)
#[derive(Debug, Clone, Copy, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum GradientStyle {
    Left2Right,
    Right2Left,
    Bottom2Top,
    Top2Bottom,
    LeftBottom2RightTop,
    RightTop2LeftBottom,
    RightBottom2LeftTop,
    LeftTop2RightBottom,
}

/// Выравнивание текста (TextPositionEnum)
#[derive(Debug, Clone, Copy, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(i8)]
pub enum TextPosition {
    Default = 0,
    LeftBottom = 1,
    LeftMiddle = 2,
    LeftTop = 3,
    TopMiddle = 4,
    TopRight = 5,
    RightMiddle = 6,
    RightBottom = 7,
    BottomMiddle = 8,
    PositionSets = 9,
    LeftBottomInner = -1,
    LeftMiddleInner = -2,
    LeftTopInner = -3,
    TopMiddleInner = -4,
    TopRightInner = -5,
    RightMiddleInner = -6,
    RightBottomInner = -7,
    BottomMiddleInner = -8,
}

/// Стиль шрифта (FontStyleEnum)
#[derive(Debug, Clone, Copy, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum FontStyle {
    Regular = 0x0,
    Bold = 0x1,
    Italic = 0x2,
    Underline = 0x4,
    Strikeout = 0x8,
}

/// Диапазон масштаба
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MashtabRange {
    pub min: f64,
    pub max: f64,
}

impl Default for MashtabRange {
    fn default() -> Self {
        Self { min: 0.0, max: 0.0 }
    }
}

/// Блок легенды
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegendBlock {
    pub id: i64,
    pub size: f64,
    // pub scaled: bool,
}

impl Default for LegendBlock {
    fn default() -> Self {
        Self {
            id: 0,
            size: 0.0,
            // scaled: false,
        }
    }
}

/// Заливка легенды
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegendFill {
    pub color1: String,
    pub color2: String,
    pub scaled: bool,
    pub style: FillStyle,
    pub gradient_style: GradientStyle,
    pub block: LegendBlock,
}

impl Default for LegendFill {
    fn default() -> Self {
        Self {
            color1: String::new(),
            color2: String::new(),
            scaled: false,
            style: FillStyle::Solid,
            gradient_style: GradientStyle::Left2Right,
            block: LegendBlock::default(),
        }
    }
}

/// Граница легенды
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegendBorder {
    pub color: String,
    pub style: BorderStyle,
    pub scaled: bool,
    pub size: f64,
}

impl Default for LegendBorder {
    fn default() -> Self {
        Self {
            color: String::new(),
            style: BorderStyle::Solid,
            scaled: false,
            size: 0.0,
        }
    }
}

/// Шрифт легенды
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegendFont {
    pub family: String,
    pub size: f64,
    pub style: FontStyle,
}

impl Default for LegendFont {
    fn default() -> Self {
        Self {
            family: String::new(),
            size: 0.0,
            style: FontStyle::Regular,
        }
    }
}

/// Текст легенды
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegendText {
    pub mashtab_range: MashtabRange,
    pub mashtab_base: f64,
    pub scaled: bool,
    pub position: TextPosition,
    pub color: String,
    pub back_color: Option<String>,
    pub font: LegendFont,
    pub is_analyze: bool,
}

impl Default for LegendText {
    fn default() -> Self {
        Self {
            mashtab_range: MashtabRange::default(),
            mashtab_base: 0.0,
            scaled: false,
            position: TextPosition::Default,
            color: String::new(),
            back_color: None,
            font: LegendFont::default(),
            is_analyze: false,
        }
    }
}

/// Графический примитив
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Primitive {
    pub coords: Vec<f64>,
    pub text_coord_x: f64,
    pub text_coord_y: f64,
    pub text_angle: f64,
    pub rect: Rect,
    pub name: Arc<str>,
}

impl Default for Primitive {
    fn default() -> Self {
        Self {
            coords: Vec::new(),
            text_coord_x: 0.0,
            text_coord_y: 0.0,
            text_angle: 0.0,
            rect: Rect::default(),
            name: Arc::from(""),
        }
    }
}

/// Прямоугольник
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

impl Default for Rect {
    fn default() -> Self {
        Self {
            left: 0.0,
            right: 0.0,
            top: 0.0,
            bottom: 0.0,
        }
    }
}

/// Свойства отрисовки с координатами (DrawProperties1)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DrawProperties1 {
    pub left_top: [f64; 2],
    pub scale: f64,
    pub mashtab: f64,
}

impl Default for DrawProperties1 {
    fn default() -> Self {
        Self {
            left_top: [0.0, 0.0],
            scale: 0.0,
            mashtab: 0.0,
        }
    }
}

/// Графический образ (IObraz)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] // ToSchema
pub struct Obraz {
    pub name: Arc<str>,
    pub coords: Vec<f64>,
}

impl Default for Obraz {
    fn default() -> Self {
        Self {
            name: Arc::from(""),
            coords: Vec::new(),
        }
    }
}

/// Слой (ILayer)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)] // ToSchema
#[serde(rename_all = "camelCase")]
pub struct Layer {
    pub legend_id: i64,
    pub obrazes: Vec<Obraz>,
}

/// Легенда (ILegend)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Legend {
    pub id: i64,
    #[serde(rename = "type")]
    pub gr_type: GrType,
    pub mashtab_range: MashtabRange,
    pub priority: i32,
    pub block: LegendBlock,
    pub fill: LegendFill,
    pub border: LegendBorder,
    pub text: LegendText,
    pub primitives: Vec<Primitive>,
}

impl Default for Legend {
    fn default() -> Self {
        Self {
            id: 0,
            gr_type: GrType::Empty,
            mashtab_range: MashtabRange::default(),
            priority: 0,
            block: LegendBlock::default(),
            fill: LegendFill::default(),
            border: LegendBorder::default(),
            text: LegendText::default(),
            primitives: Vec::new(),
        }
    }
}

/// Результирующий слой (blazing, на арене)
#[derive(Debug, Clone, Serialize)]
// #[serde(rename_all = "camelCase")]
pub struct LayerResultBlazing {
    pub legend_id: i64,
    pub obrazes: ArenaSlice<ObrazResultBlazing>,
}

/// Данные для отображения (blazing, на арене) (аналог C# `ObrazResultBlazing`)
#[derive(Debug, Clone, Serialize)]
pub struct ObrazResultBlazing {
    pub name: Arc<str>,
    pub coords: ArenaSlice<f64>,
}

// impl Default for ObrazResultBlazing {
//     fn default() -> Self {
//         Self {
//             name: Arc::from(""),
//             // coords: ArenaSlice::new(),
//         }
//     }
// }

impl<T> Serialize for ArenaSlice<T>
where
    T: serde::ser::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;

        let array = self.as_slice();
        for element in array {
            seq.serialize_element(element)?;
        }
        seq.end()
    }
}
