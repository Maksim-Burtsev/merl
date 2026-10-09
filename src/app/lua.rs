use super::*;

impl App {
    pub(super) fn lua_qualified(
        &self,
        here: &Path,
        text: &str,
        chain: &[String],
        word: &str,
    ) -> Vec<Candidate> {
        let Some((first, rest)) = chain.split_first() else {
            return Vec::new();
        };
        let Some(value) = search::lua_local_value(text, self.line + 1, first) else {
            return Vec::new();
        };
        let required = search::lua_required(&value);
        let (path, reason) = match required {
            Some(module) => {
                let dir = module.replace('.', "/");
                let Some(path) = ["", "lua/"]
                    .iter()
                    .flat_map(|root| [format!("{root}{dir}.lua"), format!("{root}{dir}/init.lua")])
                    .map(PathBuf::from)
                    .find(|p| self.files.contains(p))
                else {
                    return Vec::new();
                };
                (path, Reason::Import(module.to_owned()))
            }
            None if value.starts_with('{') => (here.to_path_buf(), Reason::Path(chain.join("."))),
            None => return Vec::new(),
        };
        let Some(source) = self.text_of(&path) else {
            return Vec::new();
        };
        let mut table = match required {
            Some(_) => search::lua_returned(&source),
            None => Some(first.clone()),
        };
        for name in rest {
            let Some(next) = search::lua_table(&source, table.as_deref(), name) else {
                return Vec::new();
            };
            table = Some(next);
        }
        let Some(table) = table else {
            return Vec::new();
        };
        search::lua_members(&source, &table, word)
            .into_iter()
            .map(|line| Candidate {
                hit: Hit {
                    deleted: None,
                    path: path.clone(),
                    line1: line,
                    byte_col: None,
                    text: source.lines().nth(line - 1).unwrap_or_default().to_owned(),
                },
                reason: reason.clone(),
            })
            .collect()
    }
}
