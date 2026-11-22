use std::time::Instant;
use windows::core::HSTRING;
use windows::Graphics::Printing3D::*;
use windows::Storage::{FileAccessMode, StorageFile};
use windows::Storage::Streams::{Buffer, DataReader, IBuffer};

fn bytes(b: &IBuffer) -> Vec<u8> {
    let r = DataReader::FromBuffer(b).unwrap();
    let n = b.Length().unwrap() as usize;
    let mut v = vec![0u8; n];
    r.ReadBytes(&mut v).unwrap();
    v
}

fn main() -> windows::core::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let (inp, outp, repair) = (&a[1], &a[2], a.get(3).map(|s| s != "norepair").unwrap_or(true));
    let f = StorageFile::GetFileFromPathAsync(&HSTRING::from(inp.as_str()))?.join()?;
    let s = f.OpenAsync(FileAccessMode::Read)?.join()?;
    let pkg = Printing3D3MFPackage::new()?;
    let t = Instant::now();
    let model = pkg.LoadModelFromPackageAsync(&s)?.join()?;
    println!("load ms {}", t.elapsed().as_millis());
    let t = Instant::now();
    if repair { model.RepairAsync()?.join()?; }
    println!("repair ms {}", t.elapsed().as_millis());
    let meshes = model.Meshes()?;
    for i in 0..meshes.Size()? {
        let m = meshes.GetAt(i)?;
        println!("mesh {i} verts={} tris={} vfmt={:?} tfmt={:?}", m.VertexCount()?, m.IndexCount()?,
            m.VertexPositionsDescription()?.Format, m.TriangleIndicesDescription()?.Format);
        std::fs::write(format!("{outp}.{i}.v"), bytes(&m.GetVertexPositions()?)).unwrap();
        std::fs::write(format!("{outp}.{i}.t"), bytes(&m.GetTriangleIndices()?)).unwrap();
    }
    let _ = Buffer::Create(1);
    Ok(())
}
