//! # 文件树组件逻辑 (File Tree Logic)
//! 
//! **架构作用**:
//! 将扁平的文档列表 (path string) 转换为层级化的树形结构 (`FileNode`)，供 Sidebar 使用。
//! 

use deve_core::models::DocId;
use std::collections::BTreeMap;

/// 树节点结构
/// 代表侧边栏树中的一个文件或文件夹。
#[derive(Clone, Debug, PartialEq)]
pub struct FileNode {
    /// 文件夹为 None，文件为 Some(DocId)
    pub id: Option<DocId>, 
    pub name: String,
    pub children: Vec<FileNode>,
    /// 完整的路径，用于创建上下文和验证
    pub path: String, 
}

// 构建树的内部辅助结构
struct TempNode {
    id: Option<DocId>,
    children: BTreeMap<String, TempNode>,
}

/// 构建文件树
/// 
/// **逻辑**:
/// 1. 遍历扁平的 (DocId, Path) 列表。
/// 2. 按 `/` 分割路径。
/// 3. 插入到中间的 `TempNode` 树中 (使用 BTreeMap 进行自动排序)。
/// 4. 递归地将 `TempNode` 转换为 `FileNode` 列表。
pub fn build_file_tree(docs: Vec<(DocId, String)>) -> Vec<FileNode> {
    let mut root = TempNode { id: None, children: BTreeMap::new() };
    
    for (id, raw_path) in docs {
        let path = raw_path.replace("\\", "/");
        let parts: Vec<&str> = path.split('/').collect();
        let mut current = &mut root;
        
        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;
            
            // 导航或创建
            current = current.children.entry(part.to_string()).or_insert_with(|| TempNode {
                id: None,
                children: BTreeMap::new(),
            });

            if is_last {
                current.id = Some(id);
            }
        }
    }
    
    // 递归转换
    fn convert(name: String, node: TempNode, path_prefix: String) -> FileNode {
        let full_path = if path_prefix.is_empty() { name.clone() } else { format!("{}/{}", path_prefix, name) };
        let children: Vec<FileNode> = node.children.into_iter()
            .map(|(n, c)| convert(n, c, full_path.clone()))
            .collect();
        
        FileNode {
            id: node.id,
            name,
            children,
            path: full_path,
        }
    }
    
    root.children.into_iter()
        .map(|(n, c)| convert(n, c, "".to_string()))
        .collect()
}
