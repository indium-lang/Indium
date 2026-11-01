use crate::{
    ast::*,
    checker,
    diagnostic::{Diagnostic, Result, Span},
    parser,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};
#[derive(Clone)]
pub struct Program {
    pub entry: String,
    pub modules: BTreeMap<String, Vec<Stmt>>,
    pub sources: BTreeMap<String, String>,
}
pub fn resolve_import(file: &str, path: &str) -> PathBuf {
    Path::new(file)
        .parent()
        .unwrap_or(Path::new("."))
        .join(path)
}
pub fn compile_file(path: &Path) -> Result<Program> {
    let mut p = Program {
        entry: String::new(),
        modules: BTreeMap::new(),
        sources: BTreeMap::new(),
    };
    let mut visiting = BTreeSet::new();
    p.entry = load(path, &mut p, &mut visiting, Span::default())?;
    Ok(p)
}
pub fn compile_source(source: &str) -> Result<Program> {
    let entry = "<source>".to_string();
    let body = parser::parse(source)
        .and_then(|b| {
            checker::check(&b)?;
            Ok(b)
        })
        .map_err(|e| e.in_file(&entry))?;
    if has_local_import(&body) {
        return Err(
            Diagnostic::new("Local imports require a source file path", Span::default())
                .in_file(&entry),
        );
    }
    Ok(Program {
        entry: entry.clone(),
        modules: BTreeMap::from([(entry.clone(), body)]),
        sources: BTreeMap::from([(entry, source.to_owned())]),
    })
}
fn has_local_import(body: &[Stmt]) -> bool {
    body.iter().any(|s| match &s.kind {
        StmtKind::Import { local, .. } => *local,
        StmtKind::Function { body, .. }
        | StmtKind::While { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Loop(body)
        | StmtKind::Every { body, .. } => has_local_import(body),
        StmtKind::If {
            then_body,
            else_body,
            ..
        } => has_local_import(then_body) || has_local_import(else_body),
        _ => false,
    })
}
fn load(
    path: &Path,
    p: &mut Program,
    visiting: &mut BTreeSet<String>,
    span: Span,
) -> Result<String> {
    let absolute = path
        .canonicalize()
        .map_err(|e| Diagnostic::new(format!("Cannot open '{}': {e}", path.display()), span))?;
    let file = absolute.to_string_lossy().into_owned();
    if visiting.contains(&file) {
        return Err(Diagnostic::new(
            format!("Circular import involving '{file}'"),
            span,
        ));
    }
    if p.modules.contains_key(&file) {
        return Ok(file);
    }
    if visiting.len() >= 64 {
        return Err(Diagnostic::new("Module import nesting exceeds 64", span));
    }
    visiting.insert(file.clone());
    let source = std::fs::read_to_string(&absolute)
        .map_err(|e| Diagnostic::new(format!("Cannot read source: {e}"), span).in_file(&file))?;
    let body = parser::parse(&source).map_err(|e| e.in_file(&file))?;
    checker::check(&body).map_err(|e| e.in_file(&file))?;
    load_imports(&body, &file, p, visiting)?;
    visiting.remove(&file);
    p.sources.insert(file.clone(), source);
    p.modules.insert(file.clone(), body);
    Ok(file)
}
fn load_imports(
    body: &[Stmt],
    file: &str,
    p: &mut Program,
    visiting: &mut BTreeSet<String>,
) -> Result<()> {
    for s in body {
        match &s.kind {
            StmtKind::Import {
                path, local: true, ..
            } => {
                if Path::new(path).extension().and_then(|s| s.to_str()) != Some("in") {
                    return Err(Diagnostic::new(
                        "Local modules must use the .in extension",
                        s.span,
                    )
                    .in_file(file));
                }
                load(&resolve_import(file, path), p, visiting, s.span)
                    .map_err(|e| e.in_file(file))?;
            }
            StmtKind::Function { body, .. }
            | StmtKind::While { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::Loop(body)
            | StmtKind::Every { body, .. } => load_imports(body, file, p, visiting)?,
            StmtKind::If {
                then_body,
                else_body,
                ..
            } => {
                load_imports(then_body, file, p, visiting)?;
                load_imports(else_body, file, p, visiting)?;
            }
            _ => {}
        }
    }
    Ok(())
}
