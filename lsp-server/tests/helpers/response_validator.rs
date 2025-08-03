use anyhow::{anyhow, Result};
use serde_json::Value;
use tower_lsp::lsp_types::*;

pub struct ResponseValidator;

impl ResponseValidator {
    pub fn validate_jsonrpc_response(response: &Value) -> Result<()> {
        // Check required fields
        if !response.is_object() {
            return Err(anyhow!("Response must be an object"));
        }

        let obj = response.as_object().unwrap();

        // Check jsonrpc version
        match obj.get("jsonrpc") {
            Some(Value::String(version)) if version == "2.0" => {},
            _ => return Err(anyhow!("Missing or invalid jsonrpc version")),
        }

        // Must have either result or error, and id
        let has_result = obj.contains_key("result");
        let has_error = obj.contains_key("error");
        let has_id = obj.contains_key("id");

        if !has_id {
            return Err(anyhow!("Response missing id field"));
        }

        if !has_result && !has_error {
            return Err(anyhow!("Response must have either result or error"));
        }

        if has_result && has_error {
            return Err(anyhow!("Response cannot have both result and error"));
        }

        Ok(())
    }

    pub fn validate_error_response(response: &Value, expected_code: i64) -> Result<()> {
        Self::validate_jsonrpc_response(response)?;

        let error = response["error"].as_object()
            .ok_or_else(|| anyhow!("Error field must be an object"))?;

        let code = error["code"].as_i64()
            .ok_or_else(|| anyhow!("Error code must be a number"))?;

        if code != expected_code {
            return Err(anyhow!("Expected error code {}, got {}", expected_code, code));
        }

        let _message = error["message"].as_str()
            .ok_or_else(|| anyhow!("Error message must be a string"))?;

        Ok(())
    }

    pub fn validate_initialize_response(response: &InitializeResult) -> Result<()> {
        // Validate server info
        if let Some(ref server_info) = response.server_info {
            if server_info.name.is_empty() {
                return Err(anyhow!("Server name cannot be empty"));
            }
        } else {
            return Err(anyhow!("Server info is required"));
        }

        // Validate capabilities structure
        let caps = &response.capabilities;
        
        // If text document sync is provided, validate it
        if let Some(ref sync) = caps.text_document_sync {
            match sync {
                TextDocumentSyncCapability::Kind(kind) => {
                    match *kind {
                        TextDocumentSyncKind::NONE | 
                        TextDocumentSyncKind::FULL | 
                        TextDocumentSyncKind::INCREMENTAL => {},
                        _ => return Err(anyhow!("Invalid text document sync kind")),
                    }
                },
                TextDocumentSyncCapability::Options(_) => {
                    // Options format is also valid
                }
            }
        }

        // Validate completion provider if present
        if let Some(ref completion) = caps.completion_provider {
            if let Some(ref chars) = completion.trigger_characters {
                if chars.is_empty() {
                    return Err(anyhow!("Trigger characters should not be empty array"));
                }
            }
        }

        Ok(())
    }

    pub fn validate_capability_intersection(
        client_caps: &ClientCapabilities, 
        server_caps: &ServerCapabilities
    ) -> Result<()> {
        // If client doesn't support hover, server shouldn't advertise it
        if let Some(ref text_doc) = client_caps.text_document {
            if text_doc.hover.is_none() && server_caps.hover_provider.is_some() {
                return Err(anyhow!("Server advertises hover but client doesn't support it"));
            }

            if text_doc.completion.is_none() && server_caps.completion_provider.is_some() {
                return Err(anyhow!("Server advertises completion but client doesn't support it"));
            }
        }

        Ok(())
    }

    pub fn validate_notification(notification: &Value) -> Result<()> {
        if !notification.is_object() {
            return Err(anyhow!("Notification must be an object"));
        }

        let obj = notification.as_object().unwrap();

        // Check jsonrpc version
        match obj.get("jsonrpc") {
            Some(Value::String(version)) if version == "2.0" => {},
            _ => return Err(anyhow!("Missing or invalid jsonrpc version")),
        }

        // Must have method, must not have id
        if !obj.contains_key("method") {
            return Err(anyhow!("Notification missing method field"));
        }

        if obj.contains_key("id") {
            return Err(anyhow!("Notification must not have id field"));
        }

        Ok(())
    }

    pub fn validate_request_id_correlation(request: &Value, response: &Value) -> Result<()> {
        let request_id = request.get("id")
            .ok_or_else(|| anyhow!("Request missing id"))?;
        
        let response_id = response.get("id")
            .ok_or_else(|| anyhow!("Response missing id"))?;

        if request_id != response_id {
            return Err(anyhow!("Request and response IDs don't match"));
        }

        Ok(())
    }

    pub fn extract_error_code(response: &Value) -> Result<i64> {
        let error = response.get("error")
            .and_then(|e| e.as_object())
            .ok_or_else(|| anyhow!("No error object in response"))?;

        error.get("code")
            .and_then(|c| c.as_i64())
            .ok_or_else(|| anyhow!("No error code in error object"))
    }

    pub fn is_success_response(response: &Value) -> bool {
        response.get("result").is_some() && response.get("error").is_none()
    }

    pub fn is_error_response(response: &Value) -> bool {
        response.get("error").is_some() && response.get("result").is_none()
    }
}