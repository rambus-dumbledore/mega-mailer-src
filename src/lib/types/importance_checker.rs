use crate::storage::Storage;

#[derive(Debug)]
pub struct ImportanceChecker {
    important_emails: Vec<String>,
    tags: Vec<String>,
}

impl ImportanceChecker {
    pub fn new(storage: &Storage, username: &String) -> ImportanceChecker {
        let important_emails = storage.get_important_emails(username).unwrap_or(vec![]);
        let tags = storage.get_important_tags(username).unwrap_or(vec![]);
        ImportanceChecker {
            important_emails,
            tags,
        }
    }

    pub fn check(&self, email: &String, subject: &String) -> bool {
        let contain_important_email = self.important_emails.contains(email);
        let contain_important_tag = self.tags.iter().any(|tag| subject.contains(tag));
        contain_important_email || contain_important_tag
    }
}
