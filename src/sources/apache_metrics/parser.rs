fn parse(
    packet: &str,
    namespace: &str,
    now: DateTime<Utc>,
    tags: &BTreeMap<String, String>,
) -> (Vec<Metric>, Vec<ParseError>) {
    packet
        .lines()
        .into_iter()
        .filter_map(|l| {
            let mut parts = l.splitn(2, ":");
            let key = parts.next();
            let value = parts.next().map(|s| s.trim());
            match (key, value) {
                (Some(k), Some(v)) => Some((k, v)),
                _ => None,
            }
        })
        .map(|(key, value)| line_to_metrics(key, value, namespace, now, &tags))
        .fold(
            (Vec::new(), Vec::new()),
            |(mut metrics, mut errs), current| {
                match current {
                    LineResult::Metrics(m) => metrics.extend(m),
                    LineResult::Error(err) => errs.push(err),
                    LineResult::None => {}
                }
                (metrics, errs)
            },
        )
}

#[derive(Debug)]
struct ParseError {
    key: String,
    err: Box<dyn Error>,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "could not parse value for {}: {}", self.key, self.err)
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.err.as_ref())
    }
}

enum LineResult {
    Metrics(Vec<Metric>),
    Error(ParseError),
    None,
}

fn line_to_metrics(
    key: &str,
    value: &str,
    namespace: &str,
    now: DateTime<Utc>,
    tags: &BTreeMap<String, String>,
) -> LineResult {
    match key {
        "Uptime" => match value.parse::<f64>() {
            Ok(value) => LineResult::Metrics(vec![
                Metric {
                    name: encode_namespace(namespace, "uptime_seconds_total"),
                    timestamp: Some(now),
                    tags: Some(tags.clone()),
                    kind: MetricKind::Absolute,
                    value: MetricValue::Counter { value },
                },
                Metric {
                    name: encode_namespace(namespace, "up"),
                    timestamp: Some(now),
                    tags: Some(tags.clone()),
                    kind: MetricKind::Absolute,
                    value: MetricValue::Counter { value: 1.0 },
                },
            ]),
            Err(err) => LineResult::Error(ParseError {
                key: key.to_string(),
                err: err.into(),
            }),
        },
        "Total Accesses" => match value.parse::<f64>() {
            Ok(value) => LineResult::Metrics(vec![Metric {
                name: encode_namespace(namespace, "access_total"),
                timestamp: Some(now),
                tags: Some(tags.clone()),
                kind: MetricKind::Absolute,
                value: MetricValue::Counter { value },
            }]),
            Err(err) => LineResult::Error(ParseError {
                key: key.to_string(),
                err: err.into(),
            }),
        },
        "Total kBytes" => match value.parse::<u32>().map(|v| v * 1024) {
            Ok(value) => LineResult::Metrics(vec![Metric {
                name: encode_namespace(namespace, "sent_bytes_total"),
                timestamp: Some(now),
                tags: Some(tags.clone()),
                kind: MetricKind::Absolute,
                value: MetricValue::Counter {
                    value: value.into(),
                },
            }]),
            Err(err) => LineResult::Error(ParseError {
                key: key.to_string(),
                err: err.into(),
            }),
        },
        "Total Duration" => match value.parse::<f64>() {
            Ok(value) => LineResult::Metrics(vec![Metric {
                name: encode_namespace(namespace, "duration_seconds_total"),
                timestamp: Some(now),
                tags: Some(tags.clone()),
                kind: MetricKind::Absolute,
                value: MetricValue::Counter { value }, // TODO verify unit
            }]),
            Err(err) => LineResult::Error(ParseError {
                key: key.to_string(),
                err: err.into(),
            }),
        },
        "CPUUser" => match value.parse::<f64>() {
            Ok(value) => LineResult::Metrics(vec![Metric {
                name: encode_namespace(namespace, "cpu_seconds_total"),
                timestamp: Some(now),
                tags: Some(tags.clone()).map(|mut tags| {
                    tags.insert("type".to_string(), "user".to_string());
                    tags
                }),
                kind: MetricKind::Absolute,
                value: MetricValue::Gauge { value },
            }]),
            Err(err) => LineResult::Error(ParseError {
                key: key.to_string(),
                err: err.into(),
            }),
        },
        "CPUSystem" => match value.parse::<f64>() {
            Ok(value) => LineResult::Metrics(vec![Metric {
                name: encode_namespace(namespace, "cpu_seconds_total"),
                timestamp: Some(now),
                tags: Some(tags.clone()).map(|mut tags| {
                    tags.insert("type".to_string(), "system".to_string());
                    tags
                }),
                kind: MetricKind::Absolute,
                value: MetricValue::Gauge { value },
            }]),
            Err(err) => LineResult::Error(ParseError {
                key: key.to_string(),
                err: err.into(),
            }),
        },
        "CPUChildrenUser" => match value.parse::<f64>() {
            Ok(value) => LineResult::Metrics(vec![Metric {
                name: encode_namespace(namespace, "cpu_seconds_total"),
                timestamp: Some(now),
                tags: Some(tags.clone()).map(|mut tags| {
                    tags.insert("type".to_string(), "children_user".to_string());
                    tags
                }),
                kind: MetricKind::Absolute,
                value: MetricValue::Gauge { value },
            }]),
            Err(err) => LineResult::Error(ParseError {
                key: key.to_string(),
                err: err.into(),
            }),
        },
        "CPUChildrenSystem" => match value.parse::<f64>() {
            Ok(value) => LineResult::Metrics(vec![Metric {
                name: encode_namespace(namespace, "cpu_seconds_total"),
                timestamp: Some(now),
                tags: Some(tags.clone()).map(|mut tags| {
                    tags.insert("type".to_string(), "children_system".to_string());
                    tags
                }),
                kind: MetricKind::Absolute,
                value: MetricValue::Gauge { value },
            }]),
            Err(err) => LineResult::Error(ParseError {
                key: key.to_string(),
                err: err.into(),
            }),
        },
        "CPULoad" => match value.parse::<f64>() {
            Ok(value) => LineResult::Metrics(vec![Metric {
                name: encode_namespace(namespace, "cpu_load"),
                timestamp: Some(now),
                tags: Some(tags.clone()),
                kind: MetricKind::Absolute,
                value: MetricValue::Gauge { value },
            }]),
            Err(err) => LineResult::Error(ParseError {
                key: key.to_string(),
                err: err.into(),
            }),
        },
        "IdleWorkers" => match value.parse::<f64>() {
            Ok(value) => LineResult::Metrics(vec![Metric {
                name: encode_namespace(namespace, "workers"),
                timestamp: Some(now),
                tags: Some(tags.clone()).map(|mut tags| {
                    tags.insert("state".to_string(), "idle".to_string());
                    tags
                }),
                kind: MetricKind::Absolute,
                value: MetricValue::Gauge { value },
            }]),
            Err(err) => LineResult::Error(ParseError {
                key: key.to_string(),
                err: err.into(),
            }),
        },
        "BusyWorkers" => match value.parse::<f64>() {
            Ok(value) => LineResult::Metrics(vec![Metric {
                name: encode_namespace(namespace, "workers"),
                timestamp: Some(now),
                tags: Some(tags.clone()).map(|mut tags| {
                    tags.insert("state".to_string(), "busy".to_string());
                    tags
                }),
                kind: MetricKind::Absolute,
                value: MetricValue::Gauge { value },
            }]),
            Err(err) => LineResult::Error(ParseError {
                key: key.to_string(),
                err: err.into(),
            }),
        },
        "ConnsTotal" => match value.parse::<f64>() {
            Ok(value) => LineResult::Metrics(vec![Metric {
                name: encode_namespace(namespace, "connections"),
                timestamp: Some(now),
                tags: Some(tags.clone()).map(|mut tags| {
                    tags.insert("state".to_string(), "total".to_string());
                    tags
                }),
                kind: MetricKind::Absolute,
                value: MetricValue::Gauge { value },
            }]),
            Err(err) => LineResult::Error(ParseError {
                key: key.to_string(),
                err: err.into(),
            }),
        },
        "ConnsAsyncWriting" => match value.parse::<f64>() {
            Ok(value) => LineResult::Metrics(vec![Metric {
                name: encode_namespace(namespace, "connections"),
                timestamp: Some(now),
                tags: Some(tags.clone()).map(|mut tags| {
                    tags.insert("state".to_string(), "writing".to_string());
                    tags
                }),
                kind: MetricKind::Absolute,
                value: MetricValue::Gauge { value },
            }]),
            Err(err) => LineResult::Error(ParseError {
                key: key.to_string(),
                err: err.into(),
            }),
        },
        "ConnsAsyncClosing" => match value.parse::<f64>() {
            Ok(value) => LineResult::Metrics(vec![Metric {
                name: encode_namespace(namespace, "connections"),
                timestamp: Some(now),
                tags: Some(tags.clone()).map(|mut tags| {
                    tags.insert("state".to_string(), "closing".to_string());
                    tags
                }),
                kind: MetricKind::Absolute,
                value: MetricValue::Gauge { value },
            }]),
            Err(err) => LineResult::Error(ParseError {
                key: key.to_string(),
                err: err.into(),
            }),
        },
        "ConnsAsyncKeepAlive" => match value.parse::<f64>() {
            Ok(value) => LineResult::Metrics(vec![Metric {
                name: encode_namespace(namespace, "connections"),
                timestamp: Some(now),
                tags: Some(tags.clone()).map(|mut tags| {
                    tags.insert("state".to_string(), "keepalive".to_string());
                    tags
                }),
                kind: MetricKind::Absolute,
                value: MetricValue::Gauge { value },
            }]),
            Err(err) => LineResult::Error(ParseError {
                key: key.to_string(),
                err: err.into(),
            }),
        },
        "Scoreboard" => {
            let to_metric = |state: &str, count: &u32| Metric {
                name: encode_namespace(namespace, "scoreboard"),
                timestamp: Some(now),
                tags: Some(tags.clone()).map(|mut tags| {
                    tags.insert("state".to_string(), state.to_string());
                    tags
                }),
                kind: MetricKind::Absolute,
                value: MetricValue::Gauge {
                    value: (*count).into(),
                },
            };

            let scores = value.chars().fold(HashMap::new(), |mut m, c| {
                *m.entry(c).or_insert(0u32) += 1;
                m
            });

            let scoreboard: HashMap<char, &str> = vec![
                ('_', "waiting"),
                ('S', "starting"),
                ('R', "reading"),
                ('W', "sending"),
                ('K', "keepalive"),
                ('D', "dnslookup"),
                ('C', "closing"),
                ('L', "logging"),
                ('G', "finishing"),
                ('I', "idle_cleanup"),
                ('.', "open"),
            ]
            .into_iter()
            .collect();

            LineResult::Metrics(
                scoreboard
                    .iter()
                    .map(|(c, name)| to_metric(name, scores.get(c).unwrap_or(&0u32)))
                    .collect::<Vec<_>>(),
            )
        }
        _ => LineResult::None,
    }
}

fn encode_namespace(namespace: &str, name: &str) -> String {
    if !namespace.is_empty() {
        format!("{}_{}", namespace, name)
    } else {
        name.to_string()
    }
}
