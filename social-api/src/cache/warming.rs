use std::sync::Arc;

use crate::cache::LikeCache;
use crate::models::like::TimeWindow;
use crate::repositories::LikeRepository;

const WARM_TOP_N: u32 = 100;

/// Warm caches with hot data before server accepts traffic.
pub async fn warm_caches(repo: &Arc<dyn LikeRepository>, cache: &Arc<dyn LikeCache>) {
    tracing::info!("Starting cache warming...");

    // 1. Warm top liked content counts
    match repo.get_top_liked(None, TimeWindow::All, WARM_TOP_N).await {
        Ok(top_items) => {
            let count = top_items.len();
            for item in top_items {
                cache
                    .set_count(&item.content_type, item.content_id, item.count)
                    .await;
            }
            tracing::info!(items = count, "Warmed top liked content counts");
        }
        Err(e) => {
            tracing::warn!(error = %e, "Failed to warm top liked counts (non-fatal)");
        }
    }

    // 2. Warm per-content-type leaderboards
    for content_type in &["post", "bonus_hunter", "top_picks"] {
        match repo
            .get_top_liked(Some(content_type), TimeWindow::All, WARM_TOP_N)
            .await
        {
            Ok(items) => {
                for item in &items {
                    cache
                        .set_count(&item.content_type, item.content_id, item.count)
                        .await;
                }
                tracing::info!(
                    content_type = content_type,
                    items = items.len(),
                    "Warmed leaderboard cache"
                );
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    content_type = content_type,
                    "Failed to warm leaderboard (non-fatal)"
                );
            }
        }
    }

    tracing::info!("Cache warming complete");
}
