use super::{App, Candidate, Hit, Path, PathBuf, Reason, search};

struct Owner {
    path: PathBuf,
    reason: Reason,
    source: String,
    table: Option<String>,
}

impl App {
    pub(super) fn lua_qualified(
        &self,
        here: &Path,
        text: &str,
        chain: &[String],
        before: &str,
        word: &str,
    ) -> Vec<Candidate> {
        let line = self.line + 1;
        let o = match search::lua_called_receiver(before) {
            Some(call) if chain.is_empty() => self.lua_expr_owner(here, text, call, line, 0),
            _ => self.lua_owner(here, text, chain, line, 0),
        };
        let Some(o) = o else {
            return Vec::new();
        };
        let Some(table) = o.table.as_deref() else {
            return Vec::new();
        };
        let mut lines = search::lua_members(&o.source, table, word);
        if lines.is_empty() {
            lines.extend(search::lua_handed_on(&o.source, table, word));
        }
        lines
            .into_iter()
            .map(|line| Candidate {
                hit: Hit {
                    deleted: None,
                    path: o.path.clone(),
                    line1: line,
                    byte_col: None,
                    text: o
                        .source
                        .lines()
                        .nth(line - 1)
                        .unwrap_or_default()
                        .to_owned(),
                },
                reason: o.reason.clone(),
            })
            .collect()
    }

    fn lua_owner(
        &self,
        here: &Path,
        text: &str,
        chain: &[String],
        line: usize,
        hops: usize,
    ) -> Option<Owner> {
        let (first, rest) = chain.split_first()?;
        if hops > 4 {
            return None;
        }
        let mut owner = if first == "self" {
            let table = search::lua_self_table(text, line)?;
            let parts: Vec<String> = table.split('.').map(str::to_owned).collect();
            Owner {
                reason: Reason::Receiver(table),
                ..self.lua_owner(here, text, &parts, line, hops + 1)?
            }
        } else {
            self.lua_value_owner(here, text, chain, line, hops)?
        };
        for name in rest {
            owner.table = Some(search::lua_table(
                &owner.source,
                owner.table.as_deref(),
                name,
            )?);
        }
        Some(owner)
    }

    fn lua_value_owner(
        &self,
        here: &Path,
        text: &str,
        chain: &[String],
        line: usize,
        hops: usize,
    ) -> Option<Owner> {
        let first = chain.first()?;
        let (bound, value) = search::lua_local_at(text, line, first)?;
        if search::lua_table_value(&value) {
            return Some(Owner {
                path: here.to_path_buf(),
                reason: Reason::Path(chain.join(".")),
                source: self.text_of(here)?,
                table: Some(first.clone()),
            });
        }
        self.lua_expr_owner(here, text, &value, bound, hops)
    }

    fn lua_expr_owner(
        &self,
        here: &Path,
        text: &str,
        value: &str,
        line: usize,
        hops: usize,
    ) -> Option<Owner> {
        if hops > 4 {
            return None;
        }
        if let Some(module) = search::lua_required(value) {
            let path = search::lua_module_file(module, &self.files)?;
            let source = self.text_of(&path)?;
            return Some(Owner {
                table: search::lua_returned(&source),
                reason: Reason::Import(module.to_owned()),
                path,
                source,
            });
        }
        if let Some((receiver, method)) = search::lua_method_step(value)
            && let Some(o) = self.lua_expr_owner(here, text, receiver, line, hops + 1)
            && let Some(table) = o.table.as_deref()
            && let [at] = search::lua_members(&o.source, table, method)[..]
            && search::lua_returns_self(&o.source, at)
        {
            return Some(o);
        }
        if let Some(parts) = search::lua_name_path(value) {
            return self.lua_owner(here, text, &parts, line, hops + 1);
        }
        let (callee, made_by) = search::lua_call(value)?;
        let o = self.lua_owner(here, text, &callee, line, hops + 1)?;
        let table = o.table.clone()?;
        let [line] = search::lua_members(&o.source, &table, &made_by)[..] else {
            return None;
        };
        search::lua_makes_instance(&o.source, line, &table).then_some(Owner {
            reason: Reason::Receiver(table),
            ..o
        })
    }
}
