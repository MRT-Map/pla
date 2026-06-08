mod component;
mod error;
mod namespace;
mod node;
mod node_type;
mod node_vec;
#[cfg(feature = "pla2")]
mod pla2;

pub use component::*;
pub use error::*;
pub use namespace::*;
pub use node::*;
pub use node_type::*;
pub use node_vec::*;
#[cfg(feature = "pla2")]
pub use pla2::*;

#[cfg(test)]
pub(crate) mod test {
    use proptest::prelude::*;

    use crate::{Namespace, PlaNode, PlaNodeVec};

    prop_compose! {
        pub fn vec2()(a in any::<f32>(), b in any::<f32>()) -> (f32, f32) {
            (a, b)
        }
    }

    #[cfg(feature = "emath")]
    prop_compose! {
        pub fn emath_vec2()(a in any::<f32>(), b in any::<f32>()) -> emath::Vec2 {
            emath::vec2(a, b)
        }
    }

    prop_compose! {
        pub fn line()(coord in vec2(), label in prop::option::of(any::<u8>())) -> PlaNode<(f32, f32)> {
            PlaNode::Line { coord, label }
        }
    }
    prop_compose! {
        pub fn quad()(ctrl in vec2(), coord in vec2(), label in prop::option::of(any::<u8>())) -> PlaNode<(f32, f32)> {
            PlaNode::QuadraticBezier { ctrl, coord, label }
        }
    }
    prop_compose! {
        pub fn cubic()(ctrl1 in vec2(), ctrl2 in vec2(), coord in vec2(), label in prop::option::of(any::<u8>())) -> PlaNode<(f32, f32)> {
            PlaNode::CubicBezier { ctrl1, ctrl2, coord, label }
        }
    }

    pub fn arb_nodes() -> impl Strategy<Value = PlaNodeVec<(f32, f32)>> {
        prop::collection::vec(prop_oneof![line(), quad(), cubic()], 0..10)
            .prop_map(|a| a.into_iter().collect())
    }

    pub fn arb_toml() -> impl Strategy<Value = toml::Value> {
        let leaf = prop_oneof![
            ".*".prop_map(toml::Value::String),
            any::<i64>().prop_map(toml::Value::Integer),
            // any::<f64>().prop_map(toml::Value::Float), issues with -9.051895622533191e-213 becoming -9.051895622533192e-213
            any::<bool>().prop_map(toml::Value::Boolean),
        ];
        leaf.prop_recursive(8, 256, 10, |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..10).prop_map(toml::Value::Array),
                prop::collection::hash_map(".*", inner, 0..10)
                    .prop_map(|a| toml::Value::Table(toml::Table::from_iter(a))),
            ]
        })
    }

    prop_compose! {
        pub fn arb_namespace()(value in "[A-Za-z0-9_]+") -> Namespace {
            Namespace::new(&value).unwrap()
        }
    }
}
