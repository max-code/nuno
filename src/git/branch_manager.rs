use git2::{Branch, Repository};

pub struct BranchManager<'repo> {
    repo: &'repo Repository,
    pub local_branches: Vec<Branch<'repo>>,
}

impl<'repo> BranchManager<'repo> {
    pub fn new(repo: &'repo Repository) -> Result<Self, git2::Error> {
        let local_branches = repo
            .branches(Some(git2::BranchType::Local))?
            .filter_map(Result::ok)
            .map(|(branch, _)| branch)
            .collect();

        Ok(Self {
            repo,
            local_branches,
        })
    }

    pub fn refresh_branches(&mut self) -> Result<(), git2::Error> {
        self.local_branches = self
            .repo
            .branches(Some(git2::BranchType::Local))?
            .filter_map(Result::ok)
            .map(|(branch, _)| branch)
            .collect::<Vec<_>>();
        Ok(())
    }

    pub fn get_all_local_branch_names(&self) -> Result<Vec<String>, git2::Error> {
        Ok(self
            .local_branches
            .iter()
            .filter_map(|branch| branch.name().ok()?.map(String::from))
            .collect())
    }

    pub fn get_current_branch(&self) -> Result<String, git2::Error> {
        let head = self.repo.head()?;

        if head.is_branch() {
            Ok(head.shorthand().unwrap_or("HEAD").to_string())
        } else {
            // Detached head state
            let commit = head.peel_to_commit()?;
            Ok(commit.id().to_string())
        }
    }

    pub fn switch_to_branch(&self, branch: &Branch) -> Result<(), git2::Error> {
        let branch_name = branch
            .name()?
            .ok_or_else(|| git2::Error::from_str("Invalid UTF-8 in branch name."))?;

        let current_head_name = self.get_current_branch()?;

        let statuses = self.repo.statuses(None)?;
        if !statuses.is_empty() {
            return Err(git2::Error::from_str(&format!(
                "Uncommitted local changes on branch {}",
                current_head_name,
            )));
        }

        let mut opts = git2::build::CheckoutBuilder::new();

        self.repo.set_head(&format!("refs/heads/{}", branch_name))?;
        self.repo.checkout_head(Some(&mut opts))
    }

    pub fn fetch_on_branch(&self, branch: &Branch) -> Result<(), git2::Error> {
        let branch_name = branch
            .name()?
            .ok_or_else(|| git2::Error::from_str("Invalid UTF-8 in branch name"))?;

        let mut remote = self.repo.find_remote("origin")?;
        let refspec = format!(
            "+refs/heads/{}:refs/remotes/origin/{}",
            branch_name, branch_name
        );

        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.download_tags(git2::AutotagOption::None);

        remote.fetch(&[&refspec], Some(&mut fetch_options), None)?;

        Ok(())
    }
}
