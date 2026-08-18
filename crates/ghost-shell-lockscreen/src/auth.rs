//! PAM authentication support.
use std::ffi::{OsStr, OsString};

use nonstick::{
    AuthnFlags, ConversationAdapter, Result as PamResult, Transaction,
    TransactionBuilder,
};

const PAM_SERVICE: &str = "ghost-shell";

/// Supplies credentials to PAM conversation prompts.
struct PasswordConversation {
    username: OsString,
    password: OsString,
}

impl ConversationAdapter for PasswordConversation {
    fn prompt(&self, _request: impl AsRef<OsStr>) -> PamResult<OsString> {
        Ok(self.username.clone())
    }

    fn masked_prompt(
        &self,
        _request: impl AsRef<OsStr>,
    ) -> PamResult<OsString> {
        Ok(self.password.clone())
    }

    fn error_msg(&self, _message: impl AsRef<OsStr>) {}

    fn info_msg(&self, _message: impl AsRef<OsStr>) {}
}

pub fn username() -> OsString {
    std::env::var_os("USER").unwrap()
}

/// Authenticates `username` with `password` and validates the account.
///
/// # Errors
///
/// Returns a PAM error if the transaction cannot be created, authentication
/// fails, or account validation fails.
pub fn authenticate(username: &OsStr, password: &OsStr) -> PamResult<()> {
    let conversation = PasswordConversation {
        username: username.to_owned(),
        password: password.to_owned(),
    };

    let mut transaction = TransactionBuilder::new_with_service(PAM_SERVICE)
        .username(username)
        .build(conversation.into_conversation())?;

    transaction.authenticate(AuthnFlags::empty())?;
    transaction.account_management(AuthnFlags::empty())?;

    Ok(())
}
