use std::{
    collections::{BTreeMap, HashSet},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use itertools::Itertools;
use ordered_float::NotNan;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{
    Error, FullId, Namespace, PlaComponent, PlaNode, PlaNodeType, PlaNodeTypeBezier, Result,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pla2Component<T: PlaNodeType> {
    pub namespace: Namespace,
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub r#type: String,
    pub layer: NotNan<f32>,
    pub nodes: Vec<T>,
    pub tags: HashSet<String>,
    pub attrs: Option<BTreeMap<String, toml::Value>>,
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

    #[must_use]
    pub fn map_coords<U: PlaNodeType, F: Fn(T) -> U>(self, f: F) -> Pla2Component<U> {
        Pla2Component {
            namespace: self.namespace,
            id: self.id,
            display_name: self.display_name,
            description: self.description,
            r#type: self.r#type,
            layer: self.layer,
            nodes: self.nodes.into_iter().map(f).collect(),
            tags: self.tags,
            attrs: self.attrs,
        }
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
    pub namespace: Namespace,
    pub components: Vec<Pla2Component<T>>,
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

    #[must_use]
    pub fn map_coords<U: PlaNodeType, F: Fn(T) -> U>(self, f: F) -> Pla2File<U> {
        Pla2File {
            namespace: self.namespace,
            components: self
                .components
                .into_iter()
                .map(|a| a.map_coords(&f))
                .collect(),
        }
    }
}
impl<T: PlaNodeType + Serialize> Pla2File<T> {
    pub fn to_json_string(&self) -> serde_json::error::Result<String> {
        serde_json::to_string(&self.components)
    }
    pub fn to_json_bytes(&self) -> serde_json::error::Result<Vec<u8>> {
        serde_json::to_vec(&self.components)
    }
    pub fn to_json_writer<W: Write>(&self, writer: W) -> serde_json::error::Result<()> {
        serde_json::to_writer(writer, &self.components)
    }
    pub fn to_msgpack_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(&self.components)
    }
}
impl<'de, T: PlaNodeType + Deserialize<'de>> Pla2File<T> {
    fn from_file(components: Vec<Pla2Component<T>>, namespace: Option<Namespace>) -> Result<Self> {
        let namespaces = components
            .iter()
            .map(|a| &a.namespace)
            .unique()
            .collect::<HashSet<_>>();
        let namespace = match (namespaces.into_iter().at_most_one(), namespace) {
            (Ok(None), None) => Namespace::default(),
            (Ok(None), Some(n)) => n,
            (Ok(Some(n)), None) => n.clone(),
            (Ok(Some(n1)), Some(n2)) => {
                if *n1 == n2 {
                    n2
                } else {
                    return Err(Error::IncorrectNamespace(n1.clone(), n2));
                }
            }
            (Err(e), _) => return Err(Error::MultipleNamespaces(e.cloned().collect())),
        };
        Ok(Self {
            namespace,
            components,
        })
    }
    pub fn from_json_string(input: &'de str, namespace: Option<Namespace>) -> Result<Self> {
        Self::from_file(serde_json::from_str(input)?, namespace)
    }
    pub fn from_json_bytes(input: &'de [u8], namespace: Option<Namespace>) -> Result<Self> {
        Self::from_file(serde_json::from_slice(input)?, namespace)
    }
    pub fn from_msgpack_bytes(input: &'de [u8], namespace: Option<Namespace>) -> Result<Self> {
        Self::from_file(rmp_serde::from_slice(input)?, namespace)
    }
}
impl<T: PlaNodeType + DeserializeOwned> Pla2File<T> {
    pub fn from_msgpack_read<R: Read>(input: R, namespace: Option<Namespace>) -> Result<Self> {
        Self::from_file(rmp_serde::from_read(input)?, namespace)
    }
}

#[cfg(all(test, feature = "bezier-epaint"))]
mod test {
    use itertools::Itertools;
    use ordered_float::NotNan;
    use proptest::prelude::*;

    use crate::{
        Error, Namespace, Pla2Component, Pla2File,
        test::{arb_namespace, arb_toml, emath_vec2},
    };

    prop_compose! {
        fn arb_pla2(namespace_strategy: impl Strategy<Value = Namespace>)(
            namespace in namespace_strategy,
            id in ".*",
            display_name in ".*",
            description in ".*",
            r#type in ".*",
            layer in any::<f32>().prop_filter_map("not nan", |a| NotNan::new(a).ok()),
            nodes in prop::collection::vec(emath_vec2(), 0..10),
            tags in prop::collection::hash_set(".*", 1..10),
            attrs in prop::option::of(prop::collection::btree_map(".*", arb_toml(), 1..10)),
        ) -> Result<Pla2Component<emath::Vec2>, TestCaseError> {
            prop_assume!(attrs.as_ref().is_none_or(|attrs| !attrs.values().any(|v| *v == toml::Value::Boolean(true))));
            Ok(Pla2Component::<emath::Vec2> {
                namespace,
                id,
                display_name,
                description,
                r#type,
                layer,
                nodes: nodes.into_iter().dedup().collect(),
                tags,
                attrs,
            })
        }
    }
    prop_compose! {
        fn arb_pla2_file()(
            namespace in arb_namespace()
        )(
            namespace in Just(namespace.clone()),
            components in prop::collection::vec(arb_pla2(Just(namespace)), 1..10)
        ) -> Result<Pla2File<emath::Vec2>, TestCaseError> {
            let components = components.into_iter().collect::<Result<Vec<_>, TestCaseError>>()?;
            Ok(Pla2File {
                namespace, components
            })
        }
    }

    proptest! {
        #[test]
        fn test_pla2to3to2(
            pla2 in arb_pla2(arb_namespace()),
        ) {
            let pla2 = pla2?;
            let key_already_exists_error_expected = pla2.attrs.as_ref().is_some_and(|attrs| attrs.keys().any(|k| pla2.tags.contains(k)));
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

    proptest! {
        #![proptest_config(ProptestConfig {
            max_shrink_iters: 2048,
            ..ProptestConfig::default()
        })]

        #[test]
        fn test_pla2_json(
            pla2_file in arb_pla2_file(),
        ) {
            let pla2_file = pla2_file?.map_coords(|a| (a.x, a.y));
            let json = pla2_file.to_json_bytes()?;
            let result = Pla2File::from_json_bytes(&json, None)?;
            prop_assert_eq!(pla2_file, result);
        }

        #[test]
        fn test_pla2_msgpack(
            pla2_file in arb_pla2_file(),
        ) {
            let pla2_file = pla2_file?.map_coords(|a| (a.x, a.y));
            let json = pla2_file.to_msgpack_bytes()?;
            let result = Pla2File::from_msgpack_bytes(&json, None)?;
            prop_assert_eq!(pla2_file, result);
        }
    }
}
