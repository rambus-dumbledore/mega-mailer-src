use crate::storage::Storage;

#[derive(Debug)]
pub struct ImportanceChecker {
    important_emails: Vec<String>,
}

impl ImportanceChecker {
    pub fn new(storage: &Storage, username: &String) -> ImportanceChecker {
        let important_emails = storage.get_important_emails(username).unwrap_or(vec![]);
        ImportanceChecker{
            important_emails
        }
    }

    pub fn check(&self, email: &String) -> bool {
        self.important_emails.contains(email)
    }
}
