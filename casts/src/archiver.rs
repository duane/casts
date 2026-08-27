//! The archival worker: download an entry's audio, put it in S3, record it,
//! delete the local copy.
//!
//! Runs inside `casts-grpc-server` so a single deployment serves gRPC and
//! archives in the background.

use anyhow::{anyhow, Context, Result};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use commit_store::grpc_interface::commit_store_client::CommitStoreClient;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tonic::transport::Channel;
use uuid::Uuid;

use crate::poller::{archive_key, hydrate, pending_archives, record_archived, refresh_feeds};

/// Worker configuration, read from the environment.
///
/// The worker is off unless `CASTS_ARCHIVE_BUCKET` is set, so it is opt-in
/// per deployment and local runs are unaffected.
#[derive(Clone, Debug)]
pub struct ArchiverConfig {
    pub bucket: String,
    pub stream_id: Uuid,
    pub poll_interval: Duration,
    /// Entries archived per pass. Bounded because a first run over a full
    /// podcast back-catalogue is hundreds of gigabytes, and the task's
    /// writable layer is finite.
    pub batch: usize,
    /// Anything larger is skipped rather than filling the disk.
    pub max_bytes: u64,
    pub tmpdir: PathBuf,
}

impl ArchiverConfig {
    /// `None` when no bucket is configured, which disables the worker.
    pub fn from_env(stream_id: Uuid) -> Option<Self> {
        let bucket = std::env::var("CASTS_ARCHIVE_BUCKET").ok()?;
        let env_num = |name: &str, default: u64| -> u64 {
            std::env::var(name)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default)
        };
        Some(ArchiverConfig {
            bucket,
            stream_id,
            poll_interval: Duration::from_secs(env_num("CASTS_POLL_INTERVAL_SECS", 900)),
            batch: env_num("CASTS_ARCHIVE_BATCH", 5) as usize,
            max_bytes: env_num("CASTS_ARCHIVE_MAX_BYTES", 500 * 1024 * 1024),
            tmpdir: std::env::var("CASTS_ARCHIVE_TMPDIR")
                .unwrap_or_else(|_| "/tmp/casts-archive".to_string())
                .into(),
        })
    }
}

/// Download `url` to `dest`, refusing anything over `max_bytes`.
///
/// Streams to disk rather than buffering: podcast episodes run to hundreds of
/// megabytes and holding one in memory inside a 1 GB task is how the worker
/// would take the server down with it.
async fn download_to(url: &url::Url, dest: &Path, max_bytes: u64) -> Result<u64> {
    // A bare User-Agent-less GET is rejected outright (400) by at least one
    // widely-used podcast redirector (podtrac.com, fronting History of
    // Rome's audio among others) -- confirmed live, not a guess. `reqwest`
    // sends no default User-Agent, so every entry behind such a redirector
    // failed every pass until this was added.
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    let client = CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(concat!("casts-archiver/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build the download HTTP client")
    });

    let response = client
        .get(url.clone())
        .send()
        .await
        .with_context(|| format!("fetching {url}"))?;
    let status = response.status();
    let response = response
        .error_for_status()
        .with_context(|| format!("fetching {url}: server returned {status}"))?;

    // Trust Content-Length only to reject early; it can lie, so the running
    // total below is what actually enforces the cap.
    if let Some(len) = response.content_length() {
        if len > max_bytes {
            return Err(anyhow!("{url} is {len} bytes, over the {max_bytes} limit"));
        }
    }

    let mut file = tokio::fs::File::create(dest)
        .await
        .with_context(|| format!("creating {}", dest.display()))?;
    let mut written: u64 = 0;
    let mut stream = response.bytes_stream();
    use futures::StreamExt;
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.with_context(|| format!("reading body of {url}"))?;
        written += chunk.len() as u64;
        if written > max_bytes {
            let _ = tokio::fs::remove_file(dest).await;
            return Err(anyhow!(
                "{url} exceeded the {max_bytes} byte limit mid-stream"
            ));
        }
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    Ok(written)
}

/// One archival pass: refresh feeds, then archive up to `batch` entries.
///
/// Returns how many entries were archived.
pub async fn run_once(
    client: &mut CommitStoreClient<Channel>,
    s3: &S3Client,
    config: &ArchiverConfig,
) -> Result<usize> {
    let report = refresh_feeds(client, &config.stream_id).await?;
    if !report.ok() {
        for (feed_id, reason) in &report.failures {
            log::warn!("feed {feed_id} could not be refreshed: {reason}");
        }
    }
    log::info!(
        "refresh: {} changed, {} unchanged, {} failed",
        report.changed,
        report.unchanged,
        report.failures.len()
    );

    // The worker is the natural snapshotter: it already runs on a timer, and
    // it is the main source of the commits that make hydration expensive.
    // A failure here is logged, not propagated -- a missing snapshot only
    // makes reads slower, and must not stop the archival pass.
    match crate::poller::snapshot_if_due(client, &config.stream_id).await {
        Ok(true) => log::info!("snapshot written"),
        Ok(false) => log::debug!("no snapshot needed this pass"),
        Err(err) => log::warn!("could not write a snapshot: {err}"),
    }

    tokio::fs::create_dir_all(&config.tmpdir).await.ok();
    let aggregate = hydrate(client, &config.stream_id).await?;
    let pending = pending_archives(&aggregate, config.batch);
    if pending.is_empty() {
        return Ok(0);
    }
    log::info!("{} entries pending archive this pass", pending.len());

    let mut archived = 0usize;
    for item in pending {
        let key = archive_key(&config.stream_id, &item.feed_id, &item.entry_guid);
        let dest = config.tmpdir.join(format!("{archived}.part"));

        // Order matters: upload must be confirmed before the event is
        // emitted, and the event before the local delete. A crash anywhere
        // leaves the work retryable rather than claiming an archive that
        // does not exist.
        let size = match download_to(&item.audio_url, &dest, config.max_bytes).await {
            Ok(size) => size,
            Err(err) => {
                // {err:?} walks the anyhow context chain (server status,
                // then the URL); {err} alone prints only the outermost
                // context and silently drops the actual cause -- which is
                // exactly what made the podtrac 400s above look like an
                // unexplained transient failure instead of an obvious one.
                log::warn!("skipping {}: {err:?}", item.entry_guid);
                let _ = tokio::fs::remove_file(&dest).await;
                continue;
            }
        };

        let body = match ByteStream::from_path(&dest).await {
            Ok(body) => body,
            Err(err) => {
                log::warn!("could not read {}: {err}", dest.display());
                let _ = tokio::fs::remove_file(&dest).await;
                continue;
            }
        };
        let mut put = s3.put_object().bucket(&config.bucket).key(&key).body(body);
        if let Some(ref content_type) = item.content_type {
            put = put.content_type(content_type);
        }
        if let Err(err) = put.send().await {
            log::warn!("upload of {} failed: {err}", item.entry_guid);
            let _ = tokio::fs::remove_file(&dest).await;
            continue;
        }

        match record_archived(
            client,
            &config.stream_id,
            &item,
            &config.bucket,
            &key,
            size as i64,
        )
        .await
        {
            Ok(true) => {
                archived += 1;
                log::info!(
                    "archived {} -> s3://{}/{key}",
                    item.entry_guid,
                    config.bucket
                );
            }
            // Already recorded: another worker won the race, or this is a
            // retry after a crash between upload and commit. The object is
            // in place either way.
            Ok(false) => log::info!("{} was already archived", item.entry_guid),
            Err(err) => log::warn!("could not record archive for {}: {err}", item.entry_guid),
        }

        let _ = tokio::fs::remove_file(&dest).await;
    }
    Ok(archived)
}

/// Run passes forever, spaced by `poll_interval`.
///
/// Never returns: a failed pass is logged and retried next tick rather than
/// ending the task. The gRPC service keeps serving regardless, which is why
/// the health check deliberately does not cover this worker -- a podcast host
/// being unreachable must not cause ECS to replace a healthy server.
pub async fn run_forever(mut client: CommitStoreClient<Channel>, config: ArchiverConfig) {
    // Left-over scratch from a previous task that died mid-download.
    let _ = tokio::fs::remove_dir_all(&config.tmpdir).await;

    let aws = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let s3 = S3Client::new(&aws);
    log::info!(
        "archival worker started: bucket={} interval={}s batch={}",
        config.bucket,
        config.poll_interval.as_secs(),
        config.batch
    );

    let mut ticker = tokio::time::interval(config.poll_interval);
    loop {
        let _ = ticker.tick().await;
        match run_once(&mut client, &s3, &config).await {
            Ok(0) => log::debug!("archival pass: nothing to do"),
            Ok(n) => log::info!("archival pass: {n} entries archived"),
            Err(err) => log::warn!("archival pass failed, retrying next tick: {err}"),
        }
    }
}
