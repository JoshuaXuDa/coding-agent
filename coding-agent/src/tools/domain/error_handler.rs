//! Error handling helper service
//!
//! Centralizes error conversion logic to eliminate repetitive error handling code
//! across all tools. This service provides consistent error transformation and
//! XML error building capabilities.

use anyhow::Result;
use tirea::prelude::ToolError;
use super::xml_builder::XmlBuilder;

/// Convert anyhow::Error to ToolError with standard message
pub fn to_tool_error(error: anyhow::Error) -> ToolError {
    ToolError::ExecutionFailed(error.to_string())
}

/// Convert anyhow::Error to ToolError with additional context
pub fn to_tool_error_with_context(error: anyhow::Error, context: &str) -> ToolError {
    ToolError::ExecutionFailed(format!("{}: {}", context, error))
}

/// Propagate Result by converting error to ToolError
pub fn propagate_error<T>(result: Result<T>) -> Result<T, ToolError> {
    result.map_err(to_tool_error)
}

/// Propagate Result with context by converting error to ToolError
pub fn propagate_error_with_context<T>(result: Result<T>, context: &str) -> Result<T, ToolError> {
    result.map_err(|e| to_tool_error_with_context(e, context))
}

/// Build error XML response
pub fn build_error_xml(
    tool_name: &str,
    error_code: &str,
    message: &str,
    details: &str,
) -> Result<String, ToolError> {
    XmlBuilder::build_error(tool_name, error_code, message, details)
        .map_err(to_tool_error)
}

/// Convenience functions for common error handling patterns
pub struct ErrorHandler;

impl ErrorHandler {
    /// Convert a Result's error to ToolError (for use with ? operator)
    pub fn convert<T>(result: Result<T>) -> Result<T, ToolError> {
        propagate_error(result)
    }

    /// Convert a Result's error to ToolError with context
    pub fn convert_with_context<T>(result: Result<T>, context: &str) -> Result<T, ToolError> {
        propagate_error_with_context(result, context)
    }

    /// Convert anyhow::Error to ToolError (for use with ? operator)
    pub fn to_tool_error(error: anyhow::Error) -> ToolError {
        to_tool_error(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_tool_error() {
        let error = anyhow::anyhow!("Test error");
        let tool_error = to_tool_error(error);
        assert!(matches!(tool_error, ToolError::ExecutionFailed(_)));
        assert_eq!(tool_error.to_string(), "ExecutionFailed: Test error");
    }

    #[test]
    fn test_to_tool_error_with_context() {
        let error = anyhow::anyhow!("Test error");
        let tool_error = to_tool_error_with_context(error, "Reading file");
        assert!(matches!(tool_error, ToolError::ExecutionFailed(_)));
        assert!(tool_error.to_string().contains("Reading file"));
        assert!(tool_error.to_string().contains("Test error"));
    }

    #[test]
    fn test_propagate_error_ok() {
        let result: Result<i32> = Ok(42);
        let converted = propagate_error(result);
        assert_eq!(converted.unwrap(), 42);
    }

    #[test]
    fn test_propagate_error_err() {
        let result: Result<i32> = Err(anyhow::anyhow!("Failed"));
        let converted = propagate_error(result);
        assert!(converted.is_err());
        assert!(matches!(converted.unwrap_err(), ToolError::ExecutionFailed(_)));
    }

    #[test]
    fn test_propagate_error_with_context_ok() {
        let result: Result<i32> = Ok(42);
        let converted = propagate_error_with_context(result, "Context");
        assert_eq!(converted.unwrap(), 42);
    }

    #[test]
    fn test_propagate_error_with_context_err() {
        let result: Result<i32> = Err(anyhow::anyhow!("Failed"));
        let converted = propagate_error_with_context(result, "Opening file");
        let err = converted.unwrap_err();
        assert!(err.to_string().contains("Opening file"));
        assert!(err.to_string().contains("Failed"));
    }

    #[test]
    fn test_build_error_xml() {
        let xml = build_error_xml(
            "test_tool",
            "FILE_NOT_FOUND",
            "File not found",
            "/path/to/file"
        );
        assert!(xml.is_ok());
        let xml_str = xml.unwrap();
        assert!(xml_str.contains("test_tool"));
        assert!(xml_str.contains("FILE_NOT_FOUND"));
        assert!(xml_str.contains("File not found"));
        assert!(xml_str.contains("/path/to/file"));
    }

    #[test]
    fn test_error_handler_convert() {
        let result: Result<i32> = Ok(42);
        assert_eq!(ErrorHandler::convert(result).unwrap(), 42);

        let result: Result<i32> = Err(anyhow::anyhow!("Error"));
        assert!(ErrorHandler::convert(result).is_err());
    }

    #[test]
    fn test_error_handler_convert_with_context() {
        let result: Result<i32> = Err(anyhow::anyhow!("Error"));
        let err = ErrorHandler::convert_with_context(result, "Context").unwrap_err();
        assert!(err.to_string().contains("Context"));
        assert!(err.to_string().contains("Error"));
    }
}
