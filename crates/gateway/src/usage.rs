use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct Usage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

impl Usage {
    pub fn from_anthropic_body(body: &[u8]) -> Option<Usage> {
        #[derive(Deserialize)]
        struct AnthropicResp {
            usage: Option<AnthropicUsage>,
        }
        #[derive(Deserialize)]
        struct AnthropicUsage {
            input_tokens: Option<u64>,
            output_tokens: Option<u64>,
        }
        serde_json::from_slice::<AnthropicResp>(body)
            .ok()
            .and_then(|r| r.usage)
            .map(|u| Usage {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
            })
    }

    pub fn from_openai_body(body: &[u8]) -> Option<Usage> {
        #[derive(Deserialize)]
        struct OpenAiResp {
            usage: Option<OpenAiUsage>,
        }
        #[derive(Deserialize)]
        struct OpenAiUsage {
            prompt_tokens: Option<u64>,
            completion_tokens: Option<u64>,
        }
        serde_json::from_slice::<OpenAiResp>(body)
            .ok()
            .and_then(|r| r.usage)
            .map(|u| Usage {
                input_tokens: u.prompt_tokens,
                output_tokens: u.completion_tokens,
            })
    }

    pub fn total_tokens(&self) -> u64 {
        self.input_tokens.unwrap_or(0) + self.output_tokens.unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthropic_usage_parsed() {
        let body = br#"{"usage":{"input_tokens":10,"output_tokens":5}}"#;
        let u = Usage::from_anthropic_body(body).unwrap();
        assert_eq!(u.input_tokens, Some(10));
        assert_eq!(u.output_tokens, Some(5));
        assert_eq!(u.total_tokens(), 15);
    }

    #[test]
    fn openai_usage_parsed() {
        let body = br#"{"usage":{"prompt_tokens":8,"completion_tokens":4}}"#;
        let u = Usage::from_openai_body(body).unwrap();
        assert_eq!(u.input_tokens, Some(8));
        assert_eq!(u.output_tokens, Some(4));
    }

    #[test]
    fn missing_usage_returns_none() {
        let body = br#"{"content":"hello"}"#;
        assert!(Usage::from_anthropic_body(body).is_none());
        assert!(Usage::from_openai_body(body).is_none());
    }
}
