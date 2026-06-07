use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter, Write},
    io::{BufRead, Cursor},
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use ordered_float::NotNan;

use crate::{
    error::{Error, InvalidLabelError, InvalidLayerError, Result},
    node::PlaNode,
    node_type::{PlaNodeType, PlaNodeTypeGet, PlaNodeTypeNew},
    node_vec::PlaNodeVec,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FullId {
    pub namespace: String,
    pub id: String,
}

impl Display for FullId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.namespace, self.id)?;
        Ok(())
    }
}

impl FullId {
    #[must_use]
    pub const fn new(namespace: String, id: String) -> Self {
        Self { namespace, id }
    }
}

#[derive(Debug, PartialEq)]
pub struct PlaComponent<S: ?Sized, T: PlaNodeType> {
    pub full_id: FullId,
    pub ty: Arc<S>,
    pub display_name: String,
    pub layer: NotNan<f32>,
    pub nodes: PlaNodeVec<T>,
    pub misc: BTreeMap<String, toml::Value>,
}

impl<S: ?Sized, T: PlaNodeType> Clone for PlaComponent<S, T> {
    fn clone(&self) -> Self {
        Self {
            full_id: self.full_id.clone(),
            ty: Arc::clone(&self.ty),
            display_name: self.display_name.clone(),
            layer: self.layer,
            nodes: self.nodes.clone(),
            misc: self.misc.clone(),
        }
    }
}

impl<S: ?Sized, T: PlaNodeType> Display for PlaComponent<S, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.full_id)?;
        if !self.display_name.is_empty() {
            write!(f, " ({})", self.display_name)?;
        }
        Ok(())
    }
}

impl<S: ?Sized, T: PlaNodeType> PlaComponent<S, T> {
    #[must_use]
    pub fn file_name(&self) -> String {
        format!("{}.pla3", self.full_id.id)
    }
    #[must_use]
    pub fn path(&self, root: &Path) -> PathBuf {
        root.join(&*self.full_id.namespace).join(self.file_name())
    }
}

impl<S: ?Sized, T: PlaNodeTypeNew> PlaComponent<S, T>
where
    <T::C as FromStr>::Err: 'static,
{
    fn get_coord(split: &[&str], i: usize) -> Result<T> {
        let (x, y) = (split[i], split[i + 1]);
        let (x, y) = (
            x.parse()
                .map_err(|e| Error::InvalidCoordinate(x.to_owned(), Box::new(e)))?,
            y.parse()
                .map_err(|e| Error::InvalidCoordinate(y.to_owned(), Box::new(e)))?,
        );
        Ok(PlaNodeTypeNew::new(x, y))
    }
    fn get_label(split: &[&str], i: usize) -> Result<Option<u8>> {
        let Some(label) = split.get(i) else {
            return Ok(None);
        };
        let Some(label) = label.strip_prefix("#") else {
            return Err(Error::InvalidLabel(
                label.to_string(),
                InvalidLabelError::MissingPrefix,
            ));
        };
        label
            .parse::<u8>()
            .map_err(|e| Error::InvalidLabel(label.to_owned(), e.into()))
            .map(Some)
    }
    pub fn load<STR, GT: Fn(&str) -> Option<Arc<S>>>(
        input: STR,
        full_id: FullId,
        get_type: GT,
    ) -> Result<(Self, Option<Error>)>
    where
        Cursor<STR>: BufRead,
    {
        Self::load_from_buf(Cursor::new(input), full_id, get_type)
    }
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(full_id)))]
    pub fn load_from_buf<B: BufRead, GT: Fn(&str) -> Option<Arc<S>>>(
        buf: B,
        full_id: FullId,
        get_type: GT,
    ) -> Result<(Self, Option<Error>)> {
        let mut unknown_type_error = None;
        let mut lines = buf.lines();

        let mut nodes = PlaNodeVec::new();
        while let Some(next) = lines.next().transpose()?
            && next != "---"
        {
            if next.is_empty() {
                continue;
            }
            let split = next.split(' ').collect::<Vec<_>>();
            let node = match split.len() {
                2 | 3 => PlaNode::Line {
                    coord: Self::get_coord(&split, 0)?,
                    label: Self::get_label(&split, 2)?,
                },
                4 | 5 => PlaNode::QuadraticBezier {
                    ctrl: Self::get_coord(&split, 0)?,
                    coord: Self::get_coord(&split, 2)?,
                    label: Self::get_label(&split, 4)?,
                },
                6 | 7 => PlaNode::CubicBezier {
                    ctrl1: Self::get_coord(&split, 0)?,
                    ctrl2: Self::get_coord(&split, 2)?,
                    coord: Self::get_coord(&split, 4)?,
                    label: Self::get_label(&split, 6)?,
                },
                len => return Err(Error::InvalidSplitLength(next.clone(), len)),
            };

            if nodes.is_empty()
                && matches!(
                    node,
                    PlaNode::QuadraticBezier { .. } | PlaNode::CubicBezier { .. }
                )
            {
                return Err(Error::FirstNodeIsCurve(format!("{node:?}")));
            }

            nodes.push(node);
        }

        let mut display_name = String::new();
        let mut layer = NotNan::<f32>::default();
        let mut ty = if nodes.len() == 1 {
            get_type("simplePoint").ok_or_else(|| Error::MissingType("simplePoint".into()))
        } else {
            get_type("simpleLine").ok_or_else(|| Error::MissingType("simpleLine".into()))
        };
        let mut misc = BTreeMap::<String, toml::Value>::new();

        let toml_str = lines.try_fold(String::new(), |a, b| {
            Result::<_, std::io::Error>::Ok(a + "\n" + &b?)
        })?;
        for (k, v) in toml::from_str::<toml::Table>(&toml_str)? {
            match &*k {
                "display_name" => {
                    v.as_str()
                        .ok_or_else(|| Error::InvalidDisplayName(v.clone()))?
                        .clone_into(&mut display_name);
                }
                "layer" => {
                    let float = if let Some(f) = v.as_float() {
                        f as f32
                    } else if let Some(i) = v.as_integer() {
                        i as f32
                    } else {
                        return Err(Error::InvalidLayer(
                            v,
                            InvalidLayerError::NeitherIntegerNorFloat,
                        ));
                    };
                    layer = NotNan::new(float)
                        .map_err(|e| Error::InvalidLayer(v, InvalidLayerError::IsNaN(e)))?;
                }
                "type" => {
                    let ty_str = v
                        .as_str()
                        .ok_or_else(|| Error::InvalidSkinType(v.clone()))?;
                    if let Some(s) = get_type(ty_str) {
                        ty = Ok(s);
                    } else {
                        unknown_type_error =
                            Some(Error::UnknownType(full_id.clone(), ty_str.into()));
                    }
                }
                _ => {
                    misc.insert(k, v);
                }
            }
        }

        Ok((
            Self {
                full_id,
                ty: ty?,
                display_name,
                layer,
                nodes,
                misc,
            },
            unknown_type_error,
        ))
    }
}

impl<S: ?Sized, T: PlaNodeTypeGet> PlaComponent<S, T> {
    pub fn save_to_string<'a, TS: Fn(&'a S) -> V, V: Into<toml::Value> + 'a>(
        &'a self,
        format_ty: TS,
    ) -> Result<String>
    where
        S: 'a,
    {
        let mut out = String::new();
        self.save_to_writer(&mut out, format_ty)?;
        Ok(out)
    }
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(self.full_id)))]
    pub fn save_to_writer<'a, W: Write, TS: Fn(&'a S) -> V, V: Into<toml::Value> + 'a>(
        &'a self,
        mut writer: W,
        format_ty: TS,
    ) -> Result<W>
    where
        S: 'a,
    {
        for node in &self.nodes {
            match node {
                PlaNode::Line { coord, .. } => write!(writer, "{} {}", coord.x(), coord.y())?,
                PlaNode::QuadraticBezier { ctrl, coord, .. } => {
                    write!(
                        writer,
                        "{} {} {} {}",
                        ctrl.x(),
                        ctrl.y(),
                        coord.x(),
                        coord.y()
                    )?;
                }
                PlaNode::CubicBezier {
                    ctrl1,
                    ctrl2,
                    coord,
                    ..
                } => write!(
                    writer,
                    "{} {} {} {} {} {}",
                    ctrl1.x(),
                    ctrl1.y(),
                    ctrl2.x(),
                    ctrl2.y(),
                    coord.x(),
                    coord.y()
                )?,
            }
            if let Some(label) = node.label() {
                writeln!(writer, " #{label}")?;
            } else {
                writer.write_str("\n")?;
            }
        }

        writer.write_str("---\n")?;

        let attrs = self
            .misc
            .clone()
            .into_iter()
            .chain([
                ("display_name".into(), self.display_name.clone().into()),
                ("layer".into(), (*self.layer).into()),
                ("type".into(), format_ty(&self.ty).into()),
            ])
            .collect::<toml::Table>();
        writer.write_str(&toml::to_string_pretty(&attrs)?)?;

        Ok(writer)
    }
}

#[cfg(test)]
mod test {
    use std::{assert_matches, sync::Arc};

    use ordered_float::NotNan;
    use proptest::prelude::*;

    use crate::{
        Error, FullId, InvalidLabelError, InvalidLayerError, PlaComponent, PlaNode,
        test::{arb_nodes, arb_toml},
    };

    proptest! {
        #[test]
        fn test_loading_no_crash(s in ".*", namespace in ".*", id in ".*") {
            let _ = PlaComponent::<str, (f32, f32)>::load(&s, FullId::new(namespace, id), |t| Some(t.into()));
        }
    }

    proptest! {
        #[test]
        fn test_save_load(
            namespace in ".*",
            id in ".*",
            ty in ".*",
            display_name in ".*",
            layer in any::<f32>().prop_filter_map("not nan", |a| NotNan::new(a).ok()),
            nodes in arb_nodes(),
            misc in prop::collection::btree_map(".*", arb_toml(), 0..10),
        ) {
            prop_assume!(nodes.first().is_none_or(|n| matches!(n, PlaNode::Line { .. })));
            let full_id = FullId::new(namespace, id);
            let pla3 = PlaComponent {
                full_id: full_id.clone(),
                ty: Arc::new(ty),
                display_name,
                layer,
                nodes,
                misc,
            };
            let string = pla3.save_to_string(Clone::clone).unwrap();
            let (result, _) = PlaComponent::load(&string, full_id, |a| Some(Arc::new(a.to_owned()))).unwrap();
            prop_assert_eq!(pla3, result);
        }
    }

    fn load_expect_error(string: &str) -> Error {
        PlaComponent::<String, (f32, f32)>::load(
            string,
            FullId::new(String::new(), String::new()),
            |a| Some(Arc::new(a.to_owned())),
        )
        .unwrap_err()
    }

    #[test]
    fn test_invalid_label_missing_prefix() {
        let string = "0 0 abc";
        let result = load_expect_error(string);
        assert_matches!(
            result,
            Error::InvalidLabel(_, InvalidLabelError::MissingPrefix)
        );
    }
    #[test]
    fn test_invalid_label_invalid_number() {
        let string = "0 0 #abc";
        let result = load_expect_error(string);
        assert_matches!(
            result,
            Error::InvalidLabel(_, InvalidLabelError::InvalidNumber(_))
        );
    }
    #[test]
    fn test_invalid_split_length_1() {
        let string = "0";
        let result = load_expect_error(string);
        assert_matches!(result, Error::InvalidSplitLength(_, 1));
    }
    #[test]
    fn test_invalid_split_length_8() {
        let string = "0 0 0 0 0 0 0 0";
        let result = load_expect_error(string);
        assert_matches!(result, Error::InvalidSplitLength(_, 8));
    }
    #[test]
    fn test_invalid_coordinate() {
        let string = "0 abc";
        let result = load_expect_error(string);
        assert_matches!(result, Error::InvalidCoordinate(_, _));
    }
    #[test]
    fn test_first_node_is_curve_quad() {
        let string = "0 0 0 0";
        let result = load_expect_error(string);
        assert_matches!(result, Error::FirstNodeIsCurve(_));
    }
    #[test]
    fn test_first_node_is_curve_cubic() {
        let string = "0 0 0 0 0 0";
        let result = load_expect_error(string);
        assert_matches!(result, Error::FirstNodeIsCurve(_));
    }
    #[test]
    fn test_invalid_display_name() {
        let string = "\n---\ndisplay_name = true";
        let result = load_expect_error(string);
        assert_matches!(result, Error::InvalidDisplayName(_));
    }
    #[test]
    fn test_invalid_layer_neither_integer_nor_float() {
        let string = "\n---\nlayer = true";
        let result = load_expect_error(string);
        assert_matches!(
            result,
            Error::InvalidLayer(_, InvalidLayerError::NeitherIntegerNorFloat)
        );
    }
    #[test]
    fn test_invalid_layer_is_nan() {
        let string = "\n---\nlayer = nan";
        let result = load_expect_error(string);
        assert_matches!(result, Error::InvalidLayer(_, InvalidLayerError::IsNaN(_)));
    }
    #[test]
    fn test_invalid_skin_type() {
        let string = "\n---\ntype = true";
        let result = load_expect_error(string);
        assert_matches!(result, Error::InvalidSkinType(_));
    }
    #[test]
    fn test_invalid_metadata() {
        let string = "\n---\nabc =";
        let result = load_expect_error(string);
        assert_matches!(result, Error::TOMLDeserialisation(_));
    }
}
