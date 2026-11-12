"""Fail fast before undo/redo if physical input did not reach its target."""
import json,sys,zipfile
def load(path):return json.loads(zipfile.ZipFile(path).read('scene.json'))
before,after=map(load,sys.argv[1:3]);committed=sys.argv[3]=='commit'
assert before['project_id']==after['project_id'], 'project identity changed'
assert [o['object_id'] for o in before['objects']]==[o['object_id'] for o in after['objects']], 'wrong fixture/identity'
assert after['scene_revision']==before['scene_revision']+int(committed), 'native action did not commit exactly once'
assert (before['objects']!=after['objects'])==committed, 'native change/no-op mismatch'
print('NATIVE_STEP_VERIFIED',sys.argv[3])
