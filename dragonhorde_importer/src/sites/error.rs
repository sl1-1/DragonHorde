use std::fmt;

#[derive(Debug, Clone)]
pub(crate) struct NotFound;

// Generation of an error is completely separate from how it is displayed.
// There's no need to be concerned about cluttering complex logic with the display style.
//
// Note that we don't store any extra info about the errors. This means we can't state
// which string failed to parse without modifying our types to carry that information.
impl fmt::Display for NotFound {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Submission Not Found")
    }
}

impl std::error::Error for NotFound {
    fn description(&self) -> &str {"Submission not found"}
}
