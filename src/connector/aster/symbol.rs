#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AsterSymbol(pub String);

impl AsterSymbol {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
