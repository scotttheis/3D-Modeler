//! Bounded VRML97 mesh scene reader. No scripts, URLs or external resources execute.
//! Supports static Transform/Group/Shape scenes with IndexedFaceSet geometry.
use crate::{
    CoreError, MAX_WRL_FACES, MAX_WRL_FILE_BYTES, MAX_WRL_OBJECTS, MAX_WRL_VERTICES, Mesh,
    Transform, WrlItem,
};
use std::{fs::File, io::Read, path::Path};

/// Reads a static WRL mesh scene in millimetres, preserving object transforms and materials.
/// Unsupported VRML nodes/fields fail explicitly before any scene mutation.
pub fn load_wrl(path: impl AsRef<Path>) -> Result<Vec<WrlItem>, CoreError> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_WRL_FILE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    load_wrl_bytes(&bytes)
}

/// Parses bounded UTF-8 VRML97 bytes without fetching external resources.
pub fn load_wrl_bytes(bytes: &[u8]) -> Result<Vec<WrlItem>, CoreError> {
    if bytes.len() as u64 > MAX_WRL_FILE_BYTES {
        return Err(error("file size limit exceeded"));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| error("expected UTF-8 VRML"))?;
    if !text.starts_with("#VRML V2.0 utf8") {
        return Err(error("expected #VRML V2.0 utf8 header"));
    }
    let mut parser = Parser {
        remaining: text,
        nodes: 0,
        vertices: 0,
        faces: 0,
    };
    let mut items = Vec::new();
    while parser.peek().is_some() {
        items.extend(parser.node(0)?);
        if items.len() > MAX_WRL_OBJECTS {
            return Err(error("object limit exceeded"));
        }
    }
    if items.is_empty() {
        return Err(error("file contains no mesh objects"));
    }
    Ok(items)
}
fn error(message: impl std::fmt::Display) -> CoreError {
    CoreError::wrl(message)
}
struct Parser<'a> {
    remaining: &'a str,
    nodes: usize,
    vertices: usize,
    faces: usize,
}
impl<'a> Parser<'a> {
    fn token(&mut self) -> Option<&'a str> {
        loop {
            self.remaining = self
                .remaining
                .trim_start_matches(|c: char| c.is_whitespace() || c == ',');
            if self.remaining.starts_with('#') {
                self.remaining = self.remaining.split_once('\n').map_or("", |(_, rest)| rest);
            } else {
                break;
            }
        }
        let first = self.remaining.chars().next()?;
        let len = if "{}[]".contains(first) {
            1
        } else if first == '"' {
            let mut escaped = false;
            let mut end = None;
            for (i, ch) in self.remaining.char_indices().skip(1) {
                if ch == '"' && !escaped {
                    end = Some(i + 1);
                    break;
                }
                escaped = ch == '\\' && !escaped;
            }
            end.unwrap_or(self.remaining.len())
        } else {
            self.remaining
                .find(|c: char| c.is_whitespace() || ",{}[]#".contains(c))
                .unwrap_or(self.remaining.len())
        };
        let (token, rest) = self.remaining.split_at(len);
        self.remaining = rest;
        Some(token)
    }
    fn peek(&self) -> Option<&'a str> {
        let mut copy = Self {
            remaining: self.remaining,
            nodes: 0,
            vertices: 0,
            faces: 0,
        };
        copy.token()
    }
    fn next(&mut self) -> Result<&'a str, CoreError> {
        self.token().ok_or_else(|| error("unexpected end of WRL"))
    }
    fn want(&mut self, token: &str) -> Result<(), CoreError> {
        let got = self.next()?;
        if got == token {
            Ok(())
        } else {
            Err(error(format!("expected {token}, found {got}")))
        }
    }
    fn number(&mut self) -> Result<f32, CoreError> {
        self.next()?
            .parse::<f32>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or_else(|| error("expected finite number"))
    }
    fn vector(&mut self) -> Result<[f32; 3], CoreError> {
        Ok([self.number()?, self.number()?, self.number()?])
    }
    fn boolean(&mut self) -> Result<bool, CoreError> {
        match self.next()? {
            "TRUE" => Ok(true),
            "FALSE" => Ok(false),
            _ => Err(error("expected TRUE or FALSE")),
        }
    }
    fn node(&mut self, depth: usize) -> Result<Vec<WrlItem>, CoreError> {
        self.nodes += 1;
        if depth > 64 || self.nodes > MAX_WRL_OBJECTS * 8 {
            return Err(error("node/depth limit exceeded"));
        }
        let mut kind = self.next()?;
        let mut name = "Imported".to_owned();
        if kind == "DEF" {
            name = self.next()?.to_owned();
            kind = self.next()?;
        }
        self.want("{")?;
        let mut items = Vec::new();
        let mut translation = [0.; 3];
        let mut scale = [1.; 3];
        let mut rotation = ([0., 0., 1.], 0.);
        let mut mesh = None;
        let mut color = None;
        while self.peek() != Some("}") {
            let field = self.next()?;
            match (kind, field) {
                ("WorldInfo", "title") => {
                    let value = self.next()?;
                    if !value.starts_with('"') || !value.ends_with('"') {
                        return Err(error("invalid title"));
                    }
                }
                ("Transform", "translation") => translation = self.vector()?,
                ("Transform", "scale") => scale = self.vector()?,
                ("Transform", "rotation") => rotation = (self.vector()?, self.number()?),
                ("Transform" | "Group", "children") => {
                    self.want("[")?;
                    while self.peek() != Some("]") {
                        items.extend(self.node(depth + 1)?);
                        if items.len() > MAX_WRL_OBJECTS {
                            return Err(error("object limit exceeded"));
                        }
                    }
                    self.want("]")?;
                }
                ("Shape", "appearance") => color = Some(self.appearance()?),
                ("Shape", "geometry") => mesh = Some(self.geometry()?),
                _ => {
                    return Err(error(format!(
                        "unsupported VRML node or field: {kind}.{field}"
                    )));
                }
            }
        }
        self.want("}")?;
        if kind == "Shape" {
            items.push(WrlItem::new(
                mesh.ok_or_else(|| error("Shape has no geometry"))?,
                Transform::identity(),
                name,
                color,
            )?);
        } else if kind == "Transform" {
            let transform = Transform::identity()
                .translated(translation)?
                .rotated(rotation.1.to_degrees(), rotation.0)?
                .scaled(scale)?;
            for item in &mut items {
                item.transform = transform.multiply(&item.transform)?;
                if item.name == "Imported" {
                    item.name.clone_from(&name);
                }
            }
        } else if kind != "Group" && kind != "WorldInfo" {
            return Err(error(format!("unsupported VRML node: {kind}")));
        }
        Ok(items)
    }
    fn appearance(&mut self) -> Result<[f32; 4], CoreError> {
        self.want("Appearance")?;
        self.want("{")?;
        self.want("material")?;
        self.want("Material")?;
        self.want("{")?;
        let mut color = [0.8, 0.8, 0.8, 1.];
        while self.peek() != Some("}") {
            match self.next()? {
                "diffuseColor" => {
                    let rgb = self.vector()?;
                    color[..3].copy_from_slice(&rgb);
                }
                "transparency" => color[3] = 1. - self.number()?,
                "ambientIntensity" | "shininess" => {
                    self.number()?;
                }
                "specularColor" | "emissiveColor" => {
                    self.vector()?;
                }
                field => return Err(error(format!("unsupported material field: {field}"))),
            }
        }
        self.want("}")?;
        self.want("}")?;
        Ok(color)
    }
    fn geometry(&mut self) -> Result<Mesh, CoreError> {
        self.want("IndexedFaceSet")?;
        self.want("{")?;
        let mut vertices = Vec::new();
        let mut faces = Vec::new();
        let mut ccw = true;
        while self.peek() != Some("}") {
            match self.next()? {
                "solid" | "convex" => {
                    self.boolean()?;
                }
                "ccw" => ccw = self.boolean()?,
                "creaseAngle" => {
                    self.number()?;
                }
                "coord" => {
                    self.want("Coordinate")?;
                    self.want("{")?;
                    self.want("point")?;
                    self.want("[")?;
                    while self.peek() != Some("]") {
                        self.vertices += 1;
                        if self.vertices > MAX_WRL_VERTICES {
                            return Err(error("vertex limit exceeded"));
                        }
                        vertices.push(self.vector()?);
                    }
                    self.want("]")?;
                    self.want("}")?;
                }
                "coordIndex" => {
                    self.want("[")?;
                    let mut polygon = Vec::new();
                    while self.peek() != Some("]") {
                        let index = self
                            .next()?
                            .parse::<i64>()
                            .map_err(|_| error("invalid vertex index"))?;
                        if index == -1 {
                            self.face(&mut polygon, &mut faces)?;
                        } else {
                            polygon.push(
                                u32::try_from(index).map_err(|_| error("invalid vertex index"))?,
                            );
                            if polygon.len() > 3 {
                                return Err(error(
                                    "WRL import currently requires triangulated faces",
                                ));
                            }
                        }
                    }
                    if !polygon.is_empty() {
                        self.face(&mut polygon, &mut faces)?;
                    }
                    self.want("]")?;
                }
                field => return Err(error(format!("unsupported IndexedFaceSet field: {field}"))),
            }
        }
        self.want("}")?;
        if !ccw {
            for face in &mut faces {
                face.swap(1, 2);
            }
        }
        Mesh::new(vertices, faces)
    }
    fn face(&mut self, polygon: &mut Vec<u32>, faces: &mut Vec<[u32; 3]>) -> Result<(), CoreError> {
        let face: [u32; 3] = polygon
            .as_slice()
            .try_into()
            .map_err(|_| error("expected triangular face"))?;
        self.faces += 1;
        if self.faces > MAX_WRL_FACES {
            return Err(error("face limit exceeded"));
        }
        faces.push(face);
        polygon.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exported_transforms_colors_and_multiple_shapes_round_trip() {
        let mesh = crate::make_box([2., 4., 6.]).expect("box");
        let transform = Transform::identity()
            .translated([12., -3., 4.])
            .expect("translation")
            .rotated(32., [0., 1., 0.])
            .expect("rotation")
            .scaled([2., 3., 4.])
            .expect("scale");
        let item = WrlItem::new(
            mesh,
            transform,
            "Part".to_owned(),
            Some([0.2, 0.4, 0.8, 0.5]),
        )
        .expect("item");
        let bytes = crate::export_wrl(&[item.clone(), item.clone()]).expect("export");
        let actual = load_wrl_bytes(&bytes).expect("import");
        assert_eq!(actual.len(), 2);
        for imported in actual {
            assert_eq!(imported.mesh, item.mesh);
            assert_eq!(imported.color, item.color);
            for (a, b) in imported
                .transform
                .rows()
                .iter()
                .flatten()
                .zip(transform.rows().iter().flatten())
            {
                assert!((a - b).abs() < 0.0001);
            }
        }
    }
    #[test]
    fn malformed_and_unsupported_vrml_fail_without_panicking() {
        for text in [
            "",
            "#VRML V2.0 utf8",
            "#VRML V2.0 utf8\nShape {",
            "#VRML V2.0 utf8\nScript {}",
            "#VRML V2.0 utf8\nTransform { translation NaN 0 0 }",
            "#VRML V2.0 utf8\nShape { geometry IndexedFaceSet { coord Coordinate { point [0 0 0,1 0 0,0 1 0] } coordIndex [0 1 999 -1] } }",
        ] {
            assert!(load_wrl_bytes(text.as_bytes()).is_err(), "{text}");
        }
        let nested = format!(
            "#VRML V2.0 utf8\n{}{}",
            "Group { children [".repeat(66),
            "] }".repeat(66)
        );
        assert!(load_wrl_bytes(nested.as_bytes()).is_err());
    }
}
