#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BinanceSymbol(pub String);

impl BinanceSymbol {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
