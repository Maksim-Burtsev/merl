use super::super::typed::{Base, base_argument};
use super::{App, CsType, Hit, Kind, PathBuf, Typed, search};

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
        let (t, arg) = base_argument(owner, (index, params.len()), 0, &|t: &Typed| {
            let text = self.text_of(&t.path)?;
            let params = search::generic_header(Kind::CSharp, &text, t.line, &t.name)?.params;
            let bases = search::cs_bases(&text, t.line)
                .into_iter()
                .filter_map(|entry| {
                    let Some(CsType::Project(b)) = self.cs_resolve(&t.path, &text, t.line, &entry)
                    else {
                        return None;
                    };
                    Some(if (&b.path, b.line) == (&hit.path, holder) {
                        let name = search::cs_type_name(&entry);
                        Base::Holder(name.and_then(|n| search::base_arguments(&entry, &n)))
                    } else {
                        Base::Other(b)
                    })
                })
                .collect();
            Some((params, bases))
        })?;
        Some((t.path, t.line, arg))
    }
}
