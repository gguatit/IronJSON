pub fn glob_match(pattern: &str, path: &str) -> bool {
    let pattern_segments: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    glob_match_segments(&pattern_segments, &path_segments)
}

fn glob_match_segments(pattern: &[&str], path: &[&str]) -> bool {
    let mut pi = 0;
    let mut xi = 0;

    while pi < pattern.len() && xi < path.len() {
        match pattern[pi] {
            "**" => {
                if pi + 1 == pattern.len() {
                    return true;
                }
                let rest_pattern = &pattern[pi + 1..];
                for start in xi..=path.len() {
                    if glob_match_segments(rest_pattern, &path[start..]) {
                        return true;
                    }
                }
                return false;
            }
            seg if seg.contains('*') && !seg.contains('?') => {
                if !wildcard_segment_match(seg, path[xi]) {
                    return false;
                }
            }
            seg => {
                if seg != path[xi] {
                    return false;
                }
            }
        }
        pi += 1;
        xi += 1;
    }

    while pi < pattern.len() && pattern[pi] == "**" {
        pi += 1;
    }

    pi == pattern.len() && xi == path.len()
}

fn wildcard_segment_match(pattern: &str, segment: &str) -> bool {
    let pattern_bytes = pattern.as_bytes();
    let segment_bytes = segment.as_bytes();
    let mut pi = 0;
    let mut si = 0;
    let mut star_pi = usize::MAX;
    let mut star_si = 0usize;

    while si < segment_bytes.len() {
        if pi < pattern_bytes.len() && pattern_bytes[pi] == b'*' {
            star_pi = pi;
            star_si = si;
            pi += 1;
        } else if pi < pattern_bytes.len()
            && (pattern_bytes[pi] == segment_bytes[si]
                || pattern_bytes[pi] == b'?')
        {
            pi += 1;
            si += 1;
        } else if star_pi != usize::MAX {
            pi = star_pi + 1;
            star_si += 1;
            si = star_si;
        } else {
            return false;
        }
    }

    while pi < pattern_bytes.len() && pattern_bytes[pi] == b'*' {
        pi += 1;
    }

    pi == pattern_bytes.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        assert!(glob_match("/api/users", "/api/users"));
        assert!(!glob_match("/api/users", "/api/posts"));
    }

    #[test]
    fn test_single_wildcard() {
        assert!(glob_match("/api/*", "/api/users"));
        assert!(glob_match("/api/*", "/api/posts"));
        assert!(!glob_match("/api/*", "/api/v1/users"));
    }

    #[test]
    fn test_double_wildcard() {
        assert!(glob_match("/api/**", "/api/v1/users"));
        assert!(glob_match("/api/**", "/api/a/b/c"));
        assert!(glob_match("/api/**/users", "/api/v1/users"));
        assert!(glob_match("/api/**/users", "/api/v1/v2/users"));
    }

    #[test]
    fn test_wildcard_in_segment() {
        assert!(glob_match("/api/user-*", "/api/user-123"));
        assert!(!glob_match("/api/user-*", "/api/post-123"));
    }

    #[test]
    fn test_no_match() {
        assert!(!glob_match("/api/users", "/api/posts"));
        assert!(!glob_match("/api/users/*", "/api/posts/1"));
    }
}
