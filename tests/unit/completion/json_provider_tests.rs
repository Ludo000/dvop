    use super::*;

    fn sample_language() -> LanguageCompletionData {
        LanguageCompletionData {
            language: "testlang".to_string(),
            description: "Test language".to_string(),
            keywords: vec![
                KeywordData {
                    keyword: "fn".to_string(),
                    r#type: "keyword".to_string(),
                    description: "Declares a function".to_string(),
                    example: "fn main() {}".to_string(),
                    category: "functions".to_string(),
                },
                KeywordData {
                    keyword: "let".to_string(),
                    r#type: "keyword".to_string(),
                    description: "Binds a value".to_string(),
                    example: "let value = 1;".to_string(),
                    category: "bindings".to_string(),
                },
            ],
            snippets: vec![SnippetData {
                trigger: "main".to_string(),
                description: "Main function".to_string(),
                content: "fn main() {}".to_string(),
                category: "functions".to_string(),
            }],
            imports: Some(ModuleHierarchy {
                modules: vec![
                    ModuleData {
                        path: "std".to_string(),
                        items: vec![ImportItem {
                            name: "fs".to_string(),
                            item_type: "module".to_string(),
                            description: "Filesystem APIs".to_string(),
                        }],
                        submodules: vec!["fs".to_string(), "io".to_string()],
                    },
                    ModuleData {
                        path: "std::fs".to_string(),
                        items: vec![ImportItem {
                            name: "read_to_string".to_string(),
                            item_type: "function".to_string(),
                            description: "Reads a file".to_string(),
                        }],
                        submodules: Vec::new(),
                    },
                ],
            }),
        }
    }

    fn write_language_file(dir: &tempfile::TempDir, language: &str, data: &LanguageCompletionData) {
        let path = dir.path().join(format!("{}.json", language));
        let json = serde_json::to_string(data).unwrap();
        std::fs::write(path, json).unwrap();
    }

    #[test]
    fn json_provider_loads_keywords_snippets_docs_and_imports() {
        let dir = tempfile::tempdir().unwrap();
        write_language_file(&dir, "testlang", &sample_language());

        let provider = JsonCompletionProvider::from_file(&dir.path().join("testlang.json")).unwrap();

        assert_eq!(provider.language_data().language, "testlang");
        assert_eq!(provider.keywords(), vec!["fn", "let"]);
        assert_eq!(provider.snippets(), vec![("main", "fn main() {}")]);
        assert!(provider
            .get_keyword_documentation("fn")
            .contains("Declares a function"));
        assert!(provider
            .get_snippet_documentation("main")
            .contains("Main function"));
        assert_eq!(provider.get_import_suggestions("std")[0].name, "fs");
        assert_eq!(provider.get_submodules("std"), vec!["fs", "io"]);
        assert_eq!(
            provider.find_matching_modules("std"),
            vec!["std".to_string(), "std::fs".to_string()]
        );
    }

    #[test]
    fn json_provider_returns_fallback_documentation_for_unknown_items() {
        let dir = tempfile::tempdir().unwrap();
        write_language_file(&dir, "testlang", &sample_language());
        let provider = JsonCompletionProvider::from_file(&dir.path().join("testlang.json")).unwrap();

        assert_eq!(
            provider.get_keyword_documentation("missing"),
            "missing - No documentation available"
        );
        assert_eq!(
            provider.get_snippet_documentation("missing"),
            "missing (snippet) - No documentation available"
        );
        assert!(provider.get_import_suggestions("missing").is_empty());
        assert!(provider.get_submodules("missing").is_empty());
        assert!(provider.find_matching_modules("missing").is_empty());
    }

    #[test]
    fn completion_manager_lists_and_loads_languages() {
        let dir = tempfile::tempdir().unwrap();
        write_language_file(&dir, "zeta", &sample_language());
        write_language_file(&dir, "alpha", &sample_language());
        std::fs::write(dir.path().join("README.txt"), "ignored").unwrap();

        let mut manager = CompletionDataManager::new(dir.path().to_string_lossy());

        assert_eq!(
            manager.list_available_languages().unwrap(),
            vec!["alpha".to_string(), "zeta".to_string()]
        );
        assert_eq!(
            manager.load_all_languages().unwrap(),
            vec!["alpha".to_string(), "zeta".to_string()]
        );
        assert!(manager.get_provider("alpha").is_some());
    }

    #[test]
    fn completion_manager_reports_missing_data_directory_and_language() {
        let dir = tempfile::tempdir().unwrap();
        let missing_dir = dir.path().join("missing");
        let mut manager = CompletionDataManager::new(missing_dir.to_string_lossy());

        assert!(manager.list_available_languages().is_err());
        assert!(manager.load_all_languages().is_err());
        assert!(manager.load_language("rust").is_err());
        assert!(manager.get_provider("rust").is_none());
    }

    #[test]
    fn completion_manager_remove_blocks_autoload_until_data_is_added() {
        let dir = tempfile::tempdir().unwrap();
        write_language_file(&dir, "testlang", &sample_language());
        let mut manager = CompletionDataManager::new(dir.path().to_string_lossy());

        assert!(manager.get_provider("testlang").is_some());
        manager.remove_provider("testlang");
        assert!(manager.get_provider("testlang").is_none());

        manager.add_language_data("testlang", sample_language());
        assert!(manager.get_provider("testlang").is_some());
    }

    #[test]
    fn completion_manager_merges_data_without_duplicate_keywords_or_snippets() {
        let dir = tempfile::tempdir().unwrap();
        let mut manager = CompletionDataManager::new(dir.path().to_string_lossy());
        manager.add_language_data("testlang", sample_language());

        let mut extra = sample_language();
        extra.keywords = vec![
            KeywordData {
                keyword: "fn".to_string(),
                r#type: "keyword".to_string(),
                description: "duplicate".to_string(),
                example: "duplicate".to_string(),
                category: "duplicate".to_string(),
            },
            KeywordData {
                keyword: "struct".to_string(),
                r#type: "keyword".to_string(),
                description: "Declares a struct".to_string(),
                example: "struct User;".to_string(),
                category: "types".to_string(),
            },
        ];
        extra.snippets = vec![
            SnippetData {
                trigger: "main".to_string(),
                description: "duplicate".to_string(),
                content: "duplicate".to_string(),
                category: "duplicate".to_string(),
            },
            SnippetData {
                trigger: "test".to_string(),
                description: "Test function".to_string(),
                content: "#[test]\nfn test() {}".to_string(),
                category: "tests".to_string(),
            },
        ];
        extra.imports = Some(ModuleHierarchy {
            modules: vec![ModuleData {
                path: "std".to_string(),
                items: Vec::new(),
                submodules: Vec::new(),
            }],
        });

        manager.merge_language_data("testlang", extra);
        let provider = manager.get_provider("testlang").unwrap();

        assert_eq!(provider.keywords(), vec!["fn", "let", "struct"]);
        assert_eq!(provider.snippets().len(), 2);
        assert_eq!(provider.get_submodules("std"), vec!["fs", "io"]);
    }
