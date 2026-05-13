//! 系统 keyring 封装。失败时优雅回退。
use keyring::Entry;

const SERVICE: &str = "typeless";

pub fn set(key: &str, value: &str) -> anyhow::Result<()> {
    let entry = Entry::new(SERVICE, key)?;
    entry.set_password(value)?;
    Ok(())
}

pub fn get(key: &str) -> Option<String> {
    Entry::new(SERVICE, key).ok().and_then(|e| e.get_password().ok())
}

pub fn delete(key: &str) -> anyhow::Result<()> {
    let entry = Entry::new(SERVICE, key)?;
    entry.delete_password().ok();
    Ok(())
}
