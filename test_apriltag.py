import json

import cv2
from pupil_apriltags import Detector
import rerun as rr
import numpy as np
from scipy.spatial.transform import Rotation as R

rr.init("april_tag_viewer", spawn=True)

detector = Detector(
    families='tag36h11',
    nthreads=1,
    quad_decimate=1.0,
    quad_sigma=0.0,
    refine_edges=1,
    decode_sharpening=0.25,
    debug=0
)

cap = cv2.VideoCapture(2)

tag_size = 160 * 0.001 # 160mm

tag_points = np.array([
    [-tag_size/2,  tag_size/2, 0.0],
    [ tag_size/2,  tag_size/2, 0.0],
    [ tag_size/2, -tag_size/2, 0.0],
    [-tag_size/2, -tag_size/2, 0.0],
], dtype=np.float32)

with open('/Users/martin/Downloads/calibration.json', 'r') as file:
    data = json.load(file)

def into_transform_matrix(p, q):
    T = np.eye(4)
    T[:3, :3] = R.from_quat(np.array([q[0], -q[1], -q[2], q[3]])).as_matrix()
    T[:3, 3] = np.array(p)

    return T

T_imu_to_camera = into_transform_matrix(data['SLAM_camera']['device_1']['imu_p_cam'], data['SLAM_camera']['device_1']['imu_q_cam'])
T_imu_display_left = into_transform_matrix(data['display']['target_p_left_display'], data['display']['target_q_left_display'])
T_imu_to_display_right = into_transform_matrix(data['display']['target_p_right_display'], data['display']['target_q_right_display'])
K_left_display = np.array(data['display']['k_left_display'], dtype=float).reshape(3, 3)
K_right_display = np.array(data['display']['k_right_display'], dtype=float).reshape(3, 3)

T_cam_to_right_display = T_imu_to_display_right @ T_imu_to_camera # T_imu_to_display_right @ np.linalg.inv(T_imu_to_camera)

rgb_camera = data['RGB_camera']['device_1']
c_x, c_y = rgb_camera['cc']
f_x, f_y = rgb_camera['fc']
r_x, r_y = rgb_camera['resolution']

c_x = c_x / r_x * 1920.0
c_y = c_y / r_y * 1080.0

K_rgb_camera = np.array([
    [f_x, 0, c_x],
    [0, f_y, c_y],
    [0,  0,  1]
], dtype=np.float64)

distortion = np.array(rgb_camera["kc"], dtype=np.float64)

slam_camera = data['SLAM_camera']

cv2.namedWindow("AprilTag Viewer", cv2.WINDOW_NORMAL)

while cap.isOpened():
    ret, frame = cap.read()
    if not ret:
        break

    gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)

    detections = detector.detect(gray)

    rr.log("video/frame", rr.Image(frame))

    positions = []
    for det in detections:
        corners = det.corners

        ok, rvec, tvec = cv2.solvePnP(
            tag_points, det.corners, K_rgb_camera, distortion,
            flags=cv2.SOLVEPNP_IPPE_SQUARE
        )
        assert ok

        R, _ = cv2.Rodrigues(rvec)
        t = tvec.reshape(3, 1)
        T = np.eye(4)
        T[:3, :3] = R
        T[:3, 3] = t[:, 0]

        tag_pts_h = np.hstack(
            [tag_points.astype(np.float64), np.ones((4, 1), np.float64)]
        )
        cam_pts_h = (T @ tag_pts_h.T).T
        cam_pts = (cam_pts_h[:, :3] / cam_pts_h[:, 3:4]).astype(np.float32)

        tri_indices = np.array(
            [
                [0, 1, 2],
                [0, 2, 3]
            ], dtype=np.uint32
        )

        rr.log(
            "tag/mesh",
            rr.Mesh3D(
                vertex_positions=cam_pts,
                triangle_indices=tri_indices,
            ),
        )

        T_right_from_tag = T_cam_to_right_display @ T
        R_right_from_tag = T_right_from_tag[:3, :3]
        t_right_from_tag = T_right_from_tag[:3, 3].reshape(3, 1)

        rvec_right, _ = cv2.Rodrigues(R_right_from_tag)

        # todo: apply distortion

        uv_right, _ = cv2.projectPoints(
            tag_points,
            rvec_right,
            t_right_from_tag,
            K_right_display,
            None,
        )
        uv_right = uv_right.reshape(-1, 2).astype(np.int32)

#         frame = np.zeros((1080, 1920, 3), dtype=np.uint8)

        cv2.polylines(frame, [uv_right], True, (0, 255, 0), 2)

        strip = np.append(corners, corners[0:1], axis=0)
        positions.append(strip.tolist())

    if positions:
        num_dets = len(positions)
        rr.log("video/tags", rr.LineStrips2D(positions))

    cv2.imshow("AprilTag Viewer", frame)

    key = cv2.waitKey(1) & 0xFF
    if key == ord('q'):
        break

# Release resources
cap.release()
cv2.destroyAllWindows()
