//! Security scanning for marketplace plugins
//!
//! Provides interfaces for malware detection, vulnerability analysis, and policy enforcement.
//! This module supports both local scanning (ClamAV) and external scanning services.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// Scan ID for audit trail
    pub id: String,
    /// Whether scan passed (clean)
    pub passed: bool,
    /// Threat level: none, low, medium, high, critical
    pub threat_level: String,
    /// Detected issues (malware, vulnerability, policy violation, etc.)
    pub issues: Vec<SecurityIssue>,
    /// Scanner used (clamav, virustotal, custom, etc.)
    pub scanner: String,
    /// Scan duration in milliseconds
    pub duration_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIssue {
    /// Issue type: malware, vulnerability, suspicious_api, policy_violation, etc.
    pub issue_type: String,
    /// Severity: low, medium, high, critical
    pub severity: String,
    /// Description
    pub description: String,
    /// File path or code location if applicable
    pub location: Option<String>,
    /// CWE or CVE reference if applicable
    pub reference: Option<String>,
}

/// Trait for security scanners
#[async_trait::async_trait]
pub trait SecurityScanner: Send + Sync {
    /// Scan a plugin manifest and return security assessment
    async fn scan_manifest(
        &self,
        plugin_id: Uuid,
        manifest_url: &str,
        source_url: Option<&str>,
    ) -> Result<ScanResult, String>;

    /// Check for known vulnerabilities in dependencies
    async fn scan_dependencies(
        &self,
        dependencies: &[Dependency],
    ) -> Result<Vec<SecurityIssue>, String>;

    /// Analyze code for suspicious patterns
    async fn analyze_code(
        &self,
        code_samples: &[(&str, &str)], // (filename, code)
    ) -> Result<Vec<SecurityIssue>, String>;

    /// Get scanner name
    fn name(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub registry: String, // "npm", "crates.io", "pypi", etc.
}

/// Common library APIs that indicate suspicious behavior
pub struct SuspiciousApiPatterns {
    /// APIs that might indicate backdoors or privilege escalation
    pub dangerous_apis: Vec<&'static str>,
    /// APIs that leak user data
    pub privacy_violation_apis: Vec<&'static str>,
    /// APIs that bypass Nexus security
    pub bypass_apis: Vec<&'static str>,
}

impl SuspiciousApiPatterns {
    pub fn default_list() -> Self {
        Self {
            dangerous_apis: vec![
                "process.execSync",
                "child_process.spawn",
                "eval",
                "Function",
                "fs.writeFileSync",
                "fs.unlink",
                "os.system",
            ],
            privacy_violation_apis: vec![
                "localStorage.getItem",
                "sessionStorage.getItem",
                "document.cookie",
                "fetch.*authorization",
            ],
            bypass_apis: vec![
                // Nexus-specific: APIs that bypass sandboxing
                "nexus.internal",
                "nexus.admin",
                "gateway.raw_socket",
            ],
        }
    }

    /// Check if code contains suspicious patterns
    pub fn check_code_patterns(&self, code: &str) -> Vec<SecurityIssue> {
        let mut issues = vec![];

        for api in &self.dangerous_apis {
            if code.contains(api) {
                issues.push(SecurityIssue {
                    issue_type: "suspicious_api".to_string(),
                    severity: "high".to_string(),
                    description: format!("Use of potentially dangerous API: {}", api),
                    location: None,
                    reference: None,
                });
            }
        }

        for api in &self.privacy_violation_apis {
            if code.contains(api) {
                issues.push(SecurityIssue {
                    issue_type: "privacy_violation".to_string(),
                    severity: "high".to_string(),
                    description: format!("Potential data leakage via: {}", api),
                    location: None,
                    reference: None,
                });
            }
        }

        for api in &self.bypass_apis {
            if code.contains(api) {
                issues.push(SecurityIssue {
                    issue_type: "sandbox_bypass".to_string(),
                    severity: "critical".to_string(),
                    description: format!("Potential security bypass: {}", api),
                    location: None,
                    reference: None,
                });
            }
        }

        issues
    }
}

/// Mock scanner for development/testing
pub struct MockMalwareScanner {
    /// Keywords that trigger mock detections
    pub trigger_keywords: HashMap<&'static str, String>,
}

impl MockMalwareScanner {
    pub fn new() -> Self {
        let mut trigger_keywords = HashMap::new();
        trigger_keywords.insert("malware", "Potentially malicious code detected".to_string());
        trigger_keywords.insert("bitcoin", "Cryptocurrency miner detected".to_string());
        trigger_keywords.insert("keylog", "Keylogger pattern detected".to_string());
        trigger_keywords.insert("backdoor", "Remote access backdoor detected".to_string());

        Self { trigger_keywords }
    }
}

#[async_trait::async_trait]
impl SecurityScanner for MockMalwareScanner {
    async fn scan_manifest(
        &self,
        _plugin_id: Uuid,
        manifest_url: &str,
        source_url: Option<&str>,
    ) -> Result<ScanResult, String> {
        let combined = format!(
            "{} {}",
            manifest_url,
            source_url.unwrap_or("")
        ).to_lowercase();

        let mut issues = vec![];
        let mut threat_level = "none".to_string();

        for (keyword, message) in &self.trigger_keywords {
            if combined.contains(keyword) {
                threat_level = "high".to_string();
                issues.push(SecurityIssue {
                    issue_type: "malware".to_string(),
                    severity: "high".to_string(),
                    description: message.clone(),
                    location: None,
                    reference: None,
                });
            }
        }

        Ok(ScanResult {
            id: uuid::Uuid::new_v4().to_string(),
            passed: issues.is_empty(),
            threat_level,
            issues,
            scanner: "mock_clamav".to_string(),
            duration_ms: 100,
        })
    }

    async fn scan_dependencies(
        &self,
        _dependencies: &[Dependency],
    ) -> Result<Vec<SecurityIssue>, String> {
        // Mock: no known vulnerabilities
        Ok(vec![])
    }

    async fn analyze_code(
        &self,
        code_samples: &[(&str, &str)],
    ) -> Result<Vec<SecurityIssue>, String> {
        let mut issues = vec![];
        let patterns = SuspiciousApiPatterns::default_list();

        for (filename, code) in code_samples {
            for issue in patterns.check_code_patterns(code) {
                issues.push(SecurityIssue {
                    location: Some(filename.to_string()),
                    ..issue
                });
            }
        }

        Ok(issues)
    }

    fn name(&self) -> &str {
        "mock_clamav"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_scanner_detects_malware() {
        let scanner = MockMalwareScanner::new();
        let result = scanner
            .scan_manifest(Uuid::nil(), "https://example.com/malware.js", None)
            .await
            .unwrap();

        assert!(!result.passed);
        assert_eq!(result.threat_level, "high");
        assert!(!result.issues.is_empty());
    }

    #[test]
    fn test_suspicious_patterns_detection() {
        let patterns = SuspiciousApiPatterns::default_list();
        let code = "const proc = process.execSync('rm -rf /')";
        let issues = patterns.check_code_patterns(code);

        assert!(!issues.is_empty());
        assert_eq!(issues[0].issue_type, "suspicious_api");
    }
}
