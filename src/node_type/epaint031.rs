use epaint::emath;
use epaint031 as epaint;

use crate::{PlaNodeTypeBezier, PlaNodeTypeBezierRect};

impl PlaNodeTypeBezier for emath::Vec2 {
    fn flatten_quadratic(
        a: Self,
        b: Self,
        c: Self,
        tolerance: impl Into<Option<f32>>,
    ) -> Vec<Self> {
        epaint::QuadraticBezierShape::from_points_stroke(
            [a.to_pos2(), b.to_pos2(), c.to_pos2()],
            false,
            epaint::Color32::default(),
            epaint::Stroke::default(),
        )
        .flatten(tolerance.into())
        .into_iter()
        .map(emath::Pos2::to_vec2)
        .collect()
    }

    fn flatten_cubic(
        a: Self,
        b: Self,
        c: Self,
        d: Self,
        tolerance: impl Into<Option<f32>>,
    ) -> Vec<Self> {
        epaint::CubicBezierShape::from_points_stroke(
            [a.to_pos2(), b.to_pos2(), c.to_pos2(), d.to_pos2()],
            false,
            epaint::Color32::default(),
            epaint::Stroke::default(),
        )
        .flatten(tolerance.into())
        .into_iter()
        .map(emath::Pos2::to_vec2)
        .collect()
    }
}

impl PlaNodeTypeBezier for emath::Pos2 {
    fn flatten_quadratic(
        a: Self,
        b: Self,
        c: Self,
        tolerance: impl Into<Option<f32>>,
    ) -> Vec<Self> {
        epaint::QuadraticBezierShape::from_points_stroke(
            [a, b, c],
            false,
            epaint::Color32::default(),
            epaint::Stroke::default(),
        )
        .flatten(tolerance.into())
    }

    fn flatten_cubic(
        a: Self,
        b: Self,
        c: Self,
        d: Self,
        tolerance: impl Into<Option<f32>>,
    ) -> Vec<Self> {
        epaint::CubicBezierShape::from_points_stroke(
            [a, b, c, d],
            false,
            epaint::Color32::default(),
            epaint::Stroke::default(),
        )
        .flatten(tolerance.into())
    }
}

impl PlaNodeTypeBezierRect for emath::Vec2 {
    fn rect_from_quadratic(a: Self, b: Self, c: Self) -> Self::Rect {
        epaint::QuadraticBezierShape::from_points_stroke(
            [a.to_pos2(), b.to_pos2(), c.to_pos2()],
            false,
            epaint::Color32::default(),
            epaint::Stroke::default(),
        )
        .logical_bounding_rect()
    }
    fn rect_from_cubic(a: Self, b: Self, c: Self, d: Self) -> Self::Rect {
        epaint::CubicBezierShape::from_points_stroke(
            [a.to_pos2(), b.to_pos2(), c.to_pos2(), d.to_pos2()],
            false,
            epaint::Color32::default(),
            epaint::Stroke::default(),
        )
        .logical_bounding_rect()
    }
}

impl PlaNodeTypeBezierRect for emath::Pos2 {
    fn rect_from_quadratic(a: Self, b: Self, c: Self) -> Self::Rect {
        epaint::QuadraticBezierShape::from_points_stroke(
            [a, b, c],
            false,
            epaint::Color32::default(),
            epaint::Stroke::default(),
        )
        .logical_bounding_rect()
    }
    fn rect_from_cubic(a: Self, b: Self, c: Self, d: Self) -> Self::Rect {
        epaint::CubicBezierShape::from_points_stroke(
            [a, b, c, d],
            false,
            epaint::Color32::default(),
            epaint::Stroke::default(),
        )
        .logical_bounding_rect()
    }
}
