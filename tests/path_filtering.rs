use glob::Pattern;
use std::path::Path;
use flash_watcher::should_process_path;

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
}
