use crate::{
    ViewportError,
    camera::Camera,
    math::{Ray, Vec3, normalize, ray_triangle},
    scene::{MAX_RENDER_OBJECTS, RenderScene},
};

/// Maximum authoritative triangle count accepted by one exact pick pass.
pub const MAX_EXACT_PICK_TRIANGLES: u64 = 30_000_000;

/// A precise hit on authoritative scene geometry.
#[derive(Clone, Debug, PartialEq)]
pub struct PickHit {
    /// Durable scene object ID.
    pub object_id: String,
    /// Authoritative face index within the object mesh.
    pub face_index: u64,
    /// World-space distance from the pick ray origin.
    pub distance: f32,
    /// World-space intersection position.
    pub position: Vec3,
    /// Barycentric coordinates in face order.
    pub barycentric: [f32; 3],
}

/// Exact CPU picking over the retained full-resolution geometry.
pub fn pick_scene(
    scene: &RenderScene,
    camera: &Camera,
    screen_x: f32,
    screen_y: f32,
) -> Result<Option<PickHit>, ViewportError> {
    if scene.objects().len() > MAX_RENDER_OBJECTS {
        return Err(ViewportError::CapacityExceeded(String::from(
            "pick scene object count exceeds the bounded limit",
        )));
    }
    let scene_radius = scene
        .world_bounds()?
        .map(|bounds| {
            let extent = [
                bounds.maximum[0] - bounds.minimum[0],
                bounds.maximum[1] - bounds.minimum[1],
                bounds.maximum[2] - bounds.minimum[2],
            ];
            (extent[0] * extent[0] + extent[1] * extent[1] + extent[2] * extent[2]).sqrt() * 0.5
        })
        .unwrap_or(1.0);
    let ray = camera.screen_ray(screen_x, screen_y, scene_radius)?;
    pick_ray(scene, ray)
}

/// Exact CPU picking from an already constructed world-space ray.
pub fn pick_ray(scene: &RenderScene, ray: Ray) -> Result<Option<PickHit>, ViewportError> {
    let mut nearest: Option<PickHit> = None;
    for object in scene.objects() {
        let world_bounds = object.world_bounds()?;
        if world_bounds.ray_entry(ray).is_none() {
            continue;
        }
        let inverse = object.transform().inverse()?;
        let local_origin_h = inverse.transform4([ray.origin[0], ray.origin[1], ray.origin[2], 1.0]);
        let local_direction_h =
            inverse.transform4([ray.direction[0], ray.direction[1], ray.direction[2], 0.0]);
        if local_origin_h[3].abs() <= f32::EPSILON {
            return Err(ViewportError::InvalidInput(String::from(
                "object transform produced an invalid pick origin",
            )));
        }
        let local_origin = [
            local_origin_h[0] / local_origin_h[3],
            local_origin_h[1] / local_origin_h[3],
            local_origin_h[2] / local_origin_h[3],
        ];
        let local_direction = normalize(
            [
                local_direction_h[0],
                local_direction_h[1],
                local_direction_h[2],
            ],
            "local pick direction",
        )?;
        let local_ray = Ray::new(local_origin, local_direction)?;
        if object.full_geometry().face_count() as u64 > MAX_EXACT_PICK_TRIANGLES {
            return Err(ViewportError::CapacityExceeded(String::from(
                "authoritative mesh exceeds exact-pick triangle limit",
            )));
        }
        for face_index in 0..object.full_geometry().face_count() {
            let Some([first, second, third]) = object.full_geometry().triangle(face_index) else {
                return Err(ViewportError::InvalidInput(String::from(
                    "authoritative triangle index is truncated",
                )));
            };
            let Some(first) = object.full_geometry().position(first) else {
                return Err(ViewportError::InvalidInput(String::from(
                    "authoritative pick vertex is unavailable",
                )));
            };
            let Some(second) = object.full_geometry().position(second) else {
                return Err(ViewportError::InvalidInput(String::from(
                    "authoritative pick vertex is unavailable",
                )));
            };
            let Some(third) = object.full_geometry().position(third) else {
                return Err(ViewportError::InvalidInput(String::from(
                    "authoritative pick vertex is unavailable",
                )));
            };
            let Some((local_distance, barycentric)) = ray_triangle(local_ray, first, second, third)
            else {
                continue;
            };
            let local_hit = local_ray.at(local_distance);
            let world_hit_h =
                object
                    .transform()
                    .transform4([local_hit[0], local_hit[1], local_hit[2], 1.0]);
            if world_hit_h[3].abs() <= f32::EPSILON {
                continue;
            }
            let world_hit = [
                world_hit_h[0] / world_hit_h[3],
                world_hit_h[1] / world_hit_h[3],
                world_hit_h[2] / world_hit_h[3],
            ];
            let delta = [
                world_hit[0] - ray.origin[0],
                world_hit[1] - ray.origin[1],
                world_hit[2] - ray.origin[2],
            ];
            let distance = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
            if !distance.is_finite() {
                continue;
            }
            let replace = nearest
                .as_ref()
                .is_none_or(|current| distance < current.distance);
            if replace {
                nearest = Some(PickHit {
                    object_id: object.object_id().to_owned(),
                    face_index: face_index as u64,
                    distance,
                    position: world_hit,
                    barycentric,
                });
            }
        }
    }
    Ok(nearest)
}

#[cfg(test)]
mod tests {
    use modeler3d_core::{Scene, SceneObject, make_box};

    use super::*;
    use crate::{camera::Camera, scene::RenderScene};

    #[test]
    fn exact_pick_uses_full_authoritative_faces() {
        let mesh = make_box([2.0, 2.0, 2.0]).map_err(|_| ()).ok();
        let Some(mesh) = mesh else {
            return;
        };
        let mut core_scene = Scene::new();
        assert!(core_scene.add_object(SceneObject::new(mesh)).is_ok());
        let render_scene = RenderScene::from_core(&core_scene).map_err(|_| ()).ok();
        let Some(render_scene) = render_scene else {
            return;
        };
        let mut camera = Camera::new();
        assert!(camera.set_viewport_size(800, 600).is_ok());
        let bounds = render_scene.world_bounds().ok().flatten();
        let Some(bounds) = bounds else {
            return;
        };
        assert!(camera.frame_bounds(&[bounds]).is_ok());
        let hit = pick_scene(&render_scene, &camera, 400.0, 300.0);
        assert!(hit.is_ok());
        assert!(hit.ok().flatten().is_some());
    }

    #[test]
    fn occluded_pick_keeps_the_nearest_stable_id_after_reordering_and_hiding() {
        let mesh = make_box([4.0; 3]).map_err(|_| ()).ok();
        let Some(mesh) = mesh else {
            return;
        };
        let mut core_scene = Scene::new();
        let back = SceneObject::new(mesh.clone());
        let back_id = back.object_id().to_owned();
        let front = SceneObject::new(mesh);
        let front_id = front.object_id().to_owned();
        assert!(core_scene.add_object(back).is_ok());
        assert!(core_scene.add_object(front).is_ok());
        assert!(core_scene.translate(1, [0.0, -1.0, 0.0]).is_ok());

        let mut camera = Camera::new();
        assert!(camera.set_viewport_size(800, 600).is_ok());
        assert!(camera.set_view(0.0, -89.0).is_ok());
        let render_scene = RenderScene::from_core(&core_scene).map_err(|_| ()).ok();
        let Some(render_scene) = render_scene else {
            return;
        };
        assert_eq!(
            pick_scene(&render_scene, &camera, 400.0, 300.0)
                .ok()
                .flatten()
                .map(|hit| hit.object_id),
            Some(front_id.clone())
        );

        let mut reordered = core_scene.snapshot();
        reordered.objects.swap(0, 1);
        assert!(core_scene.restore_snapshot(reordered).is_ok());
        let render_scene = RenderScene::from_core(&core_scene).map_err(|_| ()).ok();
        let Some(render_scene) = render_scene else {
            return;
        };
        assert_eq!(
            pick_scene(&render_scene, &camera, 400.0, 300.0)
                .ok()
                .flatten()
                .map(|hit| hit.object_id),
            Some(front_id.clone())
        );

        let front_index = core_scene.object_index(&front_id).map_err(|_| ()).ok();
        let Some(front_index) = front_index else {
            return;
        };
        assert!(core_scene.set_visible(front_index, false).is_ok());
        let render_scene = RenderScene::from_core(&core_scene).map_err(|_| ()).ok();
        let Some(render_scene) = render_scene else {
            return;
        };
        assert_eq!(render_scene.objects().len(), 1);
        assert_eq!(
            pick_scene(&render_scene, &camera, 400.0, 300.0)
                .ok()
                .flatten()
                .map(|hit| hit.object_id),
            Some(back_id)
        );
    }
}
