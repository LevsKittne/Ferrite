use ferrite_sdk::abi::Api;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Info {
    pub token: usize,
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub dependencies: String,
}

pub fn configure(_root: &Path, _api: *const Api) {}

pub fn is_managed(_file: &Path) -> bool {
    false
}

pub fn inspect(_file: &Path) -> Option<Info> {
    None
}

pub fn load(_token: usize, _handle: usize) -> bool {
    false
}

pub fn update(_token: usize) {}

pub fn scene(_token: usize, _name: &str) {}

pub fn config_changed(_token: usize) {}

pub fn unload(_token: usize) {}

pub fn release(_token: usize) {}
