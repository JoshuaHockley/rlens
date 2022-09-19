use euclid::default::{
    Box2D, Point2D, SideOffsets2D, Size2D, Transform2D, Translation2D, Vector2D,
};

pub type Point = Point2D<f32>;
pub type Rect = Box2D<f32>;
pub type Size = Size2D<f32>;
pub type IntSize = Size2D<u32>;
pub type Transform = Transform2D<f32>;
pub type Translation = Translation2D<f32>;
pub type Vector = Vector2D<f32>;
pub type SideOffsets = SideOffsets2D<f32>;
pub type Angle = euclid::Angle<f32>;
