use flash_watcher::should_process_path;
use glob::Pattern;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_patterns(patterns: &[&str]) -> Vec<Pattern> {
        patterns.iter().map(|p| Pattern::new(p).unwrap()).collect()
    }

    #[test]
    fn test_ignore_patterns() {
        let path = Path::new("/home/user/project/node_modules/package.js");
        let ignore_patterns = create_patterns(&["**/node_modules/**"]);
        let include_patterns = vec![];
        let extensions = None;

        assert!(!should_process_path(
            path,
            &extensions,
            &include_patterns,
            &ignore_patterns
        ));
    }

    #[test]
    fn test_include_patterns() {
        let path = Path::new("/home/user/project/src/app.js");
        let ignore_patterns = vec![];
        let include_patterns = create_patterns(&["**/src/**/*.js"]);
        let extensions = None;

        assert!(should_process_path(
            path,
            &extensions,
            &include_patterns,
            &ignore_patterns
        ));

        // Should not match if pattern doesn't match
        let path = Path::new("/home/user/project/lib/app.js");
        assert!(!should_process_path(
            path,
            &extensions,
            &include_patterns,
            &ignore_patterns
        ));
    }

    #[test]
    fn test_extension_filtering() {
        let path = Path::new("app.js");
        let ignore_patterns = vec![];
        let include_patterns = vec![];
        let extensions = Some("js,jsx,ts".to_string());

        assert!(should_process_path(
            path,
            &extensions,
            &include_patterns,
            &ignore_patterns
        ));

        // Should not match if extension is not in the list
        let path = Path::new("app.css");
        assert!(!should_process_path(
            path,
            &extensions,
            &include_patterns,
            &ignore_patterns
        ));
    }

    #[test]
    fn test_multiple_filters() {
        let path = Path::new("/home/user/project/src/app.js");
        let ignore_patterns = create_patterns(&["**/node_modules/**", "**/dist/**"]);
        let include_patterns = create_patterns(&["**/src/**"]);
        let extensions = Some("js,jsx".to_string());

        assert!(should_process_path(
            path,
            &extensions,
            &include_patterns,
            &ignore_patterns
        ));

        // Should not match if in ignored directory
        let path = Path::new("/home/user/project/node_modules/app.js");
        assert!(!should_process_path(
            path,
            &extensions,
            &include_patterns,
            &ignore_patterns
        ));

        // Should not match if extension not in list
        let path = Path::new("/home/user/project/src/app.css");
        assert!(!should_process_path(
            path,
            &extensions,
            &include_patterns,
            &ignore_patterns
        ));
    }

    #[test]
    fn test_no_filters() {
        let path = Path::new("any_file.txt");
        let ignore_patterns = vec![];
        let include_patterns = vec![];
        let extensions = None;

        assert!(should_process_path(
            path,
            &extensions,
            &include_patterns,
            &ignore_patterns
        ));
    }

    #[test]
    fn test_extension_filter_edge_cases() {
        // Test file with no extension
        let path_no_ext = Path::new("Makefile");
        let extensions = Some("js,ts".to_string());
        assert!(!should_process_path(path_no_ext, &extensions, &[], &[]));

        // Test extension with spaces
        let extensions_spaces = Some("js, ts, jsx ".to_string());
        let path_js = Path::new("test.js");
        let path_ts = Path::new("test.ts");
        let path_jsx = Path::new("test.jsx");
        assert!(should_process_path(path_js, &extensions_spaces, &[], &[]));
        assert!(should_process_path(path_ts, &extensions_spaces, &[], &[]));
        assert!(should_process_path(path_jsx, &extensions_spaces, &[], &[]));

        // Test single extension
        let extensions_single = Some("rs".to_string());
        let path_rs = Path::new("main.rs");
        let path_py = Path::new("main.py");
        assert!(should_process_path(path_rs, &extensions_single, &[], &[]));
        assert!(!should_process_path(path_py, &extensions_single, &[], &[]));

        // Test empty extension filter
        let extensions_empty = Some("".to_string());
        assert!(!should_process_path(path_rs, &extensions_empty, &[], &[]));
    }

    #[test]
    fn test_ignore_patterns_priority() {
        // Ignore patterns should take priority over include patterns
        let path = Path::new("src/node_modules/test.js");
        let include_patterns = create_patterns(&["src/**/*"]);
        let ignore_patterns = create_patterns(&["**/node_modules/**"]);

        assert!(!should_process_path(
            path,
            &None,
            &include_patterns,
            &ignore_patterns
        ));
    }

    #[test]
    fn test_complex_glob_patterns() {
        // Test complex glob patterns - note that brace expansion might not work in all glob implementations
        let patterns = create_patterns(&[
            "src/**/*.js",
            "src/**/*.ts",
            "src/**/*.jsx",
            "src/**/*.tsx",
            "tests/**/*.test.js",
        ]);

        assert!(should_process_path(
            Path::new("src/components/Button.jsx"),
            &None,
            &patterns,
            &[]
        ));
        assert!(should_process_path(
            Path::new("src/utils/helper.ts"),
            &None,
            &patterns,
            &[]
        ));
        assert!(should_process_path(
            Path::new("tests/unit/component.test.js"),
            &None,
            &patterns,
            &[]
        ));
        assert!(!should_process_path(
            Path::new("docs/readme.md"),
            &None,
            &patterns,
            &[]
        ));
        assert!(!should_process_path(
            Path::new("src/styles.css"),
            &None,
            &patterns,
            &[]
        ));
    }

    #[test]
    fn test_path_with_special_characters() {
        let path = Path::new("src/file with spaces.js");
        let extensions = Some("js".to_string());
        assert!(should_process_path(path, &extensions, &[], &[]));

        let path_unicode = Path::new("src/файл.js");
        assert!(should_process_path(path_unicode, &extensions, &[], &[]));

        let path_symbols = Path::new("src/file-name_with.symbols.js");
        assert!(should_process_path(path_symbols, &extensions, &[], &[]));
    }

    #[test]
    fn test_case_sensitivity() {
        let extensions = Some("JS,TS".to_string());
        let path_lower = Path::new("test.js");
        let path_upper = Path::new("test.JS");

        // Extension matching should be case sensitive
        assert!(!should_process_path(path_lower, &extensions, &[], &[]));
        assert!(should_process_path(path_upper, &extensions, &[], &[]));
    }

    #[test]
    fn test_empty_include_patterns_with_extensions() {
        // When include patterns are empty, only extension filter should apply
        let path = Path::new("anywhere/test.js");
        let extensions = Some("js".to_string());
        let include_patterns = vec![];
        let ignore_patterns = vec![];

        assert!(should_process_path(
            path,
            &extensions,
            &include_patterns,
            &ignore_patterns
        ));
    }

    #[test]
    fn test_multiple_extension_matches() {
        let extensions = Some("js,jsx,ts,tsx,vue,svelte".to_string());

        assert!(should_process_path(
            Path::new("app.js"),
            &extensions,
            &[],
            &[]
        ));
        assert!(should_process_path(
            Path::new("component.jsx"),
            &extensions,
            &[],
            &[]
        ));
        assert!(should_process_path(
            Path::new("types.ts"),
            &extensions,
            &[],
            &[]
        ));
        assert!(should_process_path(
            Path::new("component.tsx"),
            &extensions,
            &[],
            &[]
        ));
        assert!(should_process_path(
            Path::new("app.vue"),
            &extensions,
            &[],
            &[]
        ));
        assert!(should_process_path(
            Path::new("component.svelte"),
            &extensions,
            &[],
            &[]
        ));

        assert!(!should_process_path(
            Path::new("style.css"),
            &extensions,
            &[],
            &[]
        ));
        assert!(!should_process_path(
            Path::new("config.json"),
            &extensions,
            &[],
            &[]
        ));
    }
}
