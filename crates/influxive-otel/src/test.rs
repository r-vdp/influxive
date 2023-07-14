use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn sanity() {
    let tmp = tempfile::tempdir().unwrap();

    const METRIC: &'static str = "my.metric";

    let i = Arc::new(
        Influxive::new(Config {
            influxd_path: Some("bad".into()),
            influx_path: Some("bad".into()),
            database_path: Some(tmp.path().into()),
            metric_write_batch_duration: std::time::Duration::from_millis(5),
            ..Default::default()
        })
        .await
        .unwrap(),
    );

    println!("{}", i.get_host());

    i.ping().await.unwrap();

    let meter_provider = InfluxiveMeterProvider::new(i.clone());
    opentelemetry_api::global::set_meter_provider(meter_provider);

    let meter = opentelemetry_api::global::meter(METRIC);
    let metric = meter.f64_histogram(METRIC).init();

    let mut last_time = std::time::Instant::now();

    for _ in 0..12 {
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;

        let cx = opentelemetry_api::Context::new();
        metric.record(&cx, last_time.elapsed().as_secs_f64(), &[]);

        last_time = std::time::Instant::now();
    }

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let result = i
        .query(
            r#"from(bucket: "influxive")
|> range(start: -15m, stop: now())
|> filter(fn: (r) => r["_measurement"] == "my.metric")
|> filter(fn: (r) => r["_field"] == "value")"#,
        )
        .await
        .unwrap();

    // make sure the result contains at least 10 of the entries
    let line_count = result.split("\n").count();
    assert!(line_count >= 10, "{result}");

    drop(i);

    // okay if this fails on windows...
    let _ = tmp.close();
}
