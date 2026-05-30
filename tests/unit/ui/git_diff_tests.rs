    use super::*;
    use serial_test::serial;

    #[test]
    fn compute_diff_operations_pairs_unchanged_lines() {
        let old = vec!["alpha", "beta", "gamma"];
        let new = vec!["alpha", "beta", "gamma"];
        let ops = compute_diff_operations(&old, &new);

        assert_eq!(ops.len(), 3);
        assert_eq!(ops[0], (Some(0), Some(0)));
        assert_eq!(ops[1], (Some(1), Some(1)));
        assert_eq!(ops[2], (Some(2), Some(2)));
    }

    #[test]
    fn compute_diff_operations_tracks_insertions_and_deletions() {
        let old = vec!["keep", "remove"];
        let new = vec!["keep", "insert"];
        let ops = compute_diff_operations(&old, &new);

        assert_eq!(ops.len(), 3);
        assert_eq!(ops[0], (Some(0), Some(0)));
        assert_eq!(ops[1], (Some(1), None));
        assert_eq!(ops[2], (None, Some(1)));
    }

    #[test]
    fn align_diff_content_produces_equal_row_counts() {
        let (left, right, left_map, right_map, _, _) =
            align_diff_content("alpha\nbeta\n", "alpha\ngamma\n");

        assert_eq!(left.lines().count(), right.lines().count());
        assert_eq!(left_map.len(), right_map.len());
        assert_eq!(left_map.len(), left.lines().count());
    }

    #[test]
    fn align_diff_content_marks_added_and_deleted_rows() {
        let (_, _, left_map, right_map, _, _) =
            align_diff_content("old line\n", "new line\n");

        assert!(left_map.iter().any(|line| line.is_none()));
        assert!(right_map.iter().any(|line| line.is_none()));
    }

    #[test]
    fn git_status_from_git_code_maps_porcelain_pairs() {
        assert_eq!(
            GitStatus::from_git_code(' ', 'M'),
            Some(GitStatus::Modified)
        );
        assert_eq!(
            GitStatus::from_git_code('M', ' '),
            Some(GitStatus::Staged)
        );
        assert_eq!(
            GitStatus::from_git_code('?', '?'),
            Some(GitStatus::Untracked)
        );
        assert_eq!(
            GitStatus::from_git_code('M', 'M'),
            Some(GitStatus::ModifiedStaged)
        );
        assert_eq!(GitStatus::from_git_code(' ', ' '), None);
    }

    #[test]
    fn is_git_repository_detects_dot_git_directory() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_git_repository(dir.path()));

        std::fs::create_dir_all(dir.path().join(".git")).unwrap();
        assert!(is_git_repository(dir.path()));
        assert!(!is_git_repository(dir.path().join("src/main.rs").as_path()));
    }

    #[test]
    fn is_git_repository_detects_git_worktree_pointer_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".git"), "gitdir: /tmp/fake.git\n").unwrap();
        assert!(is_git_repository(dir.path()));
    }

    #[test]
    fn find_git_root_walks_up_to_repository_root() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let nested = repo.path().join("src/deep");
        std::fs::create_dir_all(&nested).unwrap();

        assert_eq!(find_git_root(&nested), Some(repo.path().to_path_buf()));
        assert_eq!(find_git_root(repo.path()), Some(repo.path().to_path_buf()));
    }

    #[test]
    fn compute_line_changes_classifies_modified_rows() {
        let old = "    1  alpha\n    2  beta\n";
        let new = "    1  alpha\n    2  gamma\n";
        let changes = compute_line_changes(old, new, 5, 5);

        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0], LineChangeType::Unchanged);
        assert_eq!(changes[1], LineChangeType::Modified);
    }

    #[test]
    fn git_status_from_git_code_handles_added_and_deleted_states() {
        assert_eq!(
            GitStatus::from_git_code('A', ' '),
            Some(GitStatus::Added)
        );
        assert_eq!(
            GitStatus::from_git_code(' ', 'D'),
            Some(GitStatus::Deleted)
        );
        assert_eq!(
            GitStatus::from_git_code('D', ' '),
            Some(GitStatus::Deleted)
        );
        assert_eq!(
            GitStatus::from_git_code('R', ' '),
            Some(GitStatus::Renamed)
        );
    }

    #[test]
    fn compute_line_changes_detects_deleted_padding_rows() {
        let old = "    1  removed\n";
        let new = "       \n";
        let changes = compute_line_changes(old, new, 5, 5);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0], LineChangeType::Deleted);
    }

    #[test]
    fn align_diff_content_preserves_unchanged_lines() {
        let (left, right, _, _, _, _) = align_diff_content("same\n", "same\n");
        assert!(left.contains("same"));
        assert!(right.contains("same"));
    }

    #[test]
    fn compute_line_changes_marks_added_lines() {
        let old = "    1  keep\n";
        let new = "    1  keep\n    2  fresh\n";
        let changes = compute_line_changes(old, new, 5, 5);

        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0], LineChangeType::Unchanged);
        assert_eq!(changes[1], LineChangeType::Added);
    }

    #[test]
    fn compute_diff_operations_handles_all_new_content() {
        let old: Vec<&str> = vec![];
        let new = vec!["alpha", "beta"];
        let ops = compute_diff_operations(&old, &new);

        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0], (None, Some(0)));
        assert_eq!(ops[1], (None, Some(1)));
    }

    #[test]
    fn compute_diff_operations_handles_all_deleted_content() {
        let old = vec!["alpha", "beta"];
        let new: Vec<&str> = vec![];
        let ops = compute_diff_operations(&old, &new);

        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0], (Some(0), None));
        assert_eq!(ops[1], (Some(1), None));
    }

    #[test]
    fn git_status_from_git_code_maps_renamed_and_modified_staged() {
        assert_eq!(
            GitStatus::from_git_code('R', ' '),
            Some(GitStatus::Renamed)
        );
        assert_eq!(
            GitStatus::from_git_code('A', 'M'),
            Some(GitStatus::ModifiedStaged)
        );
    }

    #[test]
    fn git_status_update_callback_runs_when_invoked() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let called = Rc::new(RefCell::new(false));
        let called_for_callback = called.clone();
        set_git_status_update_callback(Rc::new(move || {
            *called_for_callback.borrow_mut() = true;
        }));

        invoke_git_status_update_callback_for_tests();
        assert!(*called.borrow());
    }

    #[test]
    #[serial]
    fn trigger_git_status_update_schedules_without_panicking() {
        use std::rc::Rc;

        gtk4::test_synced(|| {
            set_git_status_update_callback(Rc::new(|| {}));
            trigger_git_status_update();
        });
    }

    #[test]
    fn compute_line_changes_marks_modified_lines() {
        let old = "    1  alpha\n";
        let new = "    1  beta\n";
        let changes = compute_line_changes(old, new, 5, 5);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0], LineChangeType::Modified);
    }

    #[test]
    fn compute_diff_operations_replaces_all_lines() {
        let old = vec!["one", "two"];
        let new = vec!["three"];
        let ops = compute_diff_operations(&old, &new);

        assert_eq!(ops.len(), 3);
        assert_eq!(ops[0], (Some(0), None));
        assert_eq!(ops[1], (Some(1), None));
        assert_eq!(ops[2], (None, Some(0)));
    }

    #[test]
    fn find_git_root_returns_none_outside_repository() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(find_git_root(dir.path()), None);
    }

    #[test]
    fn git_status_from_git_code_maps_staged_only_changes() {
        assert_eq!(
            GitStatus::from_git_code('M', ' '),
            Some(GitStatus::Staged)
        );
        assert_eq!(
            GitStatus::from_git_code(' ', 'D'),
            Some(GitStatus::Deleted)
        );
    }

    #[test]
    fn align_diff_content_handles_empty_inputs() {
        let (left, right, left_map, right_map, _, _) = align_diff_content("", "");
        assert_eq!(left.lines().count(), right.lines().count());
        assert_eq!(left_map.len(), right_map.len());
    }

    #[test]
    fn compute_diff_operations_handles_complete_replacement() {
        let old = vec!["alpha", "beta", "gamma"];
        let new = vec!["delta"];
        let ops = compute_diff_operations(&old, &new);

        assert_eq!(ops.len(), 4);
        assert_eq!(ops[0], (Some(0), None));
        assert_eq!(ops[1], (Some(1), None));
        assert_eq!(ops[2], (Some(2), None));
        assert_eq!(ops[3], (None, Some(0)));
    }

    #[test]
    fn compute_line_changes_marks_unchanged_lines_with_matching_content() {
        let old = "    1  same\n    2  same\n";
        let new = "    1  same\n    2  same\n";
        let changes = compute_line_changes(old, new, 5, 5);

        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0], LineChangeType::Unchanged);
        assert_eq!(changes[1], LineChangeType::Unchanged);
    }

    #[test]
    fn git_status_from_git_code_maps_untracked_and_conflicted_patterns() {
        assert_eq!(
            GitStatus::from_git_code('?', '?'),
            Some(GitStatus::Untracked)
        );
        assert_eq!(GitStatus::from_git_code('U', 'U'), None);
    }

    fn init_git_repo(dir: &std::path::Path) -> bool {
        std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(dir)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn configure_git_user(dir: &std::path::Path) {
        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(dir)
            .status()
            .ok();
        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(dir)
            .status()
            .ok();
    }

    fn create_initial_commit(dir: &std::path::Path) {
        configure_git_user(dir);
        std::fs::write(dir.join("README.md"), "init\n").unwrap();
        std::process::Command::new("git")
            .args(["add", "README.md"])
            .current_dir(dir)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(dir)
            .status()
            .unwrap();
    }

    #[test]
    fn get_git_status_reports_untracked_files() {
        let dir = tempfile::tempdir().unwrap();
        if !init_git_repo(dir.path()) {
            return;
        }
        std::fs::write(dir.path().join("new.txt"), "hello").unwrap();

        let changes = get_git_status(dir.path());
        assert!(changes.iter().any(|c| c.status == GitStatus::Untracked));
        assert!(changes.iter().any(|c| c.path.ends_with("new.txt")));
    }

    #[test]
    fn get_current_branch_returns_main_after_init() {
        let dir = tempfile::tempdir().unwrap();
        if !init_git_repo(dir.path()) {
            return;
        }
        create_initial_commit(dir.path());

        assert_eq!(get_current_branch(dir.path()).as_deref(), Some("main"));
    }

    #[test]
    fn get_all_branches_marks_current_branch() {
        let dir = tempfile::tempdir().unwrap();
        if !init_git_repo(dir.path()) {
            return;
        }
        create_initial_commit(dir.path());
        let branches = get_all_branches(dir.path());
        assert!(branches.iter().any(|b| b.name == "main" && b.is_current));
    }

    #[test]
    fn get_old_and_new_file_content_differ_after_working_tree_edit() {
        let dir = tempfile::tempdir().unwrap();
        if !init_git_repo(dir.path()) {
            return;
        }
        configure_git_user(dir.path());

        let file_path = dir.path().join("tracked.txt");
        std::fs::write(&file_path, "committed\n").unwrap();
        std::process::Command::new("git")
            .args(["add", "tracked.txt"])
            .current_dir(dir.path())
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(dir.path())
            .status()
            .unwrap();

        std::fs::write(&file_path, "modified\n").unwrap();

        assert_eq!(
            get_old_file_content(dir.path(), &file_path).as_deref(),
            Some("committed\n")
        );
        assert_eq!(
            get_new_file_content(&file_path).as_deref(),
            Some("modified\n")
        );
    }

    #[test]
    fn align_diff_content_uses_at_least_five_digit_line_padding() {
        let old_lines: Vec<String> = (1..=120).map(|n| format!("old {n}")).collect();
        let new_lines: Vec<String> = (1..=120).map(|n| format!("new {n}")).collect();
        let old_content = old_lines.join("\n");
        let new_content = new_lines.join("\n");

        let (_, _, _, _, old_width, new_width) = align_diff_content(&old_content, &new_content);
        assert!(old_width >= 5);
        assert!(new_width >= 5);
    }

    #[test]
    fn get_git_status_reports_modified_tracked_file() {
        let dir = tempfile::tempdir().unwrap();
        if !init_git_repo(dir.path()) {
            return;
        }
        create_initial_commit(dir.path());

        let tracked = dir.path().join("README.md");
        std::fs::write(&tracked, "changed on disk\n").unwrap();

        let changes = get_git_status(dir.path());
        assert!(changes.iter().any(|c| c.status == GitStatus::Modified));
        assert!(changes.iter().any(|c| c.path.ends_with("README.md")));
    }

    #[test]
    fn stage_and_unstage_file_updates_index_and_status() {
        let dir = tempfile::tempdir().unwrap();
        if !init_git_repo(dir.path()) {
            return;
        }
        create_initial_commit(dir.path());

        let tracked = dir.path().join("README.md");
        std::fs::write(&tracked, "staged version\n").unwrap();
        stage_file(dir.path(), &tracked).expect("stage should succeed");

        assert_eq!(
            get_staged_file_content(dir.path(), &tracked).as_deref(),
            Some("staged version\n")
        );
        assert!(get_git_status(dir.path())
            .iter()
            .any(|c| c.status == GitStatus::Staged));

        unstage_file(dir.path(), &tracked).expect("unstage should succeed");
        assert!(get_git_status(dir.path())
            .iter()
            .any(|c| c.status == GitStatus::Modified));
    }

    #[test]
    fn discard_changes_restores_tracked_file_to_head() {
        let dir = tempfile::tempdir().unwrap();
        if !init_git_repo(dir.path()) {
            return;
        }
        create_initial_commit(dir.path());

        let tracked = dir.path().join("README.md");
        std::fs::write(&tracked, "temporary edit\n").unwrap();
        discard_changes(dir.path(), &tracked).expect("discard should succeed");

        assert_eq!(
            std::fs::read_to_string(&tracked).unwrap(),
            "init\n"
        );
    }

    #[test]
    fn revert_to_head_discards_staged_and_unstaged_changes() {
        let dir = tempfile::tempdir().unwrap();
        if !init_git_repo(dir.path()) {
            return;
        }
        create_initial_commit(dir.path());

        let tracked = dir.path().join("README.md");
        std::fs::write(&tracked, "working tree\n").unwrap();
        stage_file(dir.path(), &tracked).expect("stage should succeed");
        std::fs::write(&tracked, "still dirty\n").unwrap();

        revert_to_head(dir.path(), &tracked).expect("revert should succeed");
        assert_eq!(std::fs::read_to_string(&tracked).unwrap(), "init\n");
    }

    #[test]
    fn revert_all_unstaged_restores_multiple_modified_files() {
        let dir = tempfile::tempdir().unwrap();
        if !init_git_repo(dir.path()) {
            return;
        }
        create_initial_commit(dir.path());

        let first = dir.path().join("first.txt");
        let second = dir.path().join("second.txt");
        std::fs::write(&first, "first v1\n").unwrap();
        std::fs::write(&second, "second v1\n").unwrap();
        std::process::Command::new("git")
            .args(["add", "first.txt", "second.txt"])
            .current_dir(dir.path())
            .status()
            .expect("git add should succeed");
        std::process::Command::new("git")
            .args(["commit", "-m", "add files"])
            .current_dir(dir.path())
            .status()
            .expect("git commit should succeed");

        std::fs::write(&first, "first dirty\n").unwrap();
        std::fs::write(&second, "second dirty\n").unwrap();

        revert_all_unstaged(dir.path()).expect("revert all should succeed");
        assert_eq!(std::fs::read_to_string(&first).unwrap(), "first v1\n");
        assert_eq!(std::fs::read_to_string(&second).unwrap(), "second v1\n");
    }

    #[test]
    fn commit_changes_rejects_empty_message() {
        let dir = tempfile::tempdir().unwrap();
        if !init_git_repo(dir.path()) {
            return;
        }
        create_initial_commit(dir.path());

        let err = commit_changes(dir.path(), "   ").unwrap_err();
        assert!(err.contains("Commit message cannot be empty"));
    }

    #[test]
    fn commit_changes_creates_commit_for_staged_changes() {
        let dir = tempfile::tempdir().unwrap();
        if !init_git_repo(dir.path()) {
            return;
        }
        create_initial_commit(dir.path());

        let tracked = dir.path().join("notes.txt");
        std::fs::write(&tracked, "staged content\n").unwrap();
        stage_file(dir.path(), &tracked).expect("stage should succeed");
        commit_changes(dir.path(), "add notes").expect("commit should succeed");

        let log = std::process::Command::new("git")
            .args(["log", "-1", "--pretty=%s"])
            .current_dir(dir.path())
            .output()
            .expect("git log should succeed");
        assert_eq!(String::from_utf8_lossy(&log.stdout).trim(), "add notes");
    }

    #[test]
    fn get_new_file_content_returns_none_for_missing_file() {
        let missing = std::path::PathBuf::from("/tmp/dvop-missing-git-file.txt");
        assert!(get_new_file_content(&missing).is_none());
    }

    #[test]
    fn get_staged_file_content_returns_none_for_untracked_file() {
        let dir = tempfile::tempdir().unwrap();
        if !init_git_repo(dir.path()) {
            return;
        }
        create_initial_commit(dir.path());

        let untracked = dir.path().join("new.txt");
        std::fs::write(&untracked, "never staged\n").unwrap();
        assert!(get_staged_file_content(dir.path(), &untracked).is_none());
    }

    #[test]
    fn git_status_from_git_code_maps_deleted_files() {
        assert_eq!(
            GitStatus::from_git_code('D', ' '),
            Some(GitStatus::Deleted)
        );
        assert_eq!(
            GitStatus::from_git_code(' ', 'D'),
            Some(GitStatus::Deleted)
        );
    }

    #[test]
    fn stage_file_errors_when_path_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        if !init_git_repo(dir.path()) {
            return;
        }
        create_initial_commit(dir.path());

        let missing = dir.path().join("missing.txt");
        assert!(stage_file(dir.path(), &missing).is_err());
    }
