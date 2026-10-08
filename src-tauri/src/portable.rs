use std::path::{Path, PathBuf};
pub fn data_directory(executable: &Path) -> Result<PathBuf, String> {
    executable
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|parent| parent.join("data"))
        .ok_or_else(|| "无法确定程序所在目录".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn storage_is_next_to_executable_not_working_directory() {
        assert_eq!(
            data_directory(Path::new("D:/Portable/LocalTodo/local-todo.exe")).unwrap(),
            PathBuf::from("D:/Portable/LocalTodo/data")
        );
        assert!(data_directory(Path::new("local-todo.exe")).is_err());
    }
}
