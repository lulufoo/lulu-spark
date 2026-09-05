pub mod roots;

#[cfg(test)]
#[path = "../unit-tests/config/read_api_acl_contract.rs"]
mod read_api_acl_contract;

#[cfg(test)]
#[path = "../unit-tests/config/write_api_acl_contract.rs"]
mod write_api_acl_contract;

#[cfg(test)]
#[path = "../unit-tests/config/dialog_plugin_contract.rs"]
mod dialog_plugin_contract;

pub mod paths;
pub mod secrets;
pub mod settings;