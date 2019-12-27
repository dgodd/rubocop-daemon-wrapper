use std::path::Path;

pub fn project_root(dir: &str) -> Option<String> {
    let mut path = Path::new(dir);
    loop {
        if path.join("Gemfile").exists() {
            return Some(path.to_str().unwrap().to_string());
        }
        path = path.parent()?;
    }
}

#[cfg(test)]
mod tests {
    extern crate tempdir;

    use crate::project_root::project_root;
    use std::fs::File;
    use std::io::Write;
    use tempdir::TempDir;

    #[test]
    fn pass_in_root() {
        let tmp_dir = TempDir::new("pass_in_subdir").unwrap();
        let mut tmp_file = File::create(tmp_dir.path().join("Gemfile")).unwrap();
        writeln!(tmp_file, "Context").unwrap();

        let root = project_root(tmp_dir.path().to_str().unwrap());
        assert_eq!(root.unwrap(), tmp_dir.path().to_str().unwrap());
    }

    #[test]
    fn pass_in_subdir() {
        let tmp_dir = TempDir::new("pass_in_subdir").unwrap();
        let sub_dir = tmp_dir.path().join("subdir");
        std::fs::create_dir(sub_dir.clone()).unwrap();
        let mut tmp_file = File::create(tmp_dir.path().join("Gemfile")).unwrap();
        writeln!(tmp_file, "Context").unwrap();

        let root = project_root(sub_dir.to_str().unwrap());
        assert_eq!(root.unwrap(), tmp_dir.path().to_str().unwrap());
    }

    #[test]
    fn dir_without_root() {
        let tmp_dir = TempDir::new("pass_in_subdir").unwrap();

        let root = project_root(tmp_dir.path().to_str().unwrap());
        assert_eq!(root, None)
    }
}
