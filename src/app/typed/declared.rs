use super::{App, Candidate, Hit, IMPORTED, Kind, Path, Reason, Regex, Typed, bound, search};

impl App {
    pub(super) fn imported_type(
        &self,
        kind: Kind,
        file: &Path,
        written: &str,
        depth: usize,
    ) -> Option<Typed> {
        let c = IMPORTED.captures(written)?;
        let import = format!("import {{ {} }} from \"{}\";", &c[2], &c[1]);
        let path = search::imports(kind, &import).pop()?.path;
        self.type_from_module(kind, file, &path, &c[3], depth)
    }

    pub(super) fn package_alias(
        &self,
        kind: Kind,
        file: &Path,
        parts: &[String],
        depth: usize,
    ) -> Option<Typed> {
        let [name] = parts else {
            return None;
        };
        if kind != Kind::TsJs || !search::declaration_file(file) {
            return None;
        }
        let path = bound(&search::imports(kind, &self.text_of(file)?), name)?;
        let (_, module) = path.split_last()?;
        module
            .first()
            .is_some_and(|m| !m.starts_with(['.', '/']))
            .then(|| self.type_from_module(kind, file, &path, "", depth))
            .flatten()
    }

    pub(super) fn type_from_module(
        &self,
        kind: Kind,
        file: &Path,
        path: &[String],
        tail: &str,
        depth: usize,
    ) -> Option<Typed> {
        let (taken, module) = path.split_last()?;
        let mut files = search::module_files(kind, &self.root, &self.files, file, module);
        if files.is_empty()
            && module.first().is_some_and(|m| !m.starts_with(['.', '/']))
            && let Some((roots, all)) = self.external.get(&Kind::TsJs)
        {
            files = search::package_copy(&self.root, roots, all, &self.files, module)
                .map(|copy| copy.entries())
                .unwrap_or_default();
            files.retain(|f| search::declaration_file(f));
        }
        let [module] = &files[..] else {
            return None;
        };
        self.type_decl_at(kind, module, &format!("{taken}{tail}"), depth + 1)
    }

    pub(super) fn object_parameter_type(
        &self,
        kind: Kind,
        file: &Path,
        p: &search::TypedParameter,
    ) -> Option<String> {
        let object = self
            .type_decl(kind, file, &p.object_type)
            .filter(|t| self.is_interface(t))?;
        let [method] = &self.own_member(kind, &object, &p.method)?[..] else {
            return None;
        };
        let written = search::ts_method_parameter_type(&method.text, &p.method, p.index)?;
        let t = match p.object_type.rsplit_once('.') {
            Some((head, _)) => format!("{head}.{written}"),
            None => written,
        };
        (self.type_decl(kind, file, &t)?.path == object.path).then_some(t)
    }

    pub(super) fn indexed_field(
        &self,
        kind: Kind,
        ty: &Typed,
        text: &str,
        field: &str,
    ) -> Option<Typed> {
        let [base] = &search::bases(kind, text, ty.line)[..] else {
            return None;
        };
        let object = search::ts_object_type_argument(base)?;
        let decl = self.type_decl(kind, &ty.path, base)?;
        let [member] = &self.own_member(kind, &decl, field)?[..] else {
            return None;
        };
        let header = self
            .text_of(&decl.path)?
            .lines()
            .nth(decl.line - 1)?
            .to_owned();
        let key = search::ts_indexed_parameter(&header, &member.text, field)?;
        self.type_decl(kind, &ty.path, &search::ts_object_type_key(object, &key)?)
    }

    pub(in crate::app) fn typed_key(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        line1: usize,
        word: &str,
    ) -> Option<(Hit, String)> {
        let key = search::jsdoc_typed_key(text, line1, word)?;
        let mut ty = self
            .type_decl(kind, file, &key.object_type)
            .filter(|t| self.is_interface(t))?;
        for step in &key.steps {
            let (name, returned) = match step {
                search::KeyStep::Key(name) => (name, false),
                search::KeyStep::Returned(name) => (name, true),
            };
            let member = self.one_member(kind, &ty, name)?;
            let written = search::ts_member_type(&member.text, name, returned)?;
            let beside = || {
                let (head, _) = key.object_type.rsplit_once('.')?;
                self.type_decl(kind, file, &format!("{head}.{written}"))
                    .filter(|t| t.path == member.path)
            };
            ty = self
                .type_decl(kind, &member.path, &written)
                .or_else(beside)
                .filter(|t| self.is_interface(t))?;
        }
        self.one_member(kind, &ty, word).map(|hit| (hit, ty.name))
    }

    pub(in crate::app) fn store_member(
        &self,
        here: &Path,
        imports: &[search::Import],
        name: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let kind = Kind::TsJs;
        let (file, taken) = match bound(imports, name) {
            Some(path) => {
                let (taken, module) = path.split_last()?;
                let [file] = &search::module_files(kind, &self.root, &self.files, here, module)[..]
                else {
                    return None;
                };
                (file.clone(), taken.clone())
            }
            None => (here.to_path_buf(), name.to_owned()),
        };
        let text = self.text_of(&file)?;
        let taken = regex::escape(&taken);
        let declared = Regex::new(&format!(r"\b(?:const|let|var)\s+{taken}\b")).ok()?;
        let made = Regex::new(&format!(
            r"^\s*(?:export\s+)?const\s+{taken}\s*=\s*([A-Za-z_$][\w$]*)<\s*([A-Za-z_$][\w$]*)\s*>\s*\("
        ))
        .ok()?;
        let mut declarations = text
            .lines()
            .enumerate()
            .filter(|(_, l)| declared.is_match(l));
        let (Some((at, line)), None) = (declarations.next(), declarations.next()) else {
            return None;
        };
        let c = made.captures(line)?;
        let store = bound(&search::imports(kind, &text), &c[1])?;
        let store: Vec<&str> = store.iter().map(String::as_str).collect();
        if !matches!(
            store[..],
            ["svelte", "store", "writable" | "readable"] | ["svelte-persisted-store", "persisted"]
        ) {
            return None;
        }
        let ty = self.annotated_type(kind, &file, &text, at + 1, &c[2])?;
        let label = format!("${name}: {}", ty.name);
        let hits = self.hierarchy(kind, &ty, 0, &mut |t| self.instance_member(kind, t, word))?;
        Some(
            hits.into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::Receiver(label.clone()),
                })
                .collect(),
        )
    }

    fn one_member(&self, kind: Kind, ty: &Typed, word: &str) -> Option<Hit> {
        let hits = self.hierarchy(kind, ty, 0, &mut |t| self.instance_member(kind, t, word))?;
        let [hit] = <[Hit; 1]>::try_from(hits).ok()?;
        Some(hit)
    }
}
