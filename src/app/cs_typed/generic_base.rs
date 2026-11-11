use super::{App, CsType, Hit, Kind, Path, PathBuf, Typed, search};

impl App {
    pub(super) fn cs_substituted(
        &self,
        owner: &Typed,
        hit: &Hit,
        written: String,
    ) -> (PathBuf, usize, String) {
        self.cs_class_argument(owner, hit, &written)
            .unwrap_or_else(|| (hit.path.clone(), hit.line1, written))
    }

    fn cs_class_argument(
        &self,
        owner: &Typed,
        hit: &Hit,
        written: &str,
    ) -> Option<(PathBuf, usize, String)> {
        let at = self.text_of(&hit.path)?;
        let holder = search::cs_owner(&at, hit.line1)?;
        if search::cs_generic_param_line(&at, hit.line1, written) != Some(holder) {
            return None;
        }
        let name = search::cs_type_decl(at.lines().nth(holder - 1)?)?.name;
        let params = search::generic_header(Kind::CSharp, &at, holder, &name)?.params;
        let index = params.iter().position(|p| p == written)?;
        self.cs_base_argument(owner, (&hit.path, holder, params.len()), index, 0)
    }

    fn cs_base_argument(
        &self,
        t: &Typed,
        holder: (&Path, usize, usize),
        index: usize,
        depth: usize,
    ) -> Option<(PathBuf, usize, String)> {
        if depth > 8 {
            return None;
        }
        let text = self.text_of(&t.path)?;
        let params = search::generic_header(Kind::CSharp, &text, t.line, &t.name)?.params;
        let mut found = None;
        for entry in search::cs_bases(&text, t.line) {
            let Some(CsType::Project(b)) = self.cs_resolve(&t.path, &text, t.line, &entry) else {
                continue;
            };
            let got = if (b.path.as_path(), b.line) == (holder.0, holder.1) {
                let args = search::base_arguments(&entry, &search::cs_type_name(&entry)?)?;
                let arg = args.get(index).filter(|_| args.len() == holder.2)?;
                let own = arg
                    .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .any(|w| params.iter().any(|p| p == w));
                if own || arg.is_empty() {
                    return None;
                }
                Some((t.path.clone(), t.line, arg.clone()))
            } else {
                self.cs_base_argument(&b, holder, index, depth + 1)
            };
            if got.is_some() {
                if found.is_some() {
                    return None;
                }
                found = got;
            }
        }
        found
    }
}
