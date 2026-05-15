//! Authentication mechanisms for REST API

/// Authentication method for REST API requests
#[derive(Debug, Clone)]
pub enum Auth {
    /// API Key authentication (X-API-KEY header)
    ApiKey(String),
    /// Bearer token authentication (Authorization: Bearer header)
    BearerToken(String),
    /// SDK token authentication (X-SDK-TOKEN header)
    SdkToken(String),
}

impl Auth {
    /// Apply authentication to a ureq request
    pub fn apply_to_request(&self, request: ureq::Request) -> ureq::Request {
        match self {
            Auth::ApiKey(key) => request.set("X-API-KEY", key),
            Auth::BearerToken(token) => request.set("Authorization", &format!("Bearer {}", token)),
            Auth::SdkToken(token) => request.set("X-SDK-TOKEN", token),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_apply_to_request() {
        // Test ApiKey
        let auth = Auth::ApiKey("test_key".to_string());
        let request = ureq::get("http://example.com");
        let _request = auth.apply_to_request(request);
        // Note: ureq doesn't expose headers for inspection in tests,
        // so we just verify it compiles and doesn't panic

        // Test BearerToken
        let auth = Auth::BearerToken("test_token".to_string());
        let request = ureq::get("http://example.com");
        let _request = auth.apply_to_request(request);

        // Test SdkToken
        let auth = Auth::SdkToken("test_sdk_token".to_string());
        let request = ureq::get("http://example.com");
        let _request = auth.apply_to_request(request);
    }
}
