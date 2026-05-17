    use super::*;
    use serde::Deserialize;
    use std::fs;
    use std::time::Duration;

    fn write_script(dir: &tempfile::TempDir, name: &str, body: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn run_script_reports_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.sh");

        let err = run_script(&missing, &[], None).unwrap_err();

        assert!(err.contains("Script not found"));
        assert!(err.contains("missing.sh"));
    }

    #[test]
    fn run_script_passes_args_and_stdin_and_trims_stdout() {
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(
            &dir,
            "echo-input.sh",
            r#"read input
printf 'arg=%s input=%s
' "$1" "$input"
"#,
        );

        let output = run_script(&script, &["value"], Some("from-stdin\n")).unwrap();

        assert_eq!(output, "arg=value input=from-stdin");
    }

    #[test]
    fn run_script_returns_exit_status_and_stderr() {
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(
            &dir,
            "fail.sh",
            r#"printf 'bad things happened' >&2
exit 7
"#,
        );

        let err = run_script(&script, &[], None).unwrap_err();

        assert!(err.contains("Script exited"));
        assert!(err.contains("bad things happened"));
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct ScriptPayload {
        name: String,
        count: u32,
    }

    #[test]
    fn run_script_json_deserializes_stdout() {
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(
            &dir,
            "json.sh",
            r#"printf '{"name":"extension","count":3}'"#,
        );

        let parsed: ScriptPayload = run_script_json(&script, &[]).unwrap();

        assert_eq!(
            parsed,
            ScriptPayload {
                name: "extension".to_string(),
                count: 3,
            }
        );
    }

    #[test]
    fn run_script_json_reports_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(&dir, "invalid-json.sh", r#"printf 'not-json'"#);

        let err = run_script_json::<ScriptPayload>(&script, &[]).unwrap_err();

        assert!(err.contains("Failed to parse script JSON output"));
    }

    #[test]
    fn fire_and_forget_runs_existing_script_without_blocking() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("marker.txt");
        let script = write_script(
            &dir,
            "background.sh",
            &format!(r#"printf '%s' "$1" > "{}""#, marker.display()),
        );

        run_script_fire_and_forget(&script, &["done"]);

        for _ in 0..20 {
            if marker.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
        }

        assert_eq!(fs::read_to_string(marker).unwrap(), "done");
    }

    #[test]
    fn fire_and_forget_ignores_missing_script() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.sh");

        run_script_fire_and_forget(&missing, &["ignored"]);
    }
