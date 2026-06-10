#[cfg(feature = "pla2")]
use std::collections::HashSet;
use std::num::ParseIntError;

#[cfg(feature = "pla2")]
use itertools::Itertools;
use ordered_float::FloatIsNan;
use thiserror::Error;

use crate::FullId;
#[cfg(feature = "pla2")]
use crate::Namespace;

#[derive(Error, Debug)]
pub enum InvalidLabelError {
    #[error("Does not start with #")]
    MissingPrefix,
    #[error("Invalid number")]
    InvalidNumber(#[from] ParseIntError),
}

#[derive(Error, Debug)]
pub enum InvalidLayerError {
    #[error("Neither integer nor float")]
    NeitherIntegerNorFloat,
    #[error("Is NaN: {0}")]
    IsNaN(#[cfg_attr(feature = "std", from)] FloatIsNan),
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid label `{0}`")]
    InvalidLabel(String, #[source] InvalidLabelError),
    #[error("`{0}` has invalid split length {1}")]
    InvalidSplitLength(String, usize),
    #[error("Invalid coordinate {0}: {1}")]
    InvalidCoordinate(
        String,
        #[source] Box<dyn std::error::Error + Send + Sync + 'static>,
    ),
    #[error("First node must exist and not be a curve (got {0})")]
    FirstNodeIsCurve(String),
    #[error("No type `{0}`")]
    MissingType(String),
    #[error("Invalid display name, must be string (got {0})")]
    InvalidDisplayName(toml::Value),
    #[error("Invalid layer, must be non-NaN number (got {0})")]
    InvalidLayer(toml::Value, #[source] InvalidLayerError),
    #[error("Invalid skin type, must be string (got {0})")]
    InvalidSkinType(toml::Value),
    #[error("Unknown skin type for component {0}: {1}")]
    UnknownType(FullId, String),
    #[error("Invalid namespace {0}")]
    InvalidNamespace(String),
    #[error("TOML serialisation error: {0}")]
    TOMLSerialisation(#[from] toml::ser::Error),
    #[error("TOML deserialisation error: {0}")]
    TOMLDeserialisation(#[from] toml::de::Error),
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Writing error: {0}")]
    Writing(#[from] std::fmt::Error),

    #[cfg(feature = "pla2")]
    #[error("Cannot write tag {0} into `misc` field as key already exists")]
    KeyAlreadyExistsForTag(String),
    #[cfg(feature = "pla2")]
    #[error("Components of multiple namespaces in single PLA2 file (Got {})", .0.iter().map(ToString::to_string).join(", "))]
    MultipleNamespaces(HashSet<Namespace>),
    #[cfg(feature = "pla2")]
    #[error("Namespace `{0}` found in PLA2 file of namespace `{1}`")]
    IncorrectNamespace(Namespace, Namespace),
    #[cfg(feature = "pla2")]
    #[error("JSON deserialisation error: {0}")]
    JSON(#[from] serde_json::Error),
    #[cfg(feature = "pla2")]
    #[error("MessagePack deserialisation error: {0}")]
    MessagePackDecode(#[from] rmp_serde::decode::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
