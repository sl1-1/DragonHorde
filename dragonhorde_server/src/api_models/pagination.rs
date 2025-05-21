use serde::Deserialize;
use utoipa::IntoParams;

#[derive(IntoParams, Deserialize)]
pub struct Pagination {
    /// Number of Results per page
    pub(crate) per_page: Option<u64>,
    /// Last object of previous results, provide to get next results
    pub(crate) last: Option<u64>,
}