//! Providence model
// pub use authbeam::model::Profile;

/// A 0 to 100 score resulting from a providence scan
#[derive(PartialEq, Eq, Debug)]
pub struct ProvidenceScore(u8);

impl Default for ProvidenceScore {
    fn default() -> Self {
        Self(100)
    }
}

impl Into<ProvidenceAction> for ProvidenceScore {
    /// Convert the score into the recommended action
    fn into(self) -> ProvidenceAction {
        if (self.0 < 100) && (self.0 >= 95) {
            return ProvidenceAction::Warn;
        } else if self.0 > 90 {
            return ProvidenceAction::Suspend;
        } else if self.0 < 90 {
            return ProvidenceAction::Terminate;
        }

        ProvidenceAction::Fine
    }
}

/// An action that needs to be taken as a result of a providence scan
#[derive(PartialEq, Eq, Debug)]
pub enum ProvidenceAction {
    /// Fails > 2 test(s), full deletion needed (with IP ban if applicable)
    Terminate,
    /// Fails 2 tests, suspension needed
    Suspend,
    /// Fails 1 test, warning needed
    Warn,
    /// Passes all tests, no action needed
    Fine,
}
