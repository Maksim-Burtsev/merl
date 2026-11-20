use super::{CsType, CsWrittenType, Proven, Up};
use crate::app::{App, Kind, Path, PathBuf, search};

impl App {
    pub(super) fn cs_position(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        of: search::CsValue,
        (index, count): (usize, usize),
        hops: usize,
    ) -> Option<CsWrittenType> {
        let (path, line, written) = match of {
            search::CsValue::Call { callee, awaited } => {
                self.cs_call_written(file, text, line1, &callee, awaited, hops)?
            }
            search::CsValue::Type(w) => (file.to_path_buf(), line1, w),
            _ => return None,
        };
        let (path, line, written) = search::cs_tuple_item(&written, index, count)
            .map(|item| (path.clone(), line, item))
            .or_else(|| self.cs_deconstruct_out(&path, line, &written, (index, count)))?;
        let at = self.text_of(&path)?;
        let ty = self.cs_resolve(&path, &at, line, &written)?;
        Some(CsWrittenType {
            ty,
            written,
            from: (path, line),
        })
    }

    pub(super) fn cs_tuple_head<'a>(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        name: &str,
        fields: &'a [String],
    ) -> Option<(Proven, &'a [String])> {
        let (field, rest) = fields.split_first()?;
        let lines: Vec<&str> = text.lines().collect();
        let bindings = search::bindings(Kind::CSharp, text, line1, name);
        let [b] = bindings.as_slice() else {
            return None;
        };
        let at = search::cs_written_line(&lines, b.line1, name);
        let search::CsValue::Call { callee, awaited } =
            search::cs_declared(lines.get(at - 1)?, name)
        else {
            return None;
        };
        let (path, line, tuple) = self.cs_call_written(file, text, at, &callee, awaited, 0)?;
        let written = search::cs_tuple_named(&tuple, field)?;
        let ty = self.cs_resolve(&path, &self.text_of(&path)?, line, &written)?;
        let link = format!("{name}.{field}: {}", search::cs_type_name(&written)?);
        let p = Proven {
            ty,
            link,
            written,
            from: (path, line),
        };
        Some((p, rest))
    }

    fn cs_deconstruct_out(
        &self,
        path: &Path,
        line: usize,
        written: &str,
        (index, count): (usize, usize),
    ) -> Option<(PathBuf, usize, String)> {
        let at = self.text_of(path)?;
        let CsType::Project(t) = self.cs_resolve(path, &at, line, written)? else {
            return None;
        };
        let Up::Found(hits) = self.cs_up(&t, "Deconstruct", 0) else {
            return None;
        };
        let mut fits = hits.iter().filter_map(|h| {
            let outs = search::cs_out_params(&h.text);
            (outs.len() == count).then_some((h, outs))
        });
        let (hit, outs) = fits.next()?;
        if fits.next().is_some() {
            return None;
        }
        Some((hit.path.clone(), hit.line1, outs.get(index)?.clone()))
    }
}
