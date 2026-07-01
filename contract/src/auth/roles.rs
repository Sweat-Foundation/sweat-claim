use near_plugins::AccessControlRole;

#[derive(AccessControlRole, Copy, Clone)]
pub enum Roles {
    Oracle,
    BurnManager,
    Maintainer,
}
