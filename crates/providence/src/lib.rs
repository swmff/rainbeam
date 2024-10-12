//! Providence engine
mod model;

pub struct Providence {}

impl Providence {
    /// Create a new instance of [`Providence`]
    pub fn new() -> Self {
        Self {}
    }

    pub fn score(_content: String) -> model::ProvidenceScore {
        model::ProvidenceScore::default()
    }
}
