# Python-produced multi-object import fixtures

These fixtures were created with the read-only Python reference at L:\Builder3D.
python_multi_object.gltf and python_multi_object.glb were written by
src.gltf_io.save_gltf from the same two named, colored, transformed objects.
The glTF buffer sidecars stay beside the document.

python_multi_object_cm.3mf was written by src.three_mf.save_3mf. Its object
vertices and transform translations were expressed in centimetres and the 3MF
model declares unit="centimeter". The reference loader reports the intended
millimetre geometry and transforms after applying that declared unit.

The first object is a 10 x 20 x 30 mm blue box with alpha 128/255 at
translation (4, 5, 6) mm. The second is a 2 x 3 x 4 mm red box at
(-2, 1, 3) mm. glTF stores these values in metres. The Rust app test passes a
foot override to each authoritative format and checks that it is ignored.
