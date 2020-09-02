use super::InternalEvent;
use http::Uri;
use metrics::{counter, timing};
use std::borrow::Cow;
use std::error;
use std::time::Instant;

#[derive(Debug)]
pub struct ApacheMetricsEventReceived {
    pub byte_size: usize,
    pub count: usize,
}

impl InternalEvent for ApacheMetricsEventReceived {
    fn emit_logs(&self) {
        debug!(message = "Scraped events.", ?self.count);
    }

    fn emit_metrics(&self) {
        counter!(
            "events_processed", self.count as u64,
            "component_kind" => "source",
            "component_type" => "apache_metrics",
        );
        counter!(
            "bytes_processed", self.byte_size as u64,
            "component_kind" => "source",
            "component_type" => "apache_metrics",
        );
    }
}

#[derive(Debug)]
pub struct ApacheMetricsRequestCompleted {
    pub start: Instant,
    pub end: Instant,
}

impl InternalEvent for ApacheMetricsRequestCompleted {
    fn emit_logs(&self) {
        debug!(message = "Request completed.");
    }

    fn emit_metrics(&self) {
        counter!("requests_completed", 1,
            "component_kind" => "source",
            "component_type" => "apache_metrics",
        );
        timing!("request_duration_nanoseconds", self.start, self.end,
            "component_kind" => "source",
            "component_type" => "apache_metrics",
        );
    }
}

#[derive(Debug)]
pub struct ApacheMetricsParseError<'a> {
    pub error: Box<dyn error::Error>,
    pub url: &'a Uri,
    pub body: Cow<'a, str>,
}

impl<'a> InternalEvent for ApacheMetricsParseError<'a> {
    fn emit_logs(&self) {
        error!(message = "parsing error.", url = %self.url, error = %self.error);
        debug!(
            message = %format!("failed to parse response:\n\n{}\n\n", self.body),
            url = %self.url,
            rate_limit_secs = 10
        );
    }

    fn emit_metrics(&self) {
        counter!("parse_errors", 1,
            "component_kind" => "source",
            "component_type" => "apache_metrics",
        );
    }
}

#[derive(Debug)]
pub struct ApacheMetricsErrorResponse<'a> {
    pub code: hyper::StatusCode,
    pub url: &'a Uri,
}

impl<'a> InternalEvent for ApacheMetricsErrorResponse<'a> {
    fn emit_logs(&self) {
        error!(message = "HTTP error response.", url = %self.url, code = %self.code);
    }

    fn emit_metrics(&self) {
        counter!("http_error_response", 1,
            "component_kind" => "source",
            "component_type" => "apache_metrics",
        );
    }
}

#[derive(Debug)]
pub struct ApacheMetricsHttpError<'a> {
    pub error: hyper::Error,
    pub url: &'a Uri,
}

impl<'a> InternalEvent for ApacheMetricsHttpError<'a> {
    fn emit_logs(&self) {
        error!(message = "HTTP request processing error.", url = %self.url, error = %self.error);
    }

    fn emit_metrics(&self) {
        counter!("http_request_errors", 1,
            "component_kind" => "source",
            "component_type" => "apache_metrics",
        );
    }
}
