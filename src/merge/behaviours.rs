use std::path::PathBuf;

#[derive(Clone)]
pub struct RunBehaviours {
    pub output_path: Option<PathBuf>,
    pub summary_path: Option<PathBuf>,
    pub skip_disqualifications_in_summary: bool,
    pub show_repos_with_no_prs: bool,
    pub show_prs_from_untrusted_authors: bool,
    pub show_prs_with_unmatched_head: bool,
    pub execute: bool,
    pub plain_stdout: bool,
}

#[cfg(test)]
impl RunBehaviours {
    pub(super) fn default() -> Self {
        Self {
            output_path: None,
            summary_path: None,
            skip_disqualifications_in_summary: false,
            show_repos_with_no_prs: false,
            show_prs_from_untrusted_authors: false,
            show_prs_with_unmatched_head: false,
            execute: false,
            plain_stdout: true,
        }
    }

    pub(super) fn show_prs_with_unmatched_head(mut self) -> Self {
        self.show_prs_with_unmatched_head = true;
        self
    }

    pub(super) fn show_prs_from_untrusted_authors(mut self) -> Self {
        self.show_prs_from_untrusted_authors = true;
        self
    }

    pub(super) fn skip_disqualifications_in_summary(mut self) -> Self {
        self.skip_disqualifications_in_summary = true;
        self
    }
}
