use emath031 as emath;

use crate::{PlaNodeTypeGet, PlaNodeTypeNew, PlaNodeTypeRect};

#[duplicate::duplicate_item(
    Type; [emath::Pos2]; [emath::Vec2]
)]
impl PlaNodeTypeNew for Type {
    type C = f32;
    fn new(x: Self::C, y: Self::C) -> Self {
        Self::from([x, y])
    }
}

#[duplicate::duplicate_item(
    Type; [emath::Pos2]; [emath::Vec2]
)]
impl PlaNodeTypeGet for Type {
    type C = f32;
    fn x(self) -> Self::C {
        self.x
    }
    fn y(self) -> Self::C {
        self.y
    }
}

impl PlaNodeTypeRect for emath::Vec2 {
    type Rect = emath::Rect;
    fn combine_rect(a: Self::Rect, b: Self::Rect) -> Self::Rect {
        a.union(b)
    }
    fn rect_from_point(self) -> Self::Rect {
        Self::Rect::from_pos(self.to_pos2())
    }
    fn rect_from_line(a: Self, b: Self) -> Self::Rect {
        Self::Rect::from_two_pos(a.to_pos2(), b.to_pos2())
    }
    fn rect_centre(rect: Self::Rect) -> Self {
        rect.center().to_vec2()
    }
}
impl PlaNodeTypeRect for emath::Pos2 {
    type Rect = emath::Rect;
    fn combine_rect(a: Self::Rect, b: Self::Rect) -> Self::Rect {
        a.union(b)
    }
    fn rect_from_point(self) -> Self::Rect {
        Self::Rect::from_pos(self)
    }
    fn rect_from_line(a: Self, b: Self) -> Self::Rect {
        Self::Rect::from_two_pos(a, b)
    }
    fn rect_centre(rect: Self::Rect) -> Self {
        rect.center()
    }
}
