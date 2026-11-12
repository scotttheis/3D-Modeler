"""Verify saved native pointer outcomes, independently of screenshots."""
import argparse, hashlib, json, zipfile, struct
from pathlib import Path
import numpy as np

root = Path(__file__).resolve().parent
parser = argparse.ArgumentParser()
parser.add_argument('--partial', action='store_true')
args = parser.parse_args()
def load(path):
    with zipfile.ZipFile(path) as archive:
        return json.loads(archive.read('scene.json'))
def matrices(scene):
    return np.array([o['transform'] for o in scene['objects']], dtype=np.float64)
records = []
routes = [json.loads(line) for line in (root/'rust-viewport-pointer-routes.ndjson').read_text(encoding='utf-8-sig').splitlines() if line.strip()]
latest={(r['Theme'],r['Case']):r for r in routes}
routes=list(latest.values())
for route in routes:
    theme, case = route['Theme'], route['Case']
    name = f'viewport-{case}'
    before = load(root/'native-viewport-fixtures'/f'{name}.3dp')
    prefix = route.get('OutputPrefix', f'rust-{theme}-{name}')
    after_path = root/'native-scene-states'/f'{prefix}-after.3dp'
    after = load(after_path)
    original, changed = matrices(before), matrices(after)
    ids = [o['object_id'] for o in before['objects']]
    assert ids == [o['object_id'] for o in after['objects']], (theme, case, 'identity')
    for a,b in zip(before['objects'], after['objects']):
        assert {k:v for k,v in a.items() if k!='transform'} == {k:v for k,v in b.items() if k!='transform'}, (theme,case,'metadata/visibility/mesh')
    committed = route['Termination'] == 'commit'
    assert after['scene_revision'] == before['scene_revision'] + int(committed), (theme,case,'history revision')
    if committed:
        targets = route['selection']
        assert all(not np.array_equal(original[i],changed[i]) for i in targets), (theme,case,'every target changes')
        assert all(np.array_equal(original[i],changed[i]) for i in range(len(ids)) if i not in targets), (theme,case,'unselected unchanged')
        if route['mode']=='move':
            assert np.array_equal(original[:,:3,:3],changed[:,:3,:3]), (theme,case,'move linear block')
            assert np.array_equal(original[:,1:,3],changed[:,1:,3]), (theme,case,'move untouched axes')
            offsets = changed[targets,0,3]-original[targets,0,3]
            assert np.max(offsets)-np.min(offsets)<1e-5, (theme,case,'group delta')
        if route['mode']=='rotate':
            for i in targets:
                assert np.allclose(np.linalg.svd(original[i,:3,:3])[1],np.linalg.svd(changed[i,:3,:3])[1],atol=2e-6), (theme,case,'rotation preserves stretch')
        if case.endswith('corner'):
            for i in targets:
                relative = changed[i,:3,:3]@np.linalg.inv(original[i,:3,:3])
                assert np.allclose(relative,np.eye(3)*relative[0,0],atol=2e-6), (theme,case,'uniform corner')
        if route['mode']=='resize' and not case.endswith('corner'):
            with zipfile.ZipFile(root/'native-viewport-fixtures'/f'{name}.3dp') as archive:
                for i in targets:
                    data=archive.read(before['objects'][i]['mesh_file'])
                    count=struct.unpack_from('<I',data,80)[0]
                    vertices=np.array([struct.unpack_from('<9f',data,84+50*j+12) for j in range(count)],dtype=np.float64).reshape(-1,3)
                    old=vertices@original[i,:3,:3].T+original[i,:3,3]
                    new=vertices@changed[i,:3,:3].T+changed[i,:3,3]
                    coordinates=old@np.array(route['axis_basis']).T
                    mask=np.isclose(coordinates[:,0],coordinates[:,0].min(),atol=1e-5)
                    assert np.any(mask), (theme,case,'opposite anchor exists')
                    assert np.max(np.abs(old[mask]-new[mask]))<1e-5, (theme,case,'opposite face/corner fixed')
        if case.endswith('corner'):
            center=np.array(route['gizmo_center'])
            for i in targets:
                delta=changed[i]@np.linalg.inv(original[i])
                assert np.allclose(delta[:3,:3]@center+delta[:3,3],center,atol=1e-5), (theme,case,'uniform corner preserves center, as Python')
        undo=load(root/'native-scene-states'/f'{prefix}-undo.3dp')
        redo=load(root/'native-scene-states'/f'{prefix}-redo.3dp')
        assert np.array_equal(matrices(undo),original), (theme,case,'undo exact')
        assert np.array_equal(matrices(redo),changed), (theme,case,'redo exact')
        assert [o['object_id'] for o in undo['objects']] == ids == [o['object_id'] for o in redo['objects']]
        assert [undo['scene_revision'],redo['scene_revision']] == [before['scene_revision']+2,before['scene_revision']+3]
    else:
        assert np.array_equal(original,changed), (theme,case,'cancel/no-op exact')
    records.append(dict(theme=theme,case=case,committed=committed,stable_ids=True,exact_undo_redo=committed,revision_before=before['scene_revision'],revision_after=after['scene_revision'],after_sha256=hashlib.sha256(after_path.read_bytes()).hexdigest()))
required={(theme,case) for theme in ['light','dark'] for case in ['move-single','move-hidden','move-group','rotate-single','rotate-group','resize-grid-face','resize-object-face','resize-corner','move-noop','move-cancel','move-focus']}
completed={(r['theme'],r['case']) for r in records}
missing=sorted(required-completed)
if not args.partial: assert not missing,missing
report=dict(verified=records,missing=missing,complete=not missing,source_escape_difference='Python leaves a changed pose without history; Rust cancels unchanged',source_focus_difference='Python completes the drag; Rust cancels unchanged')
(root/'native-viewport-verification.json').write_text(json.dumps(report,indent=2))
print('VERIFIED',len(records),'MISSING',len(missing))
