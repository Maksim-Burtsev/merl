use super::JvmType;
use crate::app::{App, Path};

impl App {
    pub(super) fn jvm_head<'r>(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        first: &str,
        rest: &'r [String],
        outer: Option<&str>,
    ) -> Option<(JvmType, Vec<String>, &'r [String], bool)> {
        let chained = || {
            self.jvm_chain_head(here, text, before, first, outer)
                .map(|(ty, links)| (ty, links, rest, true))
        };
        let static_head = || {
            self.jvm_static_head(here, text, first, rest)
                .map(|(ty, links, rest)| (ty, links, rest, false))
        };
        if outer.is_none() && first.starts_with(|c: char| c.is_ascii_uppercase()) {
            return chained().or_else(|| match self.java_field_owner(here, text, first) {
                Some(owner) => self
                    .jvm_follow(
                        JvmType::Project(owner),
                        Vec::new(),
                        first,
                        &[first.to_owned()],
                        false,
                    )
                    .map(|(ty, links)| (ty, links, rest, true)),
                None => static_head(),
            });
        }
        static_head().or_else(chained)
    }
}
