#[derive(Debug)]
pub(crate) struct TLError {
    details: String
}
impl TLError {
    pub(crate) fn new(msg: &str) -> TLError {
        TLError{ details: msg.to_string() }
    }
}
impl std::fmt::Display for TLError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f,"{}",self.details)
    }
}
impl std::error::Error for TLError {
    fn description(&self) -> &str {
        &self.details
    }
}
