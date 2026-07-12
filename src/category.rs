/// Discriminants are explicit because `Category as usize` indexes
/// `Config::category_dirs` (see `Workspace::category_dir`) — reordering a
/// variant without updating its value here would silently break that
/// mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Inbox = 0,
    Project = 1,
    Area = 2,
    Resource = 3,
    Archive = 4,
}

impl Category {
    pub fn is_directory_style(&self) -> bool {
        matches!(self, Category::Project | Category::Area)
    }

    /// The fixed subfolder name under `Archive` an item of this category is
    /// filed under when archived, e.g. `Category::Project` -> `"Projects"`.
    /// Never called with `Category::Archive` itself.
    pub fn archive_origin_name(&self) -> &'static str {
        match self {
            Category::Inbox => "Inbox",
            Category::Project => "Projects",
            Category::Area => "Areas",
            Category::Resource => "Resources",
            Category::Archive => unreachable!("Archive has no archive origin subfolder"),
        }
    }

    /// The four categories an item can be archived from — every variant
    /// except `Archive`.
    pub fn archivable() -> [Category; 4] {
        [
            Category::Inbox,
            Category::Project,
            Category::Area,
            Category::Resource,
        ]
    }

    /// Plural display name for user-facing messages (`list`'s
    /// no-match/empty messages), covering all five variants including
    /// `Archive` itself. Shares strings with `archive_origin_name` for the
    /// four categories both cover, but is total where `archive_origin_name`
    /// is deliberately partial.
    pub fn display_name(&self) -> &'static str {
        match self {
            Category::Inbox => "Inbox",
            Category::Project => "Projects",
            Category::Area => "Areas",
            Category::Resource => "Resources",
            Category::Archive => "Archive",
        }
    }

    /// Singular lowercase name for machine-readable output (`list
    /// --json`'s `origin` field). Distinct from `archive_origin_name`
    /// (plural, capitalized, filesystem-facing) and `display_name`
    /// (plural, capitalized, human-facing) — this one is for JSON
    /// consumers to branch on without parsing a display string.
    pub fn key(&self) -> &'static str {
        match self {
            Category::Inbox => "inbox",
            Category::Project => "project",
            Category::Area => "area",
            Category::Resource => "resource",
            Category::Archive => "archive",
        }
    }
}

/// What `ishi new`/`ishi daily` create — a different vocabulary from
/// `Category` (where an item is filed). See design.md's "Filing
/// vocabulary vs. creation vocabulary" for why these are two types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Inbox,
    Project,
    Area,
    Resource,
    Daily,
}

impl Kind {
    /// The `Category` this kind files into. `Daily` maps to `Inbox` — a
    /// daily note has no folder of its own.
    pub fn category(&self) -> Category {
        match self {
            Kind::Inbox | Kind::Daily => Category::Inbox,
            Kind::Project => Category::Project,
            Kind::Area => Category::Area,
            Kind::Resource => Category::Resource,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_and_area_are_directory_style() {
        assert!(Category::Project.is_directory_style());
        assert!(Category::Area.is_directory_style());
    }

    #[test]
    fn inbox_resource_archive_are_not_directory_style() {
        assert!(!Category::Inbox.is_directory_style());
        assert!(!Category::Resource.is_directory_style());
        assert!(!Category::Archive.is_directory_style());
    }

    #[test]
    fn archive_origin_name_maps_each_archivable_category() {
        assert_eq!(Category::Inbox.archive_origin_name(), "Inbox");
        assert_eq!(Category::Project.archive_origin_name(), "Projects");
        assert_eq!(Category::Area.archive_origin_name(), "Areas");
        assert_eq!(Category::Resource.archive_origin_name(), "Resources");
    }

    #[test]
    fn archivable_excludes_archive() {
        assert_eq!(
            Category::archivable(),
            [
                Category::Inbox,
                Category::Project,
                Category::Area,
                Category::Resource
            ]
        );
    }

    #[test]
    fn display_name_covers_all_five_variants() {
        assert_eq!(Category::Inbox.display_name(), "Inbox");
        assert_eq!(Category::Project.display_name(), "Projects");
        assert_eq!(Category::Area.display_name(), "Areas");
        assert_eq!(Category::Resource.display_name(), "Resources");
        assert_eq!(Category::Archive.display_name(), "Archive");
    }

    #[test]
    fn key_returns_singular_lowercase_names_for_all_five_variants() {
        assert_eq!(Category::Inbox.key(), "inbox");
        assert_eq!(Category::Project.key(), "project");
        assert_eq!(Category::Area.key(), "area");
        assert_eq!(Category::Resource.key(), "resource");
        assert_eq!(Category::Archive.key(), "archive");
    }

    #[test]
    fn kind_category_maps_correctly() {
        assert_eq!(Kind::Inbox.category(), Category::Inbox);
        assert_eq!(Kind::Project.category(), Category::Project);
        assert_eq!(Kind::Area.category(), Category::Area);
        assert_eq!(Kind::Resource.category(), Category::Resource);
        assert_eq!(Kind::Daily.category(), Category::Inbox);
    }
}
