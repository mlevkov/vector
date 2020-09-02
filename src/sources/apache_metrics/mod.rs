use crate::{
    config::{self, GlobalOptions},
    event::metric::{Metric, MetricKind, MetricValue},
    internal_events::{
        ApacheMetricsErrorResponse, ApacheMetricsEventReceived, ApacheMetricsHttpError,
        ApacheMetricsParseError, ApacheMetricsRequestCompleted,
    },
    shutdown::ShutdownSignal,
    Event, Pipeline,
};
use futures::{
    compat::{Future01CompatExt, Sink01CompatExt},
    future, stream, FutureExt, StreamExt, TryFutureExt,
};
use futures01::Sink;
use hyper::{Body, Client, Request};
use hyper_openssl::HttpsConnector;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use std::collections::BTreeMap;
use std::error;
use std::fmt;
use std::time::{Duration, Instant};

#[derive(Deserialize, Serialize, Clone, Debug)]
struct ApacheMetricsConfig {
    endpoints: Vec<String>,
    #[serde(default = "default_scrape_interval_secs")]
    scrape_interval_secs: u64,
    #[serde(default = "default_namespace")]
    namespace: String,
}

pub fn default_scrape_interval_secs() -> u64 {
    15
}

pub fn default_namespace() -> String {
    "apache".to_string()
}

#[typetag::serde(name = "apache_metrics")]
impl crate::config::SourceConfig for ApacheMetricsConfig {
    fn build(
        &self,
        _name: &str,
        _globals: &GlobalOptions,
        shutdown: ShutdownSignal,
        out: Pipeline,
    ) -> crate::Result<super::Source> {
        let urls = self
            .endpoints
            .iter()
            .map(|endpoint| endpoint.parse::<http::Uri>())
            .collect::<Result<Vec<_>, _>>()
            .context(super::UriParseError)?;

        Ok(apache_metrics(
            urls,
            self.scrape_interval_secs,
            self.namespace.clone(),
            shutdown,
            out,
        ))
    }

    fn output_type(&self) -> crate::config::DataType {
        config::DataType::Metric
    }

    fn source_type(&self) -> &'static str {
        "apache_metrics"
    }
}

fn apache_metrics(
    urls: Vec<http::Uri>,
    interval: u64,
    namespace: String,
    shutdown: ShutdownSignal,
    out: Pipeline,
) -> super::Source {
    let out = out
        .sink_map_err(|e| error!("error sending metric: {:?}", e))
        .sink_compat();
    let task = tokio::time::interval(Duration::from_secs(interval))
        .take_until(shutdown.compat())
        .map(move |_| stream::iter(urls.clone())) // TODO remove clone?
        .flatten()
        .map(|url| {
            let https = HttpsConnector::new().expect("TLS initialization failed");
            let client = Client::builder().build(https);

            let request = Request::get(&url)
                .body(Body::empty())
                .expect("error creating request");

            let start = Instant::now();
            client
                .request(request)
                .and_then(|response| async {
                    let (header, body) = response.into_parts();
                    let body = hyper::body::to_bytes(body).await?;
                    Ok((header, body))
                })
                .into_stream()
                .filter_map(move |response| {
                    future::ready(match response {
                        Ok((header, body)) if header.status == hyper::StatusCode::OK => {
                            emit!(ApacheMetricsRequestCompleted {
                                start,
                                end: Instant::now()
                            });

                            let byte_size = body.len();
                            let body = String::from_utf8_lossy(&body);

                            match parse(&"TODO".to_string(), &body, BTreeMap::new()) {
                                Ok(metrics) => {
                                    emit!(ApacheMetricsEventReceived {
                                        byte_size,
                                        count: metrics.len(),
                                    });
                                    Some(stream::iter(metrics).map(Event::Metric).map(Ok))
                                }
                                Err(errors) => {
                                    // TODO emit one per error
                                    errors.into_iter().next().and_then(|error| {
                                        emit!(ApacheMetricsParseError {
                                            error: error.into(),
                                            url: &url,
                                            body,
                                        });
                                        None
                                    })
                                }
                            }
                        }
                        Ok((header, _)) => {
                            emit!(ApacheMetricsErrorResponse {
                                code: header.status,
                                url: &url,
                            });
                            None
                        }
                        Err(error) => {
                            emit!(ApacheMetricsHttpError { error, url: &url });
                            None
                        }
                    })
                })
                .flatten()
        })
        .flatten()
        .forward(out)
        .inspect(|_| info!("finished sending"));

    Box::new(task.boxed().compat())
}

#[derive(Debug)]
struct ParseError;

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO define
        write!(f, "{}", "")
    }
}

// TODO define
impl error::Error for ParseError {}

fn parse(
    namespace: &str,
    packet: &str,
    tags: BTreeMap<&str, &str>,
) -> Result<Vec<Metric>, Vec<ParseError>> {
    // TODO parse errors
    Ok(Vec::new())
}

//#[cfg(feature = "sinks-apache_metrics")]
//#[cfg(test)]
//mod test {
//use super::*;
//use crate::{
//config,
//sinks::apache_metrics::ApacheMetricsSinkConfig,
//test_util::{next_addr, start_topology},
//Error,
//};
//use futures::compat::Future01CompatExt;
//use hyper::{
//service::{make_service_fn, service_fn},
//{Body, Client, Response, Server},
//};
//use pretty_assertions::assert_eq;
//use tokio::time::{delay_for, Duration};

//#[tokio::test]
//async fn test_prometheus_routing() {
//let in_addr = next_addr();
//let out_addr = next_addr();

//let make_svc = make_service_fn(|_| async {
//Ok::<_, Error>(service_fn(|_| async {
//Ok::<_, Error>(Response::new(Body::from(
//r##"
//# HELP promhttp_metric_handler_requests_total Total number of scrapes by HTTP status code.
//# TYPE promhttp_metric_handler_requests_total counter
//promhttp_metric_handler_requests_total{code="200"} 100
//promhttp_metric_handler_requests_total{code="404"} 7
//prometheus_remote_storage_samples_in_total 57011636
//# A histogram, which has a pretty complex representation in the text format:
//# HELP http_request_duration_seconds A histogram of the request duration.
//# TYPE http_request_duration_seconds histogram
//http_request_duration_seconds_bucket{le="0.05"} 24054
//http_request_duration_seconds_bucket{le="0.1"} 33444
//http_request_duration_seconds_bucket{le="0.2"} 100392
//http_request_duration_seconds_bucket{le="0.5"} 129389
//http_request_duration_seconds_bucket{le="1"} 133988
//http_request_duration_seconds_bucket{le="+Inf"} 144320
//http_request_duration_seconds_sum 53423
//http_request_duration_seconds_count 144320
//# Finally a summary, which has a complex representation, too:
//# HELP rpc_duration_seconds A summary of the RPC duration in seconds.
//# TYPE rpc_duration_seconds summary
//rpc_duration_seconds{code="200",quantile="0.01"} 3102
//rpc_duration_seconds{code="200",quantile="0.05"} 3272
//rpc_duration_seconds{code="200",quantile="0.5"} 4773
//rpc_duration_seconds{code="200",quantile="0.9"} 9001
//rpc_duration_seconds{code="200",quantile="0.99"} 76656
//rpc_duration_seconds_sum{code="200"} 1.7560473e+07
//rpc_duration_seconds_count{code="200"} 2693
//"##,
//)))
//}))
//});

//tokio::spawn(async move {
//if let Err(e) = Server::bind(&in_addr).serve(make_svc).await {
//error!("server error: {:?}", e);
//}
//});

//let mut config = config::Config::empty();
//config.add_source(
//"in",
//ApacheMetricsConfig {
//endpoints: vec![format!("http://{}", in_addr)],
//scrape_interval_secs: 1,
//},
//);
//config.add_sink(
//"out",
//&["in"],
//ApacheMetricsSinkConfig {
//address: out_addr,
//namespace: "vector".into(),
//buckets: vec![1.0, 2.0, 4.0],
//flush_period_secs: 1,
//},
//);

//let (topology, _crash) = start_topology(config, false).await;
//delay_for(Duration::from_secs(1)).await;

//let response = Client::new()
//.get(format!("http://{}/metrics", out_addr).parse().unwrap())
//.await
//.unwrap();
//assert!(response.status().is_success());

//let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
//let lines = std::str::from_utf8(&body)
//.unwrap()
//.lines()
//.collect::<Vec<_>>();

//assert_eq!(lines, vec![
//"# HELP vector_promhttp_metric_handler_requests_total promhttp_metric_handler_requests_total",
//"# TYPE vector_promhttp_metric_handler_requests_total counter",
//"vector_promhttp_metric_handler_requests_total{code=\"200\"} 100",
//"vector_promhttp_metric_handler_requests_total{code=\"404\"} 7",
//"# HELP vector_prometheus_remote_storage_samples_in_total prometheus_remote_storage_samples_in_total",
//"# TYPE vector_prometheus_remote_storage_samples_in_total gauge",
//"vector_prometheus_remote_storage_samples_in_total 57011636",
//"# HELP vector_http_request_duration_seconds http_request_duration_seconds",
//"# TYPE vector_http_request_duration_seconds histogram",
//"vector_http_request_duration_seconds_bucket{le=\"0.05\"} 24054",
//"vector_http_request_duration_seconds_bucket{le=\"0.1\"} 33444",
//"vector_http_request_duration_seconds_bucket{le=\"0.2\"} 100392",
//"vector_http_request_duration_seconds_bucket{le=\"0.5\"} 129389",
//"vector_http_request_duration_seconds_bucket{le=\"1\"} 133988",
//"vector_http_request_duration_seconds_bucket{le=\"+Inf\"} 144320",
//"vector_http_request_duration_seconds_sum 53423",
//"vector_http_request_duration_seconds_count 144320",
//"# HELP vector_rpc_duration_seconds rpc_duration_seconds",
//"# TYPE vector_rpc_duration_seconds summary",
//"vector_rpc_duration_seconds{code=\"200\",quantile=\"0.01\"} 3102",
//"vector_rpc_duration_seconds{code=\"200\",quantile=\"0.05\"} 3272",
//"vector_rpc_duration_seconds{code=\"200\",quantile=\"0.5\"} 4773",
//"vector_rpc_duration_seconds{code=\"200\",quantile=\"0.9\"} 9001",
//"vector_rpc_duration_seconds{code=\"200\",quantile=\"0.99\"} 76656",
//"vector_rpc_duration_seconds_sum{code=\"200\"} 17560473",
//"vector_rpc_duration_seconds_count{code=\"200\"} 2693",
//],
//);

//topology.stop().compat().await.unwrap();
//}
//}
