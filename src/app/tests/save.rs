use super::*;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = (std::fs::read_dir(dir).unwrap())
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn a_save_writes_the_file_in_place_through_its_symlink_and_keeps_its_inode() {
    let (path, mut a) = temp_file("save-in-place", "one\n");
    let dir = path.parent().unwrap().to_path_buf();
    let real = dir.join("real.py");
    std::fs::rename(&path, &real).unwrap();
    std::os::unix::fs::symlink("real.py", &path).unwrap();
    std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o750)).unwrap();
    let inode = std::fs::metadata(&real).unwrap().ino();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "x");
    a.save();
    assert_eq!((a.dirty, a.message.as_str()), (false, ""));
    assert!(path.is_symlink(), "the link stays a link");
    assert_eq!(std::fs::read_to_string(&real).unwrap(), "xone\n");
    let meta = std::fs::metadata(&real).unwrap();
    assert_eq!(
        meta.ino(),
        inode,
        "the same inode keeps its xattrs (Finder tags), ACL, creation date and bind mounts"
    );
    assert_eq!(meta.permissions().mode() & 0o777, 0o750);
    assert_eq!(
        names(&dir),
        ["f.py", "real.py"],
        "no temp file is left behind"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_write_that_fails_keeps_the_old_bytes_and_leaves_no_temp_file() {
    let (path, _) = temp_file("save-full-disk", "old\n");
    let dir = path.parent().unwrap().to_path_buf();
    let full = crate::real::write_after_probe(&path, b"new text\n", |file, bytes| {
        std::io::Write::write_all(file, &bytes[..3])?;
        Err(std::io::Error::new(
            std::io::ErrorKind::StorageFull,
            "No space left on device",
        ))
    });
    assert_eq!(full.unwrap_err().kind(), std::io::ErrorKind::StorageFull);
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "old\n",
        "a disk that fills mid-save leaves the file whole (#1532)"
    );
    assert_eq!(names(&dir), ["f.py"], "no temp file is left behind");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_symlink_planted_at_the_temp_name_is_never_written_through() {
    let (path, mut a) = temp_file("save-planted", "one\n");
    let dir = path.parent().unwrap().to_path_buf();
    let victim = dir.join("victim.txt");
    std::fs::write(&victim, "precious\n").unwrap();
    let planted = dir.join(format!(".f.py.merl-{}", std::process::id()));
    std::os::unix::fs::symlink(&victim, &planted).unwrap();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "x");
    a.save();
    assert_eq!((a.dirty, a.message.as_str()), (false, ""));
    assert_eq!(std::fs::read_to_string(&victim).unwrap(), "precious\n");
    assert!(
        !path.is_symlink(),
        "the file is not swapped for the planted link"
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "xone\n");
    assert!(planted.is_symlink());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_file_whose_temp_name_would_be_too_long_still_saves() {
    let (path, mut a) = temp_file("save-long-name", "one\n");
    let long = path.with_file_name(format!("{}.py", "n".repeat(247)));
    std::fs::rename(&path, &long).unwrap();
    a.buf = Buffer::load(&long).unwrap();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "x");
    a.save();
    assert_eq!((a.dirty, a.message.as_str()), (false, ""));
    assert_eq!(std::fs::read_to_string(&long).unwrap(), "xone\n");
    std::fs::remove_dir_all(long.parent().unwrap()).unwrap();
}

#[test]
fn a_file_in_a_folder_merl_cannot_write_to_saves_in_place() {
    let (path, mut a) = temp_file("save-locked-folder", "one\n");
    let dir = path.parent().unwrap().to_path_buf();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "x");
    a.save();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!((a.dirty, a.message.as_str()), (false, ""));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "xone\n");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_save_writes_a_hard_linked_file_in_place_for_every_name() {
    let (path, mut a) = temp_file("save-hard-link", "one\n");
    let twin = path.with_file_name("twin.py");
    std::fs::hard_link(&path, &twin).unwrap();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "x");
    a.save();
    assert_eq!(std::fs::read_to_string(&twin).unwrap(), "xone\n");
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}
