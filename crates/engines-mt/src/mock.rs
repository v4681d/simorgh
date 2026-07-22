//! Echo translator for tests/CI — wraps input so tests can assert routing.

use async_trait::async_trait;
use simorgh_core::traits::Translator;
use simorgh_core::types::LangPair;
use simorgh_core::Result;

pub struct MockTranslator;

#[async_trait]
impl Translator for MockTranslator {
    fn id(&self) -> &'static str { "mock" }
    fn display_name(&self) -> String { "Mock translator (testing)".into() }
    fn is_local(&self) -> bool { true }

    async fn translate(&self, text: &str, pair: &LangPair) -> Result<String> {
        Ok(format!("[{}→{}] {}", pair.source.0, pair.target.0, text))
    }
}
