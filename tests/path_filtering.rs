use glob::Pattern;
use std::path::Path;

// Import the function to test
// Note: we'll need to make sure this function is exposed for testing
mod path_filtering {
    use glob::Pattern;
    use std::path::Path;

    pub fn should_process_path(
        path: &Path,
        extensions: &Option<String>,
        include_patterns: &[Pattern],
        ignore_patterns: &[Pattern],
    ) -> bool {
        let path_str = match path.to_str() {
            Some(s) => s,
            None => return false, // Can't process paths that can't be converted to strings
        };

        // Check ignore patterns
        for pattern in ignore_patterns {
            if pattern.matches(path_str) {
                return false;
            }
        }

        // If we have include patterns, the path must match at least one
        if !include_patterns.is_empty() {
            let mut matches = false;
            for pattern in include_patterns {
                if pattern.matches(path_str) {
                    matches = true;
                    break;
                }
            }
            if !matches {
                return false;
            }
        }

        // If no extensions filter is specified, process the file
        let extensions = match extensions {
            Some(ext) => ext,
            None => return true,
        };

        // Check file extension
        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                return extensions.split(',').any(|e| e.trim() == ext_str);
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use path_filtering::should_process_path;

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
