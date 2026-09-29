// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

#[cfg(test)]
mod directory_integration_tests {
    use std::fs::{self, File};
    use std::io::{Read, Write};
    use std::path::Path;
    use tempfile::TempDir;

    use crate::PatchSet;
    use crate::apply::DirPatchApplier;
    use crate::dir_diff_scanner::DirDiffScanner;

    fn create_file(root: &Path, relative_path: &str, content: &[u8]) {
        let full_path = root.join(relative_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = File::create(full_path).unwrap();
        file.write_all(content).unwrap();
    }

    fn read_file(root: &Path, relative_path: &str) -> Vec<u8> {
        let mut buffer = Vec::new();
        let mut file = File::open(root.join(relative_path)).unwrap();
        file.read_to_end(&mut buffer).unwrap();
        buffer
    }

    #[test]
    fn test_full_workspace_diff_patch_and_rollback_lifecycle() {
        let before_workspace = TempDir::new().unwrap();
        let after_workspace = TempDir::new().unwrap();
        let target_workspace = TempDir::new().unwrap();

        // Using context = 1 so a simple 3-line file has matching hooks
        let context_lines = 1;

        // Establish State A (Before)
        create_file(
            before_workspace.path(),
            "src/main.rs",
            b"A\nB\nC\nD\nE\nF\nG\n",
        );
        create_file(
            before_workspace.path(),
            "docs/readme.txt",
            b"Initial documentation text without newline",
        );
        create_file(
            before_workspace.path(),
            "legacy/old_code.rs",
            b"// To be removed\n",
        );
        create_file(
            before_workspace.path(),
            "assets/logo.png",
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
        );

        // Build an identical Target Sandbox to apply changes onto
        create_file(
            target_workspace.path(),
            "src/main.rs",
            b"A\nB\nC\nD\nE\nF\nG\n",
        );
        create_file(
            target_workspace.path(),
            "docs/readme.txt",
            b"Initial documentation text without newline",
        );
        create_file(
            target_workspace.path(),
            "legacy/old_code.rs",
            b"// To be removed\n",
        );
        create_file(
            target_workspace.path(),
            "assets/logo.png",
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
        );

        // Establish State B (After Modifications)
        create_file(
            after_workspace.path(),
            "src/main.rs",
            b"A\nBb\nC\nD\nE\nF\nG\n",
        );
        create_file(
            after_workspace.path(),
            "docs/readme.txt",
            b"Updated documentation text without newline",
        );
        create_file(
            after_workspace.path(),
            "assets/logo.png",
            &[0x7F, 0x45, 0x4C, 0x46, 0x02, 0x01, 0x01, 0x00],
        );
        create_file(
            after_workspace.path(),
            "src/utils/math.rs",
            b"pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
        );

        // 1. Scan and compute patch array maps
        let patch_set = DirDiffScanner::compare_directories(
            before_workspace.path(),
            after_workspace.path(),
            context_lines,
        )
        .unwrap();

        // 2. Validate Serde I/O stream encoding boundaries
        let mut json_buffer = Vec::new();
        patch_set.to_writer(&mut json_buffer, true).unwrap();
        let serialized_patch: PatchSet = PatchSet::from_reader(&json_buffer[..]).unwrap();

        // 3. Execute patch forward onto Target Sandbox
        DirPatchApplier::apply_patch_set(target_workspace.path(), &serialized_patch, false)
            .unwrap();

        // 4. Verification Check: Target must now match State B perfectly
        assert_eq!(
            read_file(target_workspace.path(), "src/main.rs"),
            b"A\nBb\nC\nD\nE\nF\nG\n"
        );
        assert_eq!(
            read_file(target_workspace.path(), "docs/readme.txt"),
            b"Updated documentation text without newline"
        );
        assert_eq!(
            read_file(target_workspace.path(), "assets/logo.png"),
            &[0x7F, 0x45, 0x4C, 0x46, 0x02, 0x01, 0x01, 0x00]
        );
        assert_eq!(
            read_file(target_workspace.path(), "src/utils/math.rs"),
            b"pub fn add(a: i32, b: i32) -> i32 { a + b }\n"
        );
        assert!(!target_workspace.path().join("legacy/old_code.rs").exists());

        // 5. Rollback Changes (Reverse Application Mode)
        DirPatchApplier::apply_patch_set(target_workspace.path(), &serialized_patch, true).unwrap();

        // 6. Verification Check: Target must be back to State A perfectly
        assert_eq!(
            read_file(target_workspace.path(), "src/main.rs"),
            b"A\nB\nC\nD\nE\nF\nG\n"
        );
        assert_eq!(
            read_file(target_workspace.path(), "docs/readme.txt"),
            b"Initial documentation text without newline"
        );
        assert_eq!(
            read_file(target_workspace.path(), "assets/logo.png"),
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );
        assert!(target_workspace.path().join("legacy/old_code.rs").exists());
        assert_eq!(
            read_file(target_workspace.path(), "legacy/old_code.rs"),
            b"// To be removed\n"
        );
    }
}
