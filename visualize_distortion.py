import numpy as np
import matplotlib.pyplot as plt
import json

with open('/Users/martin/Downloads/calibration.json', 'r') as file:
    data = json.load(file)

lut_data = data['display_distortion']['left_display']['data']

# ---- load/reshape your LUT ----
# lut_data: flat list like you pasted. Each 4-tuple is [x, y, Xw, Yw]
arr = np.asarray(lut_data, np.float32).reshape(-1, 4)

# sort by (y,x) so we can reshape into a grid
arr = arr[np.lexsort((arr[:,0], arr[:,1]))]
xs = np.unique(arr[:,0]); ys = np.unique(arr[:,1])
nx, ny = len(xs), len(ys)

X  = arr[:,0].reshape(ny, nx)
Y  = arr[:,1].reshape(ny, nx)
Xw = arr[:,2].reshape(ny, nx)
Yw = arr[:,3].reshape(ny, nx)

dX = Xw - X
dY = Yw - Y
mag = np.hypot(dX, dY)

# ---- visualize displacements as arrows (downsample so it’s readable) ----
skip = max(1, (nx//32))  # adjust density
plt.figure(figsize=(8,6))
plt.quiver(X[::skip,::skip], Y[::skip,::skip], dX[::skip,::skip], dY[::skip,::skip], angles='xy', scale_units='xy', scale=1)
plt.gca().invert_yaxis()
plt.title('Display pre-warp displacement field (quiver)')
plt.xlabel('x'); plt.ylabel('y')
plt.axis('equal'); plt.tight_layout()
plt.show()

plt.figure(figsize=(7,5))
plt.imshow(mag, extent=[xs[0], xs[-1], ys[-1], ys[0]])
plt.title('Warp magnitude |(Xw,Yw)-(X,Y)|')
plt.xlabel('x'); plt.ylabel('y'); plt.colorbar(label='pixels')
plt.tight_layout()
plt.show()