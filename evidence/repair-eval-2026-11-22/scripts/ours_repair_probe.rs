use modeler3d_core::{Mesh, repair_mesh};
fn main() {
    for p in std::env::args().skip(1) {
        let (inp, outp) = p.split_once('=').unwrap();
        let vb = std::fs::read(format!("{inp}.v")).unwrap();
        let tb = std::fs::read(format!("{inp}.t")).unwrap();
        let v: Vec<[f32; 3]> = vb.chunks(24).map(|c| { let f = |i: usize| f64::from_le_bytes(c[i*8..i*8+8].try_into().unwrap()) as f32; [f(0), f(1), f(2)] }).collect();
        let t: Vec<[u32; 3]> = tb.chunks(12).map(|c| { let f = |i: usize| u32::from_le_bytes(c[i*4..i*4+4].try_into().unwrap()); [f(0), f(1), f(2)] }).collect();
        let mesh = Mesh::new(v, t).unwrap();
        let t0 = std::time::Instant::now();
        match repair_mesh(&mesh) {
            Ok(r) => {
                println!("== {inp} ({} ms) nothing_to_fix={} can_apply={} before_valid={} after_valid={}", t0.elapsed().as_millis(), r.report.nothing_to_fix(), r.report.can_apply, r.report.before.valid, r.report.after.valid);
                for s in &r.report.stages { if s.changed || s.error.is_some() { println!("   {:<34} v {}->{} f {}->{} {:?}", s.name, s.before_vertices, s.after_vertices, s.before_triangles, s.after_triangles, s.error); } }
                println!("   planar patches={} changed={}", r.report.planar_cleanup.patches, r.report.planar_cleanup.changed);
                let mut vo = Vec::new(); for x in &r.mesh.vertices { for c in x { vo.extend_from_slice(&(f64::from(*c)).to_le_bytes()); } }
                let mut to = Vec::new(); for x in &r.mesh.faces { for c in x { to.extend_from_slice(&c.to_le_bytes()); } }
                std::fs::write(format!("{outp}.v"), vo).unwrap(); std::fs::write(format!("{outp}.t"), to).unwrap();
            }
            Err(e) => println!("== {inp} ERROR {e}"),
        }
    }
}
