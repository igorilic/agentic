#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub remote_url: Option<String>,
    pub profile: String,
    pub created_at: i64,
    pub last_opened: i64,
}

#[derive(Clone)]
pub struct WorkspaceRepo;
