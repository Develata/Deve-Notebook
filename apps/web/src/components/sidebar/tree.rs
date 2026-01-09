use deve_core::models::DocId;
use std::collections::BTreeMap;

// Tree Node Structure
#[derive(Clone, Debug, PartialEq)]
pub struct FileNode {
    pub id: Option<DocId>, // None for folders
    pub name: String,
    pub children: Vec<FileNode>,
    pub path: String, // Full path for creation context
}

// Internal helper for building tree
struct TempNode {
    id: Option<DocId>,
    children: BTreeMap<String, TempNode>,
}

pub fn build_file_tree(docs: Vec<(DocId, String)>) -> Vec<FileNode> {
    let mut root = TempNode { id: None, children: BTreeMap::new() };
    
    for (id, raw_path) in docs {
        let path = raw_path.replace("\\", "/");
        let parts: Vec<&str> = path.split('/').collect();
        let mut current = &mut root;
        
        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;
            
            // Navigate or Create
            current = current.children.entry(part.to_string()).or_insert_with(|| TempNode {
                id: None,
                children: BTreeMap::new(),
            });

            if is_last {
                current.id = Some(id);
            }
        }
    }
    
    // Recursive conversion
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
