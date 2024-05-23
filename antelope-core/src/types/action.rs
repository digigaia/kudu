use crate::types::{AccountName, ActionName, PermissionName};

#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone)]
pub struct PermissionLevel {
    actor: AccountName,
    permission: PermissionName,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct Action {
    account: AccountName,
    name: ActionName,
    auth: Vec<PermissionLevel>,
    data: Vec<u8>,
}
