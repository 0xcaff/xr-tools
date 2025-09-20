import cv2
import numpy as np
import json
from pathlib import Path

with open('/Users/martin/Downloads/calibration.json', 'r') as f:
    data = json.load(f)

arr = np.asarray(data["display_distortion"]["right_display"]["data"], dtype=np.float32).reshape(-1, 4)

U, V, Xp, Yp = arr[:, 0], arr[:, 1], arr[:, 2], arr[:, 3]
N = arr.shape[0]

def unique_sorted(vals, tol_decimals=6):
    vals_r = np.round(vals.astype(np.float64), tol_decimals)
    uniq = np.unique(vals_r)
    return len(uniq)

nx = unique_sorted(U)
ny = unique_sorted(V)

Xp_grid = Xp.reshape(ny, nx)
Yp_grid = Yp.reshape(ny, nx)

ideal = cv2.imread(str(Path('/Users/martin/Downloads/1920x1080-full-hd-nature-sunny-clouds-7fetljf7qyco7qsq.jpg')),
				   cv2.IMREAD_COLOR)

H, W = ideal.shape[:2]

mapX = cv2.resize(Xp_grid, (W, H), interpolation=cv2.INTER_CUBIC).astype(np.float32)
mapY = cv2.resize(Yp_grid, (W, H), interpolation=cv2.INTER_CUBIC).astype(np.float32)

resolved = cv2.remap(
    ideal, mapX, mapY,
    interpolation=cv2.INTER_LINEAR,
    borderMode=cv2.BORDER_CONSTANT,
    borderValue=0
)

print("Resolved image shape:", resolved.shape)
cv2.imwrite('/Users/martin/Downloads/resolved.jpg', resolved)
