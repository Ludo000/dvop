    use super::*;
    use crate::extensions::native::NativeExtension;
    use serial_test::serial;
    use std::collections::HashMap;

    #[test]
    #[serial]
    fn code_completion_manifest_describes_native_extension() {
        let ext = CodeCompletionExtension::new();
        assert_eq!(ext.id(), "code-completion");

        let manifest = ext.manifest();
        assert_eq!(manifest.id, "code-completion");
        assert_eq!(manifest.name, "Code Completion");
        assert!(manifest.is_native);
        assert_eq!(manifest.enabled, ext.is_enabled());
    }

    #[test]
    #[serial]
    fn code_completion_enable_toggle_updates_global_state() {
        let mut ext = CodeCompletionExtension::new();

        ext.set_enabled(false);
        assert!(!ext.is_enabled());
        assert!(!is_enabled());

        ext.set_enabled(true);
        assert!(ext.is_enabled());
        assert!(is_enabled());
    }

    #[test]
    #[serial]
    fn code_completion_register_can_be_called_multiple_times() {
        register();
        register();
        assert!(is_enabled());
    }

    #[test]
    #[serial]
    fn code_completion_persist_enabled_state_writes_config_file() {
        let temp = tempfile::tempdir().unwrap();
        let previous_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", temp.path());

        let mut ext = CodeCompletionExtension::new();
        ext.set_enabled(false);

        let config_path = temp.path().join(".config/dvop/native_extensions.json");
        assert!(config_path.exists());
        let map: HashMap<String, bool> =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(map.get("code-completion"), Some(&false));

        match previous_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
    }

    #[test]
    #[serial]
    fn code_completion_persist_enabled_state_preserves_other_extension_keys() {
        let temp = tempfile::tempdir().unwrap();
        let config_dir = temp.path().join(".config/dvop");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("native_extensions.json"),
            r#"{"rust-diagnostics": true}"#,
        )
        .unwrap();

        let previous_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", temp.path());

        let mut ext = CodeCompletionExtension::new();
        ext.set_enabled(false);

        let map: HashMap<String, bool> = serde_json::from_str(
            &std::fs::read_to_string(config_dir.join("native_extensions.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(map.get("rust-diagnostics"), Some(&true));
        assert_eq!(map.get("code-completion"), Some(&false));

        match previous_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
    }
