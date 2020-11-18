use serde_json::Value;
use std::convert::TryFrom;
use std::time::Duration;

use super::{MetricError, RawMetric};

/// Parsed metric types
#[derive(Debug, PartialEq)]
pub enum MetricType {
    /// Time is parsed as duration in milliseconds
    /// duration is casted to float seconds
    Time(Duration), // Milliseconds
    /// Bytes
    Bytes(i64),
    /// Integer gauges
    Gauge(i64),
    /// Float gauges
    GaugeF(f64),
    /// Switch metrics having value of true/false
    Switch(u8),

    /// Labels e.g.: index, node, ip, etc.
    Label(String), // Everything not number

    /// Null value
    Null,
}

impl<'s> TryFrom<RawMetric<'s>> for MetricType {
    type Error = MetricError;

    fn try_from(metric: RawMetric) -> Result<Self, MetricError> {
        let value: &Value = metric.1;

        let unknown = || MetricError::unknown(metric.0.to_owned(), Some(value.clone()));

        let parse_i64 = || -> Result<i64, MetricError> {
            if value.is_number() {
                Ok(value.as_i64().unwrap_or(0))
            } else {
                value
                    .as_str()
                    .map(|n| n.parse::<i64>())
                    .ok_or(unknown())?
                    .map_err(|e| MetricError::from_parse_int(e, Some(value.clone())))
            }
        };

        let parse_f64 = || -> Result<f64, MetricError> {
            if value.is_f64() {
                Ok(value.as_f64().unwrap_or(0.0))
            } else {
                value
                    .as_str()
                    .map(|n| n.replace("%", "").parse::<f64>())
                    .ok_or(unknown())?
                    .map_err(|e| MetricError::from_parse_float(e, Some(value.clone())))
            }
        };

        if value.is_boolean() {
            return Ok(MetricType::Switch(if value.as_bool().unwrap_or(false) {
                1
            } else {
                0
            }));
        }

        if value.is_null() {
            return Ok(MetricType::Null);
        }

        match metric.0 {
            "size" | "memory" | "store" | "bytes" => return Ok(MetricType::Bytes(parse_i64()?)),
            "epoch" | "timestamp" | "date" | "time" | "millis" | "alive" => {
                return Ok(MetricType::Time(Duration::from_millis(
                    parse_i64().unwrap_or(0) as u64,
                )))
            }
            _ => {
                if value.is_number() {
                    if value.is_i64() {
                        return Ok(MetricType::Gauge(parse_i64()?));
                    } else {
                        return Ok(MetricType::GaugeF(parse_f64()?));
                    }
                }
            }
        }

        // TODO: rethink list matching, label could be matched by default with
        // attempt to number before return
        match metric.0 {
            // timed_out
            "out" | "value" | "committed" | "searchable" | "compound" | "throttled" => {
                Ok(MetricType::Switch(if value.as_bool().unwrap_or(false) {
                    1
                } else {
                    0
                }))
            }

            // Special cases
            // _cat/health: elasticsearch_cat_health_node_data{cluster="testing"}
            // _cat/shards: "path.data": "/var/lib/elasticsearch/m1/nodes/0"
            "data" => match parse_i64() {
                Ok(number) => Ok(MetricType::Gauge(number)),
                Err(_) => Ok(MetricType::Label(
                    value.as_str().ok_or(unknown())?.to_owned(),
                )),
            },

            "primaries" | "min" | "max" | "successful" | "nodes" | "fetch" | "order"
            | "largest" | "rejected" | "completed" | "queue" | "active" | "core" | "tasks"
            | "relo" | "unassign" | "init" | "files" | "ops" | "recovered" | "generation"
            | "contexts" | "listeners" | "pri" | "rep" | "docs" | "count" | "pid"
            | "compilations" | "deleted" | "shards" | "indices" | "checkpoint" | "avail"
            | "used" | "cpu" | "triggered" | "evictions" | "failed" | "total" | "current" => {
                Ok(MetricType::Gauge(parse_i64()?))
            }

            "avg" | "1m" | "5m" | "15m" | "number" | "percent" => {
                Ok(MetricType::GaugeF(parse_f64()?))
            }

            "cluster" | "repository" | "snapshot" | "stage" | "uuid" | "component" | "master"
            | "role" | "uptime" | "alias" | "filter" | "search" | "flavor" | "string"
            | "address" | "health" | "build" | "node" | "state" | "patterns" | "of" | "segment"
            | "host" | "ip" | "prirep" | "id" | "status" | "at" | "for" | "details" | "reason"
            | "port" | "attr" | "field" | "shard" | "index" | "name" | "type" | "version"
            | "jdk" | "description" => Ok(MetricType::Label(
                value.as_str().ok_or(unknown())?.to_owned(),
            )),
            _ => {
                if cfg!(debug_assertions) {
                    println!("Catchall metric: {:?}", metric);

                    let parsed = parse_i64().unwrap_or(-1).to_string();

                    if &parsed != "-1" && parsed.len() == value.as_str().unwrap_or("").len() {
                        println!("Unhandled metic value {:?}", metric);
                    }
                }

                Ok(MetricType::Label(
                    value.as_str().ok_or(unknown())?.to_owned(),
                ))
            }
        }
    }
}