use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

use crate::desktop::AppEntry;

pub struct AppSearch {
    matcher: SkimMatcherV2,
}

impl AppSearch {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    pub fn search<'a>(&self, query: &str, apps: &'a [AppEntry]) -> Vec<&'a AppEntry> {
        if query.is_empty() {
            return apps.iter().take(8).collect();
        }

        let mut scored = apps
            .iter()
            .filter_map(|app| {
                self.matcher
                    .fuzzy_match(&app.name, query)
                    .map(|score| (score, app))
            })
            .collect::<Vec<_>>();
        scored.sort_by_key(|entry| std::cmp::Reverse(entry.0));
        scored.into_iter().take(8).map(|(_, app)| app).collect()
    }
}

impl Default for AppSearch {
    fn default() -> Self {
        Self::new()
    }
}
