use super::{CsType, CsWrittenType, Up};
use crate::app::{App, Path, PathBuf, search};

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
