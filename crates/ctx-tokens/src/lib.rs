use std::sync::Arc;
use tiktoken_rs::CoreBPE;

/// Simple token estimator using tiktoken (cl100k_base encoding)
pub struct TokenEstimator {
    bpe: Arc<CoreBPE>,
}

impl TokenEstimator {
    /// Create new estimator with cl100k_base encoding (GPT-4, GPT-3.5-turbo)
    pub fn new() -> Self {
        Self {
            bpe: Arc::new(tiktoken_rs::cl100k_base().expect("Failed to load tiktoken encoding")),
        }
    }

    /// Estimate token count for a single string
    pub fn estimate(&self, text: &str) -> usize {
        self.bpe.encode_ordinary(text).len()
    }

    /// Estimate tokens for multiple strings (batch processing)
    pub fn estimate_batch(&self, texts: &[&str]) -> Vec<usize> {
        texts.iter().map(|text| self.estimate(text)).collect()
    }
}

impl Default for TokenEstimator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_estimation() {
        let estimator = TokenEstimator::new();

        // Simple text
        let count = estimator.estimate("Hello, world!");
        assert!(count > 0 && count < 10);

        // Empty string
        assert_eq!(estimator.estimate(""), 0);
    }

    #[test]
    fn test_batch_estimation() {
        let estimator = TokenEstimator::new();

        let texts = vec!["Hello", "world", "!"];
        let counts = estimator.estimate_batch(&texts);

        assert_eq!(counts.len(), 3);
        assert!(counts.iter().all(|&c| c > 0));
    }
}
