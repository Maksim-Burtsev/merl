use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use tokio::sync::Mutex;
use tokio::time::Instant;

use crate::TokenBucket;

/// Keeps buckets between requests. A `get` and the `set` after it are not atomic,
/// so two requests at once may both spend the same token.
#[async_trait]
pub trait BucketStore: Send + Sync {
    async fn get(&self, key: &str) -> anyhow::Result<Option<TokenBucket>>;
    async fn set(&self, key: &str, bucket: &TokenBucket, ttl: Duration) -> anyhow::Result<()>;
}

/// Keeps buckets in a map with Redis-like expiry. For tests and local runs.
#[derive(Default)]
pub struct MemoryStore {
    items: Mutex<HashMap<String, (TokenBucket, Instant)>>,
}

#[async_trait]
impl BucketStore for MemoryStore {
    async fn get(&self, key: &str) -> anyhow::Result<Option<TokenBucket>> {
        let mut items = self.items.lock().await;
        match items.get(key) {
            Some((bucket, expires_at)) if *expires_at > Instant::now() => Ok(Some(bucket.clone())),
            Some(_) => {
                items.remove(key);
                Ok(None)
            }
            None => Ok(None),
        }
    }

    async fn set(&self, key: &str, bucket: &TokenBucket, ttl: Duration) -> anyhow::Result<()> {
        let expires_at = Instant::now() + ttl;
        self.items.lock().await.insert(key.to_owned(), (bucket.clone(), expires_at));
        Ok(())
    }
}

pub struct RedisStore {
    pub conn: ConnectionManager,
    pub prefix: String,
}

#[async_trait]
impl BucketStore for RedisStore {
    async fn get(&self, key: &str) -> anyhow::Result<Option<TokenBucket>> {
        let fields: HashMap<String, f64> = self.conn.clone().hgetall(format!("{}{key}", self.prefix)).await?;
        if fields.is_empty() {
            return Ok(None);
        }
        TokenBucket::from_fields(&fields).map(Some)
    }

    async fn set(&self, key: &str, bucket: &TokenBucket, ttl: Duration) -> anyhow::Result<()> {
        let name = format!("{}{key}", self.prefix);
        redis::pipe()
            .atomic()
            .hset_multiple(&name, &bucket.to_fields())
            .expire(&name, ttl.as_secs() as i64)
            .query_async::<()>(&mut self.conn.clone())
            .await?;
        Ok(())
    }
}
