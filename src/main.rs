#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate prometheus;
use hyper::{
    header::CONTENT_TYPE,
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use prometheus::{
    IntGaugeVec, Registry, Encoder, TextEncoder, TEXT_FORMAT,
};
use std::env;
use std::result::Result;
use std::time::Duration;
use serde_json::Value;
use reqwest::Url;

lazy_static! {
    static ref SUPPLIES_CAPACITY: IntGaugeVec = register_int_gauge_vec!(
        "lexmark_supplies_capacity",
        "LexMark Supplies capacity",
        &["device", "supply"]
    )
    .unwrap();
    static ref SUPPLIES_CURRENT_LEVEL: IntGaugeVec = register_int_gauge_vec!(
        "lexmark_supplies_current_level",
        "LexMark Supplies current level",
        &["device", "supply"]
    )
    .unwrap();
    pub static ref REGISTRY: Registry = Registry::new();
}

#[tokio::main]
async fn main() {
    register_custom_metrics();
    let addr = ([0, 0, 0, 0], 16289).into();
    println!("Listening on http://{}", addr);

    tokio::task::spawn(data_collector());

    let serve_future = Server::bind(&addr).serve(make_service_fn(|_| async {
        Ok::<_, hyper::Error>(service_fn(metrics_handler))
    }));

    if let Err(err) = serve_future.await {
        eprintln!("server error: {}", err);
    }
}

fn register_custom_metrics() {
    REGISTRY
        .register(Box::new(SUPPLIES_CURRENT_LEVEL.clone()))
        .expect("collector can be registered");
}

async fn data_collector() {
    let mut collect_interval = tokio::time::interval(Duration::from_secs(33));
    loop {
        collect_interval.tick().await;
        for url in env::var("LEXMARK_URLS").unwrap().split(",") {
            collect_data_from_printer(url).await;
        }
    }
}

async fn collect_data_from_printer(url_base: &str) {
    println!("Fetching supply information from {}", url_base);
    let response = reqwest::get(Url::parse(url_base).unwrap().join("/webglue/rawcontent?timedRefresh=1&c=Status&lang=en").unwrap()).await.unwrap();
    let body = response.json::<serde_json::Map<String, Value>>().await.unwrap();
    let supplies = body["nodes"]["supplies"].as_object().unwrap();
    supplies
        .iter()
        .map(|(k, v)| (k, v["capacity"].as_i64()))
        .for_each(|(key, v)| {
            match v {
                None => SUPPLIES_CAPACITY.with_label_values(&[url_base, key]).set(0 as i64),
                Some(capacity) => SUPPLIES_CAPACITY.with_label_values(&[url_base, key]).set(capacity as i64),
            }
        });
    supplies
        .iter()
        .map(|(k, v)| (k, v["curlevel"].as_i64()))
        .for_each(|(key, v)| {
            match v {
                None => SUPPLIES_CURRENT_LEVEL.with_label_values(&[url_base, key]).set(0 as i64),
                Some(capacity) => SUPPLIES_CURRENT_LEVEL.with_label_values(&[url_base, key]).set(capacity as i64),
            }
        });
}

async fn metrics_handler(_req: Request<Body>) -> Result<Response<Body>, hyper::Error>  {
    let encoder = TextEncoder::new();

    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();

    let response = Response::builder()
        .status(200)
        .header(CONTENT_TYPE, TEXT_FORMAT)
        .body(Body::from(buffer))
        .unwrap();

    Ok(response)
}
