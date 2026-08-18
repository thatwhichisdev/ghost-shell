use std::ffi::{OsStr, OsString};

use nonstick::{
    AuthnFlags, ConversationAdapter, Result as PamResult, Transaction,
    TransactionBuilder,
};

const PAM_SERVICE: &str = "ghost-shell";

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
