use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct RepositoryOwner {
    login: String,
    id: u64,
    node_id: String,
    avatar_url: String,
    gravatar_id: String,
    url: String,
    html_url: String,
    followers_url: String,
    following_url: String,
    gists_url: String,
    starred_url: String,
    subscriptions_url: String,
    organizations_url: String,
    repos_url: String,
    events_url: String,
    received_events_url: String,
    r#type: String,
    user_view_type: String,
    site_admin: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RepositoryPermissions {
    admin: bool,
    maintain: bool,
    push: bool,
    triage: bool,
    pull: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RepositoryLicense {
    key: String,
    name: String,
    spdx_id: String,
    url: Option<String>,
    node_id: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Repository {
    id: u64,
    node_id: String,
    pub name: String,
    full_name: String,
    private: bool,
    owner: RepositoryOwner,
    html_url: String,
    description: Option<String>,
    fork: bool,
    pub url: String,
    forks_url: String,
    keys_url: String,
    collaborators_url: String,
    teams_url: String,
    hooks_url: String,
    issue_events_url: String,
    events_url: String,
    assignees_url: String,
    branches_url: String,
    tags_url: String,
    blobs_url: String,
    git_tags_url: String,
    git_refs_url: String,
    trees_url: String,
    statuses_url: String,
    languages_url: String,
    stargazers_url: String,
    contributors_url: String,
    subscribers_url: String,
    subscription_url: String,
    commits_url: String,
    git_commits_url: String,
    comments_url: String,
    issue_comment_url: String,
    contents_url: String,
    compare_url: String,
    merges_url: String,
    archive_url: String,
    downloads_url: String,
    issues_url: String,
    pulls_url: String,
    milestones_url: String,
    notifications_url: String,
    labels_url: String,
    releases_url: String,
    deployments_url: String,
    pub created_at: String, // TODO: DateTime<Utc>
    updated_at: String, // TODO: DateTime<Utc>
    pub pushed_at: String,  // TODO: DateTime<Utc>
    git_url: String,
    ssh_url: String,
    clone_url: String,
    svn_url: String,
    homepage: Option<String>,
    pub size: u64,
    pub stargazers_count: u64,
    pub watchers_count: u64,
    pub language: Option<String>,
    has_issues: bool,
    has_projects: bool,
    has_downloads: bool,
    has_wiki: bool,
    has_pages: bool,
    has_discussions: bool,
    pub forks_count: u64,
    mirror_url: Option<String>,
    archived: bool,
    disabled: bool,
    open_issues_count: u64,
    license: Option<RepositoryLicense>,
    allow_forking: bool,
    is_template: bool,
    web_commit_signoff_required: bool,
    topics: Vec<String>,
    visibility: String,
    forks: u64,
    open_issues: u64,
    watchers: u64,
    default_branch: String,
    permissions: RepositoryPermissions,
}

impl Repository {
    pub fn importance_score(&self) -> u64 {
        if self.archived || self.fork {
            return 0;
        }

        let stars = self.stargazers_count.min(1000);
        let forks = self.forks_count.min(100);
        let size = self.size.min(10000) / 100;

        (stars * 3) + (forks * 2) + size
    }
}
