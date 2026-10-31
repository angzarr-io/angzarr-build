//! `--missing` for a Python file: a hand-written line-based reader of the
//! user's class, finding the methods it has.

use crate::error::CodegenError;
use crate::scaffold::ScaffoldFile;

/// `--missing` for a Python file: `existing`, the user's interface or
/// implementation of `fresh`'s component, with the methods of `fresh` it
/// lacks appended to its class; `None` when it lacks none. A method is
/// present when the class has a method taking its message first; one
/// taking an input as `cmd` where the proto declares a fact, or the
/// reverse, is refused.
pub(crate) fn complete(
    fresh: &ScaffoldFile,
    existing: &str,
) -> Result<Option<String>, CodegenError> {
    let not_found = || CodegenError::ScaffoldImplNotFound {
        path: fresh.path.clone(),
        component: fresh.component.clone(),
    };
    let Some(fresh_class) = class(&fresh.source, None) else {
        return Ok(None);
    };
    let user_class = class(existing, Some(&fresh_class.name)).ok_or_else(not_found)?;
    let package = fresh
        .component
        .rsplit_once('.')
        .map_or("", |(package, _)| package);
    let mut missing = Vec::new();
    for method in &fresh_class.methods {
        match user_class.methods.iter().find(|m| m.ty == method.ty) {
            Some(found)
                if is_input(&found.param)
                    && is_input(&method.param)
                    && found.param != method.param =>
            {
                return Err(CodegenError::InputMismatch {
                    path: fresh.path.clone(),
                    message: format!("{package}.{}", method.ty),
                    expected: method.param.clone(),
                    found: found.param.clone(),
                });
            }
            Some(_) => {}
            None => missing.push(&fresh.source[method.span.0..method.span.1]),
        }
    }
    if missing.is_empty() {
        return Ok(None);
    }
    let end = user_class.end;
    let mut insert = String::new();
    for block in missing {
        insert.push('\n');
        insert.push_str(block);
    }
    let mut completed = existing.to_string();
    if !completed[..end].ends_with('\n') {
        insert.insert(0, '\n');
    }
    completed.insert_str(end, &insert);
    Ok(Some(completed))
}

fn is_input(param: &str) -> bool {
    param == "cmd" || param == "fact"
}

/// A class of a Python module: its name, its methods and where its body
/// ends (the byte after its last non-blank line).
struct Class {
    name: String,
    methods: Vec<PyMethod>,
    end: usize,
}

/// One method: the first parameter after `self`, its annotated type, and
/// its text (from its `def` to the end of its body, newline included).
struct PyMethod {
    param: String,
    ty: String,
    span: (usize, usize),
}

/// The first top-level class of `source` (named `name` when given).
fn class(source: &str, name: Option<&str>) -> Option<Class> {
    let mut offset = 0;
    let lines: Vec<(usize, &str)> = source
        .split_inclusive('\n')
        .map(|l| {
            let at = offset;
            offset += l.len();
            (at, l)
        })
        .collect();
    let start = lines.iter().position(|(_, l)| {
        l.strip_prefix("class ").is_some_and(|rest| {
            let found: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            name.is_none_or(|n| n == found)
        })
    })?;
    let class_name: String = lines[start]
        .1
        .trim_start_matches("class ")
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    // The body: every line after the header that is blank or indented.
    let mut last = start;
    for (i, (_, line)) in lines.iter().enumerate().skip(start + 1) {
        if line.trim().is_empty() {
            continue;
        }
        if !line.starts_with(' ') && !line.starts_with('\t') {
            break;
        }
        last = i;
    }
    let end = lines[last].0 + lines[last].1.len();
    let mut methods = Vec::new();
    // The header is never a `def`: the range may hold it.
    let defs: Vec<usize> = (start..=last)
        .filter(|&i| {
            let line = lines[i].1;
            line.starts_with("    def ") || line.starts_with("    async def ")
        })
        .collect();
    for (k, &i) in defs.iter().enumerate() {
        let stop = defs.get(k + 1).map_or(last, |next| {
            let mut j = next - 1;
            while j > i && lines[j].1.trim().is_empty() {
                j -= 1;
            }
            j
        });
        let text: String = lines[i..=stop].iter().map(|(_, l)| *l).collect();
        let Some((param, ty)) = first_param(&text) else {
            continue;
        };
        methods.push(PyMethod {
            param,
            ty,
            span: (lines[i].0, lines[stop].0 + lines[stop].1.len()),
        });
    }
    Some(Class {
        name: class_name,
        methods,
        end,
    })
}

/// The first parameter after `self` of a `def` but the state or the
/// projection it folds into, as `(name, type)`, a leading `_` of the name
/// dropped, the type its last dotted name, unquoted.
fn first_param(def: &str) -> Option<(String, String)> {
    let mut parts = params(def)?.into_iter();
    if parts.next()?.trim() != "self" {
        return None;
    }
    parts.find_map(|part| {
        let (name, ty) = split_top(&part, ':')?;
        let name = name.trim().trim_start_matches('_');
        // Unannotated, a default's `:` (a lambda's) is not its annotation.
        if name.contains('=') || matches!(name, "state" | "projection") {
            return None;
        }
        let ty = split_top(ty, '=').map_or(ty, |(ty, _)| ty);
        let ty = ty.trim().trim_matches(['"', '\'']).rsplit('.').next()?;
        Some((name.to_string(), ty.trim().to_string()))
    })
}

/// The parameters of a `def`, each its text between the list's own
/// commas: the list read by its brackets, a string's or a comment's text
/// skipped, so no bracket or comma inside a default, a subscript, a string
/// or a comment ends one; a comment dropped. `None` when the list does not
/// close.
fn params(def: &str) -> Option<Vec<String>> {
    let open = def.find('(')?;
    let mut parts = vec![String::new()];
    let mut depth = 0usize;
    let mut chars = def[open + 1..].chars();
    while let Some(c) = chars.next() {
        match c {
            '#' => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
                continue;
            }
            '"' | '\'' => {
                let part = parts.last_mut()?;
                part.push(c);
                for d in chars.by_ref() {
                    part.push(d);
                    if d == c {
                        break;
                    }
                }
                continue;
            }
            '(' | '[' | '{' => depth += 1,
            ')' if depth == 0 => return Some(parts),
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(String::new());
                continue;
            }
            _ => {}
        }
        parts.last_mut()?.push(c);
    }
    None
}

/// `text` split at its first `at` outside brackets: a parameter's name and
/// its annotation, or an annotation and its default.
fn split_top(text: &str, at: char) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    for (i, c) in text.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            c if c == at && depth == 0 => return Some((&text[..i], &text[i + 1..])),
            _ => {}
        }
    }
    None
}
