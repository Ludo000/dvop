    use super::*;

    #[test]
    fn test_rust_keywords_count() {
        let kws = rust_keywords();
        assert!(kws.len() >= 38, "Expected at least 38 keywords, got {}", kws.len());
        let names: Vec<&str> = kws.iter().map(|k| k.keyword.as_str()).collect();
        assert!(names.contains(&"fn"));
        assert!(names.contains(&"let"));
        assert!(names.contains(&"match"));
        assert!(names.contains(&"async"));
        assert!(names.contains(&"unsafe"));
    }

    #[test]
    fn test_rust_snippets_count() {
        let snips = rust_snippets();
        assert!(snips.len() >= 25, "Expected at least 25 snippets, got {}", snips.len());
        let triggers: Vec<&str> = snips.iter().map(|s| s.trigger.as_str()).collect();
        assert!(triggers.contains(&"fn"));
        assert!(triggers.contains(&"struct"));
        assert!(triggers.contains(&"impl"));
        assert!(triggers.contains(&"test"));
    }

    #[test]
    fn test_fallback_data_structure() {
        let data = fallback_data();
        assert_eq!(data.language, "rust");
        assert!(!data.keywords.is_empty());
        assert!(!data.snippets.is_empty());
        assert!(data.imports.is_none());
    }

    #[test]
    fn test_parse_all_html_sample() {
        let sample = r#"<html>
            <h3 id="structs">Structs</h3><ul class="all-items"><li><a href="collections/struct.HashMap.html">collections::HashMap</a></li><li><a href="collections/struct.HashSet.html">collections::HashSet</a></li></ul>
            <h3 id="traits">Traits</h3><ul class="all-items"><li><a href="clone/trait.Clone.html">clone::Clone</a></li></ul>
        </html>"#;

        let tmp = std::env::temp_dir().join("rust_completion_ext_test_all.html");
        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        fs::write(&tmp, sample).unwrap();
        let items = parse_all_html(&tmp);
        let _ = fs::remove_file(&tmp);

        assert!(items.contains_key("structs"));
        assert!(items.contains_key("traits"));
        let structs = &items["structs"];
        assert_eq!(structs.len(), 2);
        assert_eq!(structs[0].name, "HashMap");
        assert_eq!(structs[0].module_path, "collections::HashMap");
        assert_eq!(structs[1].name, "HashSet");
        let traits = &items["traits"];
        assert_eq!(traits.len(), 1);
        assert_eq!(traits[0].name, "Clone");
    }

    #[test]
    fn test_read_sidebar_items_sample() {
        let tmp_dir = std::env::temp_dir().join("rust_completion_ext_test_sidebar");
        let _ = fs::create_dir_all(&tmp_dir);
        let sidebar_content =
            r#"window.SIDEBAR_ITEMS = {"struct":["HashMap","HashSet"],"fn":["new"],"mod":["hash_map"]};"#;
        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        fs::write(tmp_dir.join("sidebar-items1.91.1.js"), sidebar_content).unwrap();

        let data = read_sidebar_items(&tmp_dir);
        let _ = fs::remove_dir_all(&tmp_dir);

        assert!(data.is_some());
        let data = data.unwrap();
        assert_eq!(data["struct"], vec!["HashMap", "HashSet"]);
        assert_eq!(data["fn"], vec!["new"]);
        assert_eq!(data["mod"], vec!["hash_map"]);
    }

    #[test]
    fn test_category_mappings() {
        assert_eq!(category_to_type("structs"), "type");
        assert_eq!(category_to_type("traits"), "trait");
        assert_eq!(category_to_type("functions"), "function");
        assert_eq!(category_to_type("macros"), "macro");
        assert_eq!(category_to_type("primitives"), "primitive");

        assert_eq!(category_to_keyword_category("structs"), "std_types");
        assert_eq!(category_to_keyword_category("traits"), "traits");
        assert_eq!(category_to_keyword_category("macros"), "macros");
    }

    #[test]
    fn test_category_mappings_use_keyword_fallback_for_unknown_sections() {
        assert_eq!(category_to_type("custom_section"), "keyword");
        assert_eq!(category_to_keyword_category("custom_section"), "other");
    }

    #[test]
    fn test_fallback_data_includes_rust_snippet_triggers() {
        let data = fallback_data();
        let triggers: Vec<&str> = data.snippets.iter().map(|s| s.trigger.as_str()).collect();
        assert!(triggers.contains(&"fn"));
        assert!(triggers.contains(&"match"));
    }

    #[test]
    fn test_load_rust_completions_returns_data() {
        let data = load_rust_completions();
        assert_eq!(data.language, "rust");
        assert!(!data.keywords.is_empty());
        assert!(!data.snippets.is_empty());
        let kw_names: Vec<&str> = data.keywords.iter().map(|k| k.keyword.as_str()).collect();
        assert!(kw_names.contains(&"fn"));
        assert!(kw_names.contains(&"let"));
    }

    #[test]
    fn test_cache_roundtrip() {
        let data = fallback_data();
        let toolchain = "test-toolchain-for-unit-test";
        let version = "rustc 99.0.0";
        save_cache(toolchain, version, &data);
        let loaded = load_cache(toolchain, version);
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.language, "rust");
        assert_eq!(loaded.keywords.len(), data.keywords.len());
        assert!(load_cache(toolchain, "rustc 99.1.0").is_none());
        let _ = fs::remove_file(cache_file_path());
    }

    #[test]
    fn parse_module_hierarchy_reads_sidebar_items_from_directory() {
        let tmp_dir = std::env::temp_dir().join("rust_completion_ext_test_hierarchy");
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir).unwrap();
        fs::write(
            tmp_dir.join("sidebar-items1.91.1.js"),
            r#"window.SIDEBAR_ITEMS = {"struct":["HashMap"],"mod":["hash_map"]};"#,
        )
        .unwrap();

        let hierarchy = parse_module_hierarchy(&tmp_dir);
        let _ = fs::remove_dir_all(&tmp_dir);

        assert!(!hierarchy.modules.is_empty());
        assert!(hierarchy
            .modules
            .iter()
            .any(|module| module.items.iter().any(|item| item.name == "HashMap")));
    }

    #[test]
    fn parse_module_hierarchy_collects_function_items() {
        let tmp_dir = std::env::temp_dir().join("rust_completion_ext_test_hierarchy_fn");
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir).unwrap();
        fs::write(
            tmp_dir.join("sidebar-items1.91.1.js"),
            r#"window.SIDEBAR_ITEMS = {"fn":["drop","clone"]};"#,
        )
        .unwrap();

        let hierarchy = parse_module_hierarchy(&tmp_dir);
        let _ = fs::remove_dir_all(&tmp_dir);

        let functions: Vec<_> = hierarchy
            .modules
            .iter()
            .flat_map(|module| module.items.iter())
            .filter(|item| item.item_type == "function")
            .map(|item| item.name.as_str())
            .collect();
        assert!(functions.contains(&"drop"));
        assert!(functions.contains(&"clone"));
    }
