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
        word: &str,
    ) -> Vec<Candidate> {
        let Some(o) = self.lua_owner(here, text, chain, 0) else {
            return Vec::new();
        };
        let Some(table) = o.table.as_deref() else {
            return Vec::new();
        };
        search::lua_members(&o.source, table, word)
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

    fn lua_owner(&self, here: &Path, text: &str, chain: &[String], hops: usize) -> Option<Owner> {
        let (first, rest) = chain.split_first()?;
        if hops > 4 {
            return None;
        }
        let mut owner = if first == "self" {
            let table = search::lua_self_table(text, self.line + 1)?;
            let parts: Vec<String> = table.split('.').map(str::to_owned).collect();
            Owner {
                reason: Reason::Receiver(table),
                ..self.lua_owner(here, text, &parts, hops + 1)?
            }
        } else {
            self.lua_value_owner(here, text, chain, hops)?
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
        hops: usize,
    ) -> Option<Owner> {
        let first = chain.first()?;
        let value = search::lua_local_value(text, self.line + 1, first)?;
        if let Some(module) = search::lua_required(&value) {
            let path = search::lua_module_file(module, &self.files)?;
            let source = self.text_of(&path)?;
            return Some(Owner {
                table: search::lua_returned(&source),
                reason: Reason::Import(module.to_owned()),
                path,
                source,
            });
        }
        if value.starts_with('{') {
            return Some(Owner {
                path: here.to_path_buf(),
                reason: Reason::Path(chain.join(".")),
                source: self.text_of(here)?,
                table: Some(first.clone()),
            });
        }
        let (callee, made_by) = search::lua_call(&value)?;
        let o = self.lua_owner(here, text, &callee, hops + 1)?;
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
