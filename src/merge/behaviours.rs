use std::path::PathBuf;

#[derive(Clone)]
pub struct RunBehaviours {
    pub output: bool,
    pub output_path: PathBuf,
    pub summary: bool,
    pub summary_path: PathBuf,
    pub show_repos_with_no_prs: bool,
    pub show_prs_from_untrusted_authors: bool,
    pub show_prs_with_unmatched_head: bool,
    pub execute: bool,
}
