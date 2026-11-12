"""Read highlighted native Items rows from an already captured frame."""
import json,sys
from PIL import Image
with Image.open(sys.argv[1]) as frame:
 rows=[i for i in range(int(sys.argv[2])) if any(frame.getpixel((x,453+35*i))[:3] in [(225,230,255),(26,75,130)] for x in range(1120,1250))]
print(json.dumps(rows))
