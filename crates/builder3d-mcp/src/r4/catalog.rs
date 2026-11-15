//! The authoritative R4 MCP catalog.
//!
//! The Python gateway owns the reference catalog.  This Rust adapter keeps a
//! deliberately small, typed copy of its public metadata so profile filtering,
//! annotations, and dispatch all use the same ordered contract.

use std::fmt;

/// The monotonic launch profiles supported by the Builder3D MCP contract.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum Profile {
    /// Read-only inspection tools.
    Observe,
    /// Inspection plus model mutation tools.
    Model,
    /// Model tools plus approved-root file/project tools.
    Files,
    /// The complete catalog, including explicitly open-world slicer launch.
    #[default]
    Full,
}

impl Profile {
    /// Parses the exact environment spelling used by the Python gateway.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "observe" => Some(Self::Observe),
            "model" => Some(Self::Model),
            "files" => Some(Self::Files),
            "full" => Some(Self::Full),
            _ => None,
        }
    }

    /// Returns the stable wire/profile spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Model => "model",
            Self::Files => "files",
            Self::Full => "full",
        }
    }

    /// Returns whether this launch profile includes a tool requiring `needed`.
    #[must_use]
    pub const fn includes(self, needed: Self) -> bool {
        self as u8 >= needed as u8
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Stable metadata for one registered Builder3D tool.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ToolSpec {
    /// Exact MCP tool name and command name.
    pub name: &'static str,
    /// Stable Python/Rust category used in `_meta`.
    pub category: &'static str,
    /// Whether the tool changes the scene or external state.
    pub read_only: bool,
    /// Whether the operation may remove or replace user state.
    pub destructive: bool,
    /// Whether repeating the same request is semantically idempotent.
    pub idempotent: bool,
    /// Whether the tool reaches outside the closed Builder3D scene.
    pub open_world: bool,
    /// Whether the operation is tracked by the bounded operation registry.
    pub long_running: bool,
    /// Lowest profile that may publish this tool.
    pub profile: Profile,
}

impl ToolSpec {
    /// Returns whether this tool is visible in a launch profile.
    #[must_use]
    pub const fn visible_in(self, profile: Profile) -> bool {
        profile.includes(self.profile)
    }
}

/// Version of the shared Builder3D MCP catalog.
pub const CATALOG_VERSION: &str = "2026-09-25.1";

/// The exact reference order.  Keep this list append-only unless the Python
/// catalog version changes with an explicit compatibility decision.
pub const TOOL_CATALOG: [ToolSpec; 63] = [
    ToolSpec {
        name: "get_status",
        category: "inspection",
        read_only: true,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Observe,
    },
    ToolSpec {
        name: "operation_status",
        category: "inspection",
        read_only: true,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Observe,
    },
    ToolSpec {
        name: "cancel_operation",
        category: "operations",
        read_only: false,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "scene_summary",
        category: "inspection",
        read_only: true,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Observe,
    },
    ToolSpec {
        name: "print_check",
        category: "inspection",
        read_only: true,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Observe,
    },
    ToolSpec {
        name: "scene_changes",
        category: "inspection",
        read_only: true,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Observe,
    },
    ToolSpec {
        name: "apply_actions",
        category: "transforms",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "add_primitive",
        category: "creation",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "set_torus_parameters",
        category: "mesh",
        read_only: false,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "elastic_grab",
        category: "mesh",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "crease_polish",
        category: "mesh",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "smooth_brush",
        category: "mesh",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "blob",
        category: "mesh",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "normalize_face_density",
        category: "mesh",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: true,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "select",
        category: "selection",
        read_only: false,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "set_snapping",
        category: "transforms",
        read_only: false,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "translate",
        category: "transforms",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "set_transform",
        category: "transforms",
        read_only: false,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "transform_selection",
        category: "transforms",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "snap_translate",
        category: "transforms",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "settle",
        category: "transforms",
        read_only: false,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "mirror",
        category: "transforms",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "align",
        category: "transforms",
        read_only: false,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "arrange",
        category: "transforms",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "set_color",
        category: "appearance",
        read_only: false,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "sample_image_color",
        category: "appearance",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Files,
    },
    ToolSpec {
        name: "mesh_health",
        category: "inspection",
        read_only: true,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Observe,
    },
    ToolSpec {
        name: "repair",
        category: "mesh",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: true,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "simplify",
        category: "mesh",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: true,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "subdivide",
        category: "mesh",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: true,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "smooth",
        category: "mesh",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: true,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "import_file",
        category: "files",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: true,
        profile: Profile::Files,
    },
    ToolSpec {
        name: "export_file",
        category: "files",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: true,
        profile: Profile::Files,
    },
    ToolSpec {
        name: "undo",
        category: "history",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "redo",
        category: "history",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "delete_selected",
        category: "organization",
        read_only: false,
        destructive: true,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "copy_selected",
        category: "organization",
        read_only: false,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "cut_selected",
        category: "organization",
        read_only: false,
        destructive: true,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "paste",
        category: "organization",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "duplicate_selected",
        category: "organization",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "group_selected",
        category: "organization",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "ungroup_selected",
        category: "organization",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "boolean_merge",
        category: "mesh",
        read_only: false,
        destructive: true,
        idempotent: false,
        open_world: false,
        long_running: true,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "boolean_subtract",
        category: "mesh",
        read_only: false,
        destructive: true,
        idempotent: false,
        open_world: false,
        long_running: true,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "boolean_intersect",
        category: "mesh",
        read_only: false,
        destructive: true,
        idempotent: false,
        open_world: false,
        long_running: true,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "emboss_text",
        category: "mesh",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: true,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "hollow",
        category: "mesh",
        read_only: false,
        destructive: true,
        idempotent: false,
        open_world: false,
        long_running: true,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "view_state",
        category: "view",
        read_only: true,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Observe,
    },
    ToolSpec {
        name: "set_view",
        category: "view",
        read_only: false,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "frame_view",
        category: "view",
        read_only: false,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "orbit_view",
        category: "view",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "pan_view",
        category: "view",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "zoom_view",
        category: "view",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "view_preset",
        category: "view",
        read_only: false,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "new_project",
        category: "project",
        read_only: false,
        destructive: true,
        idempotent: false,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "open_project",
        category: "project",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: true,
        profile: Profile::Files,
    },
    ToolSpec {
        name: "save_project",
        category: "project",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: true,
        profile: Profile::Files,
    },
    ToolSpec {
        name: "save_project_as",
        category: "project",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: true,
        profile: Profile::Files,
    },
    ToolSpec {
        name: "discover_slicers",
        category: "print",
        read_only: true,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Files,
    },
    ToolSpec {
        name: "launch_slicer",
        category: "print",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        long_running: true,
        profile: Profile::Full,
    },
    // Appended for desktop parity (F12, owner decision D6). The first 60 keep
    // the reference order; these two mirror the desktop Rename and Show/Hide.
    ToolSpec {
        name: "rename_object",
        category: "organization",
        read_only: false,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    ToolSpec {
        name: "set_visibility",
        category: "organization",
        read_only: false,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
    // Owner batch 7, OF-44: the desktop Paint tool (whole object or region).
    ToolSpec {
        name: "paint",
        category: "appearance",
        read_only: false,
        destructive: false,
        idempotent: true,
        open_world: false,
        long_running: false,
        profile: Profile::Model,
    },
];

/// Finds a catalog entry by its exact case-sensitive wire name.
#[must_use]
pub fn find_tool(name: &str) -> Option<ToolSpec> {
    TOOL_CATALOG.iter().copied().find(|spec| spec.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_exactly_ordered_and_profile_monotonic() {
        assert_eq!(TOOL_CATALOG.len(), 63);
        assert_eq!(TOOL_CATALOG[0].name, "get_status");
        assert_eq!(TOOL_CATALOG[59].name, "launch_slicer");
        assert_eq!(TOOL_CATALOG[60].name, "rename_object");
        assert_eq!(TOOL_CATALOG[61].name, "set_visibility");
        assert_eq!(TOOL_CATALOG[62].name, "paint");
        assert!(
            TOOL_CATALOG
                .windows(2)
                .all(|window| window[0].name != window[1].name)
        );
        let observe = TOOL_CATALOG
            .iter()
            .filter(|spec| spec.visible_in(Profile::Observe))
            .count();
        let model = TOOL_CATALOG
            .iter()
            .filter(|spec| spec.visible_in(Profile::Model))
            .count();
        let files = TOOL_CATALOG
            .iter()
            .filter(|spec| spec.visible_in(Profile::Files))
            .count();
        assert_eq!((observe, model, files), (7, 55, 62));
        assert!(Profile::Full.includes(Profile::Full));
        assert!(!Profile::Files.includes(Profile::Full));
        assert_eq!(Profile::parse(" MODEL "), Some(Profile::Model));
        assert_eq!(Profile::parse("unknown"), None);
    }
}
