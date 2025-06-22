use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod api_media;
pub use api_media::*;
pub mod api_creator;
pub use api_creator::*;
pub mod api_collection;
pub use api_collection::*;

pub mod pagination;
pub use pagination::*;

pub mod api_search_query;
pub use api_search_query::*;

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct DataVector(pub Vec<String>);
impl Default for DataVector {
    fn default() -> Self {
        DataVector(Vec::new())
    }
}

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct DataVectorI32(pub Vec<i32>);
impl Default for DataVectorI32 {
    fn default() -> Self {
        Self(Vec::new())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DataMap(pub BTreeMap<String, Vec<String>>);

impl Default for DataMap {
    fn default() -> Self {
        DataMap(BTreeMap::default())
    }
}

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct DataVectorI64(pub Vec<i64>);
impl Default for DataVectorI64 {
    fn default() -> Self {
        Self(Vec::new())
    }
}

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct DataVectorI64String(pub Vec<(i64, String)>);
impl Default for DataVectorI64String {
    fn default() -> Self {
        Self(Vec::new())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DataMapI64String(pub BTreeMap<i64, String>);

impl Default for DataMapI64String {
    fn default() -> Self {
        DataMapI64String(BTreeMap::default())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DataMapI64I64(pub BTreeMap<i64, i64>);

impl Default for DataMapI64I64 {
    fn default() -> Self {
        DataMapI64I64(BTreeMap::default())
    }
}