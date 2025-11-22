use serde::Serialize;

#[derive(Serialize)]
pub(super) struct RunData {
    pub(super) label: String,
    pub(super) contents: String,
}
