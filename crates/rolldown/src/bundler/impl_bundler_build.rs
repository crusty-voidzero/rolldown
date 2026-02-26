use crate::types::bundle_output::BundleOutput;
use anyhow::Result;
use arcstr::ArcStr;
use rolldown_common::{BundleMode, ScanMode};
use rolldown_error::BuildResult;

use super::bundler::Bundler;

impl Bundler {
  #[tracing::instrument(level = "debug", skip_all, parent = &self.session.span)]
  pub async fn write(&mut self) -> BuildResult<BundleOutput> {
    self.create_error_if_closed()?;
    // TODO: hyf0: Bad code smell: this overlaps with `incremental_write/xxx` APIs.
    #[cfg(feature = "experimental")]
    if self.options.experimental.is_incremental_build_enabled() {
      return self.incremental_write(ScanMode::Full).await;
    }
    let bundle = self.bundle_factory.create_bundle(BundleMode::FullBuild, None)?;
    bundle.write().await
  }

  /// Build with a callback after scan (before render/write).
  ///
  /// Allows watchers to register FS watches for files discovered during scan,
  /// so that changes made during render hooks (e.g. `renderStart` modifying a file)
  /// are detected.
  #[tracing::instrument(level = "debug", skip_all, parent = &self.session.span)]
  pub async fn write_for_watch(
    &mut self,
    skip_write: bool,
    on_scan_complete: impl FnOnce(Vec<ArcStr>) -> BuildResult<()> + Send,
  ) -> BuildResult<BundleOutput> {
    self.create_error_if_closed()?;
    let mut bundle = self.bundle_factory.create_bundle(BundleMode::FullBuild, None)?;

    // Phase 1: Scan
    let scan_result = bundle.scan_modules(ScanMode::Full).await;

    // Phase 2: Notify caller of discovered watch files BEFORE checking scan errors
    // (so files are watched even on error — enables recovery when user fixes the issue)
    let watch_files: Vec<ArcStr> = bundle.get_watch_files().iter().map(|f| f.clone()).collect();
    on_scan_complete(watch_files)?;

    // Phase 3: Check scan result
    let scan_output = scan_result?;

    // Phase 4: Write or Generate
    if skip_write {
      bundle.bundle_generate(scan_output).await
    } else {
      bundle.bundle_write(scan_output).await
    }
  }

  #[tracing::instrument(level = "debug", skip_all, parent = &self.session.span)]
  pub async fn generate(&mut self) -> BuildResult<BundleOutput> {
    self.create_error_if_closed()?;
    #[cfg(feature = "experimental")]
    if self.options.experimental.is_incremental_build_enabled() {
      return self.incremental_generate(ScanMode::Full).await;
    }
    let bundle = self.bundle_factory.create_bundle(BundleMode::FullBuild, None)?;
    bundle.generate().await
  }

  #[tracing::instrument(target = "devtool", level = "debug", skip_all)]
  #[cfg(feature = "experimental")]
  pub async fn scan(&mut self) -> BuildResult<()> {
    self.create_error_if_closed()?;

    let bundle = self.bundle_factory.create_bundle(BundleMode::FullBuild, None)?;
    bundle.scan().await?;
    Ok(())
  }

  #[tracing::instrument(level = "debug", skip_all)]
  pub async fn close(&mut self) -> Result<()> {
    self.inner_close().await
  }
}
