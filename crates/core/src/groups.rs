use std::collections::{BTreeMap, BTreeSet};

pub fn build_group_mask(
    groups: &BTreeMap<String, Vec<bool>>,
    expr: &str,
    len: usize,
) -> Option<Vec<bool>> {
    let tokens = parse_group_tokens(expr);
    if tokens.is_empty() {
        return None;
    }

    let has_positive = tokens.iter().any(|(include, _)| *include);
    let mut selected: BTreeSet<String> = if has_positive {
        BTreeSet::new()
    } else {
        groups.keys().cloned().collect()
    };

    for (include, pattern) in tokens {
        for name in groups.keys() {
            if glob_match(&pattern, name) {
                if include {
                    selected.insert(name.clone());
                } else {
                    selected.remove(name);
                }
            }
        }
    }

    let mut mask = vec![false; len];
    for name in selected {
        if let Some(values) = groups.get(&name) {
            if values.len() != len {
                continue;
            }
            for (idx, value) in values.iter().enumerate() {
                if *value {
                    mask[idx] = true;
                }
            }
        }
    }

    Some(mask)
}

pub fn group_expr_matches(groups: &BTreeMap<String, Vec<bool>>, expr: &str) -> bool {
    let tokens = parse_group_tokens(expr);
    if tokens.is_empty() {
        return false;
    }
    let positives: Vec<_> = tokens
        .iter()
        .filter(|(include, _)| *include)
        .map(|(_, name)| name.as_str())
        .collect();
    if positives.is_empty() {
        return !groups.is_empty();
    }
    groups
        .keys()
        .any(|name| positives.iter().any(|pattern| glob_match(pattern, name)))
}

fn parse_group_tokens(expr: &str) -> Vec<(bool, String)> {
    let mut tokens = Vec::new();
    let normalized = expr.replace(',', " ");
    for token in normalized.split_whitespace() {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            continue;
        }
        let (include, name) = match trimmed.chars().next() {
            Some('^') | Some('!') => (false, trimmed[1..].to_string()),
            _ => (true, trimmed.to_string()),
        };
        if !name.is_empty() {
            tokens.push((include, name));
        }
    }
    tokens
}

fn glob_match(pattern: &str, value: &str) -> bool {
    glob_match_inner(pattern.as_bytes(), value.as_bytes())
}

fn glob_match_inner(pattern: &[u8], value: &[u8]) -> bool {
    if pattern.is_empty() {
        return value.is_empty();
    }
    match pattern[0] {
        b'*' => {
            for idx in 0..=value.len() {
                if glob_match_inner(&pattern[1..], &value[idx..]) {
                    return true;
                }
            }
            false
        }
        b'?' => {
            if value.is_empty() {
                false
            } else {
                glob_match_inner(&pattern[1..], &value[1..])
            }
        }
        ch => {
            if value.first().copied() == Some(ch) {
                glob_match_inner(&pattern[1..], &value[1..])
            } else {
                false
            }
        }
    }
}
