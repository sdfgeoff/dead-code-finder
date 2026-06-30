use std::collections::HashSet;

use deadcode_core::SymbolKind;

use crate::symbol_index::{IndexedSymbol, RootSymbol};

pub(crate) const EXPLICITLY_ALLOWED_GROUP: &str = "explicitlyAllowed";

pub(crate) fn allow_comment_roots(source: &str, symbols: &[IndexedSymbol]) -> Vec<RootSymbol> {
    let comments = AllowComments::new(source);
    if let Some(file_codes) = &comments.file_allow_codes {
        return symbols
            .iter()
            .filter(|symbol| symbol.kind != SymbolKind::Module)
            .filter(|symbol| codes_match(file_codes, code_for_kind(&symbol.kind)))
            .map(root_for_symbol)
            .collect();
    }

    let mut roots = Vec::new();
    let allowed_class_prefixes = symbols
        .iter()
        .filter(|symbol| symbol.kind == SymbolKind::Class)
        .filter(|symbol| comments.allows_symbol_line(symbol.span.line, code_for_kind(&symbol.kind)))
        .map(|symbol| {
            roots.push(root_for_symbol(symbol));
            format!("{}.", symbol.qualified_name)
        })
        .collect::<Vec<_>>();

    let mut seen = roots
        .iter()
        .map(|root| root.symbol.clone())
        .collect::<HashSet<_>>();
    for symbol in symbols {
        if symbol.kind == SymbolKind::Module {
            continue;
        }
        if seen.contains(&symbol.qualified_name) {
            continue;
        }
        let class_allowed = allowed_class_prefixes
            .iter()
            .any(|prefix| symbol.qualified_name.starts_with(prefix));
        let symbol_allowed =
            comments.allows_symbol_line(symbol.span.line, code_for_kind(&symbol.kind));
        if class_allowed || symbol_allowed {
            seen.insert(symbol.qualified_name.clone());
            roots.push(root_for_symbol(symbol));
        }
    }
    roots
}

fn root_for_symbol(symbol: &IndexedSymbol) -> RootSymbol {
    RootSymbol {
        group: EXPLICITLY_ALLOWED_GROUP.to_string(),
        symbol: symbol.qualified_name.clone(),
    }
}

fn code_for_kind(kind: &SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Function => "DCF001",
        SymbolKind::Class => "DCF002",
        SymbolKind::Method => "DCF003",
        SymbolKind::Attribute | SymbolKind::Field => "DCF004",
        SymbolKind::Module => "DCF000",
    }
}

struct AllowComments {
    file_allow_codes: Option<Vec<String>>,
    line_allows: Vec<LineAllow>,
    lines: Vec<String>,
}

struct LineAllow {
    line: usize,
    codes: Vec<String>,
}

impl AllowComments {
    fn new(source: &str) -> Self {
        let lines = source.lines().map(str::to_string).collect::<Vec<_>>();
        let mut file_allow_codes: Option<Vec<String>> = None;
        let mut line_allows = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            let Some(allow) = parse_allow_comment(line) else {
                continue;
            };
            if allow.file_level {
                if allow.codes.is_empty() {
                    file_allow_codes = Some(Vec::new());
                } else if let Some(codes) = &mut file_allow_codes {
                    if !codes.is_empty() {
                        codes.extend(allow.codes);
                    }
                } else {
                    file_allow_codes = Some(allow.codes);
                }
            } else {
                line_allows.push(LineAllow {
                    line: index + 1,
                    codes: allow.codes,
                });
            }
        }
        Self {
            file_allow_codes,
            line_allows,
            lines,
        }
    }

    fn allows_symbol_line(&self, line: usize, code: &str) -> bool {
        self.line_allows
            .iter()
            .any(|allow| allow.line == line && allow.matches_code(code))
            || self
                .previous_significant_line(line)
                .is_some_and(|previous| {
                    self.line_allows
                        .iter()
                        .any(|allow| allow.line == previous && allow.matches_code(code))
                })
    }

    fn previous_significant_line(&self, line: usize) -> Option<usize> {
        let mut current = line.checked_sub(1)?;
        while current > 0 {
            let text = self.lines.get(current - 1)?.trim();
            if text.is_empty() {
                current -= 1;
                continue;
            }
            return text.starts_with('#').then_some(current);
        }
        None
    }
}

impl LineAllow {
    fn matches_code(&self, code: &str) -> bool {
        codes_match(&self.codes, code)
    }
}

fn codes_match(codes: &[String], code: &str) -> bool {
    codes.is_empty() || codes.iter().any(|allowed| allowed == code)
}

struct ParsedAllow {
    file_level: bool,
    codes: Vec<String>,
}

fn parse_allow_comment(line: &str) -> Option<ParsedAllow> {
    let comment = line.split_once('#')?.1.trim();
    let directive = comment.strip_prefix("dead-code-finder:")?.trim();
    let mut parts = directive.split_whitespace();
    let command = parts.next()?;
    if !matches!(command, "allow" | "allow-file") {
        return None;
    }
    let codes = parts
        .flat_map(|part| part.split(','))
        .map(str::trim)
        .filter(|part| part.starts_with("DCF"))
        .map(str::to_string)
        .collect();
    Some(ParsedAllow {
        file_level: command == "allow-file",
        codes,
    })
}
