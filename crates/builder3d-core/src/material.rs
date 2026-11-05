//! Persisted material provenance for boolean-result faces.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Reserved scene metadata key for the source colors on a boolean result.
pub const BOOLEAN_COLOR_PROVENANCE_KEY: &str = "boolean_color_provenance";

/// Source colors and face labels retained for one boolean result.
///
/// Face labels are `0` for the target and `1` for the tool. Unknown source
/// labels from a boolean kernel are normalized to the target, matching the
/// Python reference's target-color fallback.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BooleanColorProvenance {
    /// Reference operation key: `merge`, `subtract`, or `intersect`.
    pub operation: String,
    /// Target and tool RGBA values before the operation.
    pub source_colors: [[f32; 4]; 2],
    /// One target/tool source label for each result face.
    pub face_sources: Vec<u8>,
    /// Number of result faces inherited from the tool operand.
    pub tool_face_count: usize,
}

impl BooleanColorProvenance {
    /// Validates the bounded face labels and finite RGBA source colors.
    #[must_use]
    pub fn is_valid_for_face_count(&self, face_count: usize) -> bool {
        self.face_sources.len() == face_count
            && self.face_sources.iter().all(|source| *source <= 1)
            && self
                .source_colors
                .iter()
                .flatten()
                .all(|channel| channel.is_finite() && (0.0..=1.0).contains(channel))
            && self.tool_face_count
                == self
                    .face_sources
                    .iter()
                    .filter(|source| **source == 1)
                    .count()
    }

    /// Returns one opaque RGBA8 color for every face.
    #[must_use]
    pub fn face_colors_rgba8(&self) -> Vec<[u8; 4]> {
        self.face_sources
            .iter()
            .map(|source| color_rgba8(self.source_colors[usize::from(*source)]))
            .collect()
    }
}

/// Parses valid boolean face-color metadata for a known mesh face count.
#[must_use]
pub fn boolean_color_provenance(
    metadata: &Value,
    face_count: usize,
) -> Option<BooleanColorProvenance> {
    let value = metadata.get(BOOLEAN_COLOR_PROVENANCE_KEY)?.clone();
    let provenance: BooleanColorProvenance = serde_json::from_value(value).ok()?;
    provenance
        .is_valid_for_face_count(face_count)
        .then_some(provenance)
}

/// Writes boolean face-color metadata while retaining other scene metadata.
#[must_use]
pub fn with_boolean_color_provenance(
    metadata: &Value,
    provenance: &BooleanColorProvenance,
) -> Value {
    let mut updated = metadata.clone();
    let Some(fields) = updated.as_object_mut() else {
        return metadata.clone();
    };
    let Ok(value) = serde_json::to_value(provenance) else {
        return metadata.clone();
    };
    fields.insert(BOOLEAN_COLOR_PROVENANCE_KEY.to_owned(), value);
    updated
}

/// Clears stale boolean face colors after a topology-changing mesh edit.
#[must_use]
pub fn without_boolean_color_provenance(metadata: &Value) -> Value {
    let mut updated = metadata.clone();
    if let Some(fields) = updated.as_object_mut() {
        fields.remove(BOOLEAN_COLOR_PROVENANCE_KEY);
    }
    updated
}

/// Updates the target palette entry after an authored color edit.
#[must_use]
pub fn with_boolean_target_color(metadata: &Value, color: [f32; 4]) -> Value {
    let mut updated = metadata.clone();
    let Some(fields) = updated.as_object_mut() else {
        return metadata.clone();
    };
    let Some(value) = fields.get(BOOLEAN_COLOR_PROVENANCE_KEY).cloned() else {
        return updated;
    };
    let Ok(mut provenance) = serde_json::from_value::<BooleanColorProvenance>(value) else {
        return updated;
    };
    if !provenance.is_valid_for_face_count(provenance.face_sources.len())
        || color
            .iter()
            .any(|channel| !channel.is_finite() || !(0.0..=1.0).contains(channel))
    {
        return updated;
    }
    provenance.source_colors[0] = color;
    if let Ok(value) = serde_json::to_value(provenance) {
        fields.insert(BOOLEAN_COLOR_PROVENANCE_KEY.to_owned(), value);
    }
    updated
}

/// Converts a normalized RGBA color to the byte colors used by 3MF and the
/// reference mesh renderer.
#[must_use]
pub fn color_rgba8(color: [f32; 4]) -> [u8; 4] {
    color.map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boolean_provenance_round_trips_and_updates_only_target_color() {
        let metadata = serde_json::json!({ "owner_note": "kept" });
        let provenance = BooleanColorProvenance {
            operation: String::from("subtract"),
            source_colors: [[0.2, 0.4, 0.6, 1.0], [0.8, 0.3, 0.1, 0.5]],
            face_sources: vec![0, 1, 0],
            tool_face_count: 1,
        };
        let metadata = with_boolean_color_provenance(&metadata, &provenance);
        assert_eq!(
            boolean_color_provenance(&metadata, 3),
            Some(provenance.clone())
        );

        let painted = with_boolean_target_color(&metadata, [0.7, 0.1, 0.2, 0.9]);
        let Some(updated) = boolean_color_provenance(&painted, 3) else {
            panic!("valid provenance should remain available after painting");
        };
        assert_eq!(updated.source_colors[0], [0.7, 0.1, 0.2, 0.9]);
        assert_eq!(updated.source_colors[1], provenance.source_colors[1]);
        assert_eq!(updated.face_sources, provenance.face_sources);
        assert_eq!(painted["owner_note"], "kept");
    }

    #[test]
    fn malformed_face_labels_are_ignored_and_can_be_removed() {
        let malformed = serde_json::json!({
            "boolean_color_provenance": {
                "operation": "subtract",
                "source_colors": [[0.2, 0.4, 0.6, 1.0], [0.8, 0.3, 0.1, 1.0]],
                "face_sources": [0, 2],
                "tool_face_count": 0
            }
        });
        assert!(boolean_color_provenance(&malformed, 2).is_none());
        let cleared = without_boolean_color_provenance(&malformed);
        assert!(cleared.get(BOOLEAN_COLOR_PROVENANCE_KEY).is_none());
    }
}
