use super::{App, IMPORTED, Kind, Path, Typed, bound, search};

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
        let (taken, module) = path.split_last()?;
        (taken != name && module.first().is_some_and(|m| !m.starts_with(['.', '/'])))
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
}
