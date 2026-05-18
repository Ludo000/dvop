    use super::*;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    };
    use serial_test::serial;

    #[derive(Default)]
    struct Counters {
        app_start: AtomicUsize,
        directory_open: AtomicUsize,
        file_open: AtomicUsize,
        file_save: AtomicUsize,
        shutdown: AtomicUsize,
        enabled: AtomicBool,
    }

    struct FakeNativeExtension {
        id: String,
        counters: Arc<Counters>,
    }

    impl FakeNativeExtension {
        fn new(id: &str, enabled: bool, counters: Arc<Counters>) -> Self {
            counters.enabled.store(enabled, Ordering::SeqCst);
            Self {
                id: id.to_string(),
                counters,
            }
        }
    }

    impl NativeExtension for FakeNativeExtension {
        fn id(&self) -> &str {
            &self.id
        }

        fn manifest(&self) -> crate::extensions::ExtensionManifest {
            crate::extensions::ExtensionManifest {
                id: self.id.clone(),
                name: format!("{} native", self.id),
                version: "1.0.0".to_string(),
                description: "fake native extension".to_string(),
                author: "tests".to_string(),
                enabled: self.is_enabled(),
                icon: None,
                is_native: true,
                contributions: crate::extensions::ExtensionContributions::default(),
            }
        }

        fn is_enabled(&self) -> bool {
            self.counters.enabled.load(Ordering::SeqCst)
        }

        fn set_enabled(&mut self, enabled: bool) {
            self.counters.enabled.store(enabled, Ordering::SeqCst);
        }

        fn on_app_start(&self) {
            self.counters.app_start.fetch_add(1, Ordering::SeqCst);
        }

        fn on_directory_open(&self, _dir: &Path) {
            self.counters.directory_open.fetch_add(1, Ordering::SeqCst);
        }

        fn on_file_open(&self, _file_path: &Path) {
            self.counters.file_open.fetch_add(1, Ordering::SeqCst);
        }

        fn on_file_save(&self, _file_path: &Path) {
            self.counters.file_save.fetch_add(1, Ordering::SeqCst);
        }

        fn shutdown(&self) {
            self.counters.shutdown.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn clear_registry() {
        NATIVE_REGISTRY.lock().unwrap().clear();
    }

    #[test]
    #[serial]
    fn native_registry_reports_manifests_and_enabled_state() {
        clear_registry();
        let counters = Arc::new(Counters::default());

        register(Box::new(FakeNativeExtension::new(
            "fake-native",
            true,
            counters.clone(),
        )));

        assert!(is_native_extension("fake-native"));
        assert!(!is_native_extension("missing-native"));

        let manifests = get_native_manifests();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].id, "fake-native");
        assert!(manifests[0].enabled);
        assert!(manifests[0].is_native);

        set_native_enabled("fake-native", false);
        assert!(!counters.enabled.load(Ordering::SeqCst));
        set_native_enabled("missing-native", true);
        assert!(!counters.enabled.load(Ordering::SeqCst));

        clear_registry();
    }

    #[test]
    #[serial]
    fn native_events_only_fire_for_enabled_extensions_except_shutdown() {
        clear_registry();
        let enabled = Arc::new(Counters::default());
        let disabled = Arc::new(Counters::default());

        register(Box::new(FakeNativeExtension::new(
            "enabled-native",
            true,
            enabled.clone(),
        )));
        register(Box::new(FakeNativeExtension::new(
            "disabled-native",
            false,
            disabled.clone(),
        )));

        fire_on_app_start();
        fire_on_directory_open(Path::new("/tmp/project"));
        fire_on_file_open(Path::new("/tmp/project/main.rs"));
        fire_on_file_save(Path::new("/tmp/project/main.rs"));
        shutdown_all();

        assert_eq!(enabled.app_start.load(Ordering::SeqCst), 1);
        assert_eq!(enabled.directory_open.load(Ordering::SeqCst), 1);
        assert_eq!(enabled.file_open.load(Ordering::SeqCst), 1);
        assert_eq!(enabled.file_save.load(Ordering::SeqCst), 1);
        assert_eq!(enabled.shutdown.load(Ordering::SeqCst), 1);

        assert_eq!(disabled.app_start.load(Ordering::SeqCst), 0);
        assert_eq!(disabled.directory_open.load(Ordering::SeqCst), 0);
        assert_eq!(disabled.file_open.load(Ordering::SeqCst), 0);
        assert_eq!(disabled.file_save.load(Ordering::SeqCst), 0);
        assert_eq!(disabled.shutdown.load(Ordering::SeqCst), 1);

        clear_registry();
    }
