// crates\core\src\search
//! # Search Service (搜索服务)
//!
//! **架构作用**:
//! 基于 Tantivy 的全文检索引擎，为 Standard Profile 提供高级搜索能力。
//!
//! **核心功能清单**:
//! - `SearchService`: 管理 Tantivy 索引。
//!   - `index_document(doc_id, path, content)`: 索引文档。
//!   - `delete_document(doc_id)`: 从索引中删除文档。
//!   - `search(query, limit)`: 执行搜索查询。
//!
//! **类型**: Plugin MAY (插件可选) - 仅 Standard Profile 启用

use std::path::Path;
use std::sync::Mutex;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Schema, STORED, STRING, TEXT, Field, Value};
use tantivy::{doc, Index, IndexWriter, ReloadPolicy, TantivyDocument};
use crate::models::DocId;

/// Search result entry
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub doc_id: String, // UUID as string
    pub path: String,
    pub score: f32,
}

/// Tantivy-based full-text search service
pub struct SearchService {
    index: Index,
    writer: Mutex<IndexWriter>,
    schema: Schema,
    field_doc_id: Field,
    field_path: Field,
    field_content: Field,
}

impl SearchService {
    /// Create a new in-memory search service
    pub fn new_in_memory() -> anyhow::Result<Self> {
        let mut schema_builder = Schema::builder();
        // Store doc_id as STRING (exact, stored)
        let field_doc_id = schema_builder.add_text_field("doc_id", STRING | STORED);
        let field_path = schema_builder.add_text_field("path", TEXT | STORED);
        let field_content = schema_builder.add_text_field("content", TEXT);
        let schema = schema_builder.build();

        let index = Index::create_in_ram(schema.clone());
        let writer = index.writer(50_000_000)?; // 50MB heap

        Ok(Self {
            index,
            writer: Mutex::new(writer),
            schema,
            field_doc_id,
            field_path,
            field_content,
        })
    }

    /// Create a search service backed by a directory
    pub fn new_on_disk(index_path: &Path) -> anyhow::Result<Self> {
        let mut schema_builder = Schema::builder();
        let field_doc_id = schema_builder.add_text_field("doc_id", STRING | STORED);
        let field_path = schema_builder.add_text_field("path", TEXT | STORED);
        let field_content = schema_builder.add_text_field("content", TEXT);
        let schema = schema_builder.build();

        std::fs::create_dir_all(index_path)?;
        let index = Index::create_in_dir(index_path, schema.clone())
            .or_else(|_| Index::open_in_dir(index_path))?;
        let writer = index.writer(50_000_000)?;

        Ok(Self {
            index,
            writer: Mutex::new(writer),
            schema,
            field_doc_id,
            field_path,
            field_content,
        })
    }

    /// Index a document
    pub fn index_document(&self, doc_id: DocId, path: &str, content: &str) -> anyhow::Result<()> {
        let mut writer = self.writer.lock().unwrap();
        let doc_id_str = doc_id.to_string();
        
        // Delete existing document first (upsert)
        writer.delete_term(tantivy::Term::from_field_text(self.field_doc_id, &doc_id_str));
        
        writer.add_document(doc!(
            self.field_doc_id => doc_id_str,
            self.field_path => path,
            self.field_content => content,
        ))?;
        
        writer.commit()?;
        Ok(())
    }

    /// Delete a document from the index
    pub fn delete_document(&self, doc_id: DocId) -> anyhow::Result<()> {
        let mut writer = self.writer.lock().unwrap();
        let doc_id_str = doc_id.to_string();
        writer.delete_term(tantivy::Term::from_field_text(self.field_doc_id, &doc_id_str));
        writer.commit()?;
        Ok(())
    }

    /// Search for documents
    pub fn search(&self, query: &str, limit: usize) -> anyhow::Result<Vec<SearchResult>> {
        let reader = self.index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        let searcher = reader.searcher();
        
        let query_parser = QueryParser::for_index(&self.index, vec![self.field_path, self.field_content]);
        let query = query_parser.parse_query(query)?;
        
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;
        
        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
            
            let doc_id = retrieved_doc
                .get_first(self.field_doc_id)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default();
            
            let path = retrieved_doc
                .get_first(self.field_path)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default();
            
            results.push(SearchResult {
                doc_id,
                path,
                score,
            });
        }
        
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_service() -> anyhow::Result<()> {
        let service = SearchService::new_in_memory()?;
        
        // Index some documents
        let doc1 = DocId::new();
        let doc2 = DocId::new();
        let doc3 = DocId::new();
        
        service.index_document(doc1, "docs/hello.md", "Hello World")?;
        service.index_document(doc2, "docs/rust.md", "Rust programming language")?;
        service.index_document(doc3, "notes/todo.md", "Buy groceries and call mom")?;
        
        // Search
        let results = service.search("hello", 10)?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].doc_id, doc1.to_string());
        
        let results = service.search("rust", 10)?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].doc_id, doc2.to_string());
        
        // Search by path
        let results = service.search("docs", 10)?;
        assert_eq!(results.len(), 2);
        
        Ok(())
    }
}
