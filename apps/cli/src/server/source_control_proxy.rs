// apps/cli/src/server/source_control_proxy.rs
//! # Source Control Remote Proxy

use anyhow::Result;
use deve_core::ledger::traits::Repository;
use deve_core::models::DocId;
use deve_core::source_control::{ChangeEntry, CommitInfo};

pub struct RemoteSourceControlApi {
    base_url: String,
    client: reqwest::Client,
}

impl RemoteSourceControlApi {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

/// 在异步上下文中安全执行阻塞 HTTP 请求
///
/// # 不变量
/// - 使用 `block_in_place` 避免在 Tokio 工作线程上死锁
fn block_on_safe<F, T>(f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(f))
}

impl Repository for RemoteSourceControlApi {
    fn list_docs(&self) -> Result<Vec<(DocId, String)>> {
        let url = format!("{}/api/repo/docs", self.base_url);
        let res = block_on_safe(async {
            self.client
                .get(&url)
                .send()
                .await?
                .json::<Vec<(DocId, String)>>()
                .await
        })?;
        Ok(res)
    }

    fn get_doc_content(&self, doc_id: DocId) -> Result<String> {
        let url = format!("{}/api/repo/doc", self.base_url);
        let res = block_on_safe(async {
            self.client
                .get(&url)
                .query(&[("doc_id", doc_id.to_string())])
                .send()
                .await?
                .text()
                .await
        })?;
        Ok(res)
    }

    fn list_changes(&self) -> Result<Vec<ChangeEntry>> {
        let url = format!("{}/api/sc/status", self.base_url);
        let res = block_on_safe(async {
            self.client
                .get(&url)
                .send()
                .await?
                .json::<Vec<ChangeEntry>>()
                .await
        })?;
        Ok(res)
    }

    fn diff_doc_path(&self, path: &str) -> Result<String> {
        let url = format!("{}/api/sc/diff", self.base_url);
        let res = block_on_safe(async {
            self.client
                .get(&url)
                .query(&[("path", path)])
                .send()
                .await?
                .text()
                .await
        })?;
        Ok(res)
    }

    fn stage_file(&self, path: &str) -> Result<()> {
        let url = format!("{}/api/sc/stage", self.base_url);
        block_on_safe(async {
            self.client
                .post(&url)
                .json(&serde_json::json!({"path": path}))
                .send()
                .await?
                .error_for_status()?;
            Ok::<(), reqwest::Error>(())
        })?;
        Ok(())
    }

    fn commit_staged(&self, message: &str) -> Result<CommitInfo> {
        let url = format!("{}/api/sc/commit", self.base_url);
        let res = block_on_safe(async {
            self.client
                .post(&url)
                .json(&serde_json::json!({"message": message}))
                .send()
                .await?
                .error_for_status()?
                .json::<CommitInfo>()
                .await
        })?;
        Ok(res)
    }
}
