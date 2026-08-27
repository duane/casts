// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

//! Entry point: wire the `Casts` service to a real commit store and serve it.

use anyhow::Result;
use casts::grpc::Grpc2Store;
use casts::grpc_interface::casts_server::CastsServer;
use casts::service::{build_tls_config, CastsImpl};
use commit_store::store::ScanCommitsCursor;
use commit_store::Client;
use std::time::Duration;
use tonic::transport::Server;

/// How often the health check re-verifies the commit store is reachable.
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(10);

/// The gRPC health-checking spec's convention: an empty service name means
/// "overall server health", not any one service. This is what a load balancer
/// health check queries by default.
const OVERALL_HEALTH_SERVICE: &str = "";

/// True if the commit store can currently serve a trivial read.
///
/// casts is a front end over the commit store and can do nothing useful
/// without it, so "my port is open" is not health. A task that is up but has
/// lost the store should be pulled out of the load balancer rather than
/// answer every RPC with an error. Same trivial query commit-store's own
/// health check uses: a real round trip with no side effects, where what
/// matters is whether it completes, not what it returns.
async fn commit_store_is_reachable(store: &mut Grpc2Store) -> bool {
    store
        .store()
        .get_range(
            ScanCommitsCursor {
                min_commit: 0,
                max_commit: Some(0),
                reverse: false,
            },
            1,
        )
        .await
        .is_ok()
}

/// Poll `commit_store_is_reachable` on `interval` and update `reporter`,
/// forever. Spawned as a background task from `serve()`.
async fn run_health_check_loop(
    mut store: Grpc2Store,
    mut reporter: tonic_health::server::HealthReporter,
    interval: Duration,
) {
    let mut ticker = tokio::time::interval(interval);
    loop {
        let _ = ticker.tick().await;
        let status = if commit_store_is_reachable(&mut store).await {
            tonic_health::ServingStatus::Serving
        } else {
            tonic_health::ServingStatus::NotServing
        };
        reporter
            .set_service_status(OVERALL_HEALTH_SERVICE, status)
            .await;
    }
}

/// The stream the archival worker reconciles.
///
/// Defaults to the single stream the clients use (`playApp.swift`'s
/// STREAM_ID, and what `seed` writes); `CASTS_STREAM_ID` overrides it.
fn archive_stream_id() -> uuid::Uuid {
    std::env::var("CASTS_STREAM_ID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| uuid::uuid!("34c3c08d-0d55-4af4-a126-170c1380f07e"))
}

pub async fn serve() -> Result<()> {
    let server_config = casts::config::ServerConfigBuilder::default().build()?;

    // Validate the TLS configuration before doing any I/O: a misconfigured
    // server should fail immediately and obviously, not after a connect
    // attempt to the commit store that may itself hang or retry.
    let tls = build_tls_config(server_config.tls_identity, server_config.client_ca)?;

    let commit_store_cfg = commit_store::config::ClientConfigBuilder::default().build();
    log::info!("Connecting to commit store at {}", commit_store_cfg.addr);
    let client = casts::commit_store_client::connect().await?;
    let grpc_client = Client::new(client);

    let store = Grpc2Store::Grpc(grpc_client);

    // Read once and shared: the RPC service and the archival worker both
    // want a content store backed by the same bucket, and reading the
    // config twice would risk the two silently disagreeing if the
    // environment ever named two different buckets.
    let archiver_config = casts::archiver::ArchiverConfig::from_env(archive_stream_id());
    let service = match &archiver_config {
        Some(config) => {
            let aws = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            let content_store = casts::content_store::ContentStore::new(
                aws_sdk_s3::Client::new(&aws),
                config.bucket.clone(),
            );
            CastsServer::new(CastsImpl::with_content_store(store.clone(), content_store))
        }
        // No bucket configured: GetEntryContent reports itself unavailable
        // rather than the service silently answering "no notes" for
        // everything.
        None => CastsServer::new(CastsImpl::new(store.clone())),
    };

    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    // Start NOT_SERVING rather than tonic-health's default of SERVING: a cold
    // start that cannot reach the commit store must not report healthy, even
    // for the window before the first check runs.
    health_reporter
        .set_service_status(
            OVERALL_HEALTH_SERVICE,
            tonic_health::ServingStatus::NotServing,
        )
        .await;
    let _health_check_task = tokio::spawn(run_health_check_loop(
        store,
        health_reporter,
        HEALTH_CHECK_INTERVAL,
    ));

    // The archival worker shares the deployment but not the fate of the gRPC
    // service: it owns a separate commit-store client, and `run_forever`
    // logs and retries rather than returning, so a podcast host being down
    // cannot stop this task from serving. Off unless CASTS_ARCHIVE_BUCKET is
    // set, which keeps local runs and any deployment that does not want it
    // unaffected.
    match archiver_config {
        Some(config) => {
            // The worker gets its own runtime on its own OS threads, not a
            // `tokio::spawn` onto the server's.
            //
            // Sharing the server's runtime means sharing its worker threads,
            // and a task that computes without awaiting holds one until it
            // finishes. The archival pass does exactly that -- folding the
            // aggregate, diffing every feed, cloning both -- so it starved
            // the gRPC health endpoint past the 5s probe timeout and ECS
            // killed the task, repeatedly. Separate OS threads make that a
            // scheduling question the kernel answers by preemption, rather
            // than one the async runtime answers cooperatively and cannot.
            //
            // One worker thread: the pass must never be able to occupy every
            // core, however much work it finds.
            std::thread::Builder::new()
                .name(String::from("archival-worker"))
                .spawn(move || {
                    let runtime = match tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(1)
                        .enable_all()
                        .thread_name("archival-worker-rt")
                        .build()
                    {
                        Ok(runtime) => runtime,
                        Err(err) => {
                            log::error!("archival worker runtime failed to start: {err}");
                            return;
                        }
                    };
                    // Connected inside this runtime on purpose: a tonic
                    // channel drives its connection from background tasks on
                    // whichever runtime built it, so a client built on the
                    // server's runtime would pull the worker's traffic right
                    // back onto the threads this is separating it from.
                    runtime.block_on(async move {
                        match casts::commit_store_client::connect().await {
                            Ok(client) => casts::archiver::run_forever(client, config).await,
                            Err(err) => {
                                log::error!("archival worker could not connect: {err:?}");
                            }
                        }
                    });
                })?;
        }
        None => log::info!("archival worker disabled (CASTS_ARCHIVE_BUCKET unset)"),
    }

    log::info!("Started server at {:?}", server_config.listen);
    let mut builder = Server::builder();
    if let Some(tls) = tls {
        builder = builder.tls_config(tls)?;
    }

    builder
        .add_service(health_service)
        .add_service(service)
        .serve(server_config.listen)
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    serve().await?;

    Ok(())
}
