use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

use crate::{Error, FullId, PlaComponent, PlaNode, PlaNodeType, PlaNodeTypeBezier, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pla2Component<T: PlaNodeType> {
    namespace: String,
    id: String,
    display_name: String,
    description: String,
    r#type: String,
    layer: NotNan<f32>,
    nodes: Vec<T>,
    tags: HashSet<String>,
    attrs: Option<BTreeMap<String, toml::Value>>,
}

impl<T: PlaNodeType> Pla2Component<T> {
    pub fn to_pla3<S: ?Sized, GT: Fn(&str) -> Option<Arc<S>>>(
        self,
        get_type: GT,
    ) -> Result<PlaComponent<S, T>> {
        Ok(PlaComponent {
            full_id: FullId::new(self.namespace, self.id),
            ty: if let Some(ty) = get_type(&self.r#type) {
                ty
            } else if self.nodes.len() == 1 {
                get_type("simplePoint").ok_or_else(|| Error::MissingType("simplePoint".into()))?
            } else {
                get_type("simpleLine").ok_or_else(|| Error::MissingType("simpleLine".into()))?
            },
            display_name: self.display_name,
            layer: self.layer,
            nodes: self
                .nodes
                .into_iter()
                .map(|n| PlaNode::Line {
                    coord: n,
                    label: None,
                })
                .collect(),
            misc: {
                let mut misc = self.attrs.unwrap_or_default();
                if !self.description.is_empty() {
                    misc.insert("description".into(), self.description.into());
                }
                for tag in self.tags {
                    if misc.contains_key(&tag) {
                        return Err(Error::KeyAlreadyExistsForTag(tag));
                    }
                    misc.insert(tag, true.into());
                }
                misc
            },
        })
    }
    pub fn as_pla3<S: ?Sized, GT: Fn(&str) -> Option<Arc<S>>>(
        &self,
        get_type: GT,
    ) -> Result<PlaComponent<S, T>> {
        self.clone().to_pla3(get_type)
    }
}
impl<S: ?Sized, T: PlaNodeTypeBezier> PlaComponent<S, T> {
    pub fn to_pla2<TS: Fn(&S) -> V, V: Into<String>, Tolerance: Into<Option<f32>> + Copy>(
        mut self,
        format_ty: TS,
        tolerance: Tolerance,
    ) -> Pla2Component<T> {
        Pla2Component {
            namespace: self.full_id.namespace,
            id: self.full_id.id,
            display_name: self.display_name,
            description: self
                .misc
                .remove("description")
                .map_or_else(String::new, |description| {
                    description
                        .as_str()
                        .map_or_else(|| description.to_string(), ToOwned::to_owned)
                }),
            r#type: format_ty(&*self.ty).into(),
            layer: self.layer,
            nodes: self.nodes.outline(tolerance),
            tags: {
                let mut tags = HashSet::new();
                self.misc.retain(|k, v| {
                    if v.as_bool() != Some(true) {
                        return true;
                    }
                    tags.insert(k.to_owned());
                    false
                });
                tags
            },
            attrs: if self.misc.is_empty() {
                None
            } else {
                Some(self.misc)
            },
        }
    }
    pub fn as_pla2<TS: Fn(&S) -> V, V: Into<String>, Tolerance: Into<Option<f32>> + Copy>(
        &self,
        format_ty: TS,
        tolerance: Tolerance,
    ) -> Pla2Component<T> {
        self.clone().to_pla2(format_ty, tolerance)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pla2File<T: PlaNodeType> {
    namespace: String,
    components: Vec<Pla2Component<T>>,
}

impl<T: PlaNodeType> Pla2File<T> {
    #[must_use]
    pub fn json_path(&self, root: &Path) -> PathBuf {
        root.join(self.json_file_name())
    }
    #[must_use]
    pub fn json_file_name(&self) -> String {
        format!("{}.pla2.json", self.namespace)
    }

    #[must_use]
    pub fn msgpack_path(&self, root: &Path) -> PathBuf {
        root.join(self.msgpack_file_name())
    }
    #[must_use]
    pub fn msgpack_file_name(&self) -> String {
        format!("{}.pla2.msgpack", self.namespace)
    }
}
impl<T: PlaNodeType + Serialize> Pla2File<T> {
    pub fn as_json_string(&self) -> serde_json::error::Result<String> {
        serde_json::to_string(self)
    }
    pub fn as_json_bytes(&self) -> serde_json::error::Result<Vec<u8>> {
        serde_json::to_vec(self)
    }
    pub fn as_msgpack(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }
}
impl<'de, T: PlaNodeType + Deserialize<'de>> Pla2File<T> {
    pub fn from_json_string(input: &'de str) -> serde_json::error::Result<Self> {
        serde_json::from_str(input)
    }
    pub fn from_json_bytes(input: &'de [u8]) -> serde_json::error::Result<Self> {
        serde_json::from_slice(input)
    }
    pub fn from_msgpack(input: &'de [u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(input)
    }
}

#[cfg(test)]
mod test {
    use itertools::Itertools;
    use ordered_float::NotNan;
    use proptest::prelude::*;

    use crate::{
        Error, Pla2Component,
        test::{arb_toml, egui_vec2},
    };

    proptest! {
        #[test]
        fn test_pla2to3to2(
            namespace in ".*",
            id in ".*",
            display_name in ".*",
            description in ".*",
            r#type in ".*",
            layer in any::<f32>().prop_filter_map("not nan", |a| NotNan::new(a).ok()),
            nodes in prop::collection::vec(egui_vec2(), 0..=100),
            tags in prop::collection::hash_set(".*", 1..=100),
            attrs in prop::option::of(prop::collection::btree_map(".*", arb_toml(), 1..10)),
        ) {
            prop_assume!(attrs.as_ref().is_none_or(|attrs| !attrs.values().any(|v| *v == toml::Value::Boolean(true))));
            let key_already_exists_error_expected = attrs.as_ref().is_some_and(|attrs| attrs.keys().any(|k| tags.contains(k)));
            let pla2 = Pla2Component::<egui::Vec2> {
                namespace,
                id,
                display_name,
                description,
                r#type,
                layer,
                nodes: nodes.into_iter().dedup().collect(),
                tags,
                attrs,
            };
            let pla3 = match pla2.as_pla3::<str, _>(|a| Some(a.into())) {
                Ok(a) => {
                    prop_assert!(!key_already_exists_error_expected);
                    a
                }
                Err(e) => {
                    prop_assert!(key_already_exists_error_expected);
                    prop_assert!(matches!(e, Error::KeyAlreadyExistsForTag(_)));
                    return Ok(());
                }
            };

            let result = pla3.to_pla2(str::to_owned, None);
            prop_assert_eq!(pla2, result);
        }
    }
}
