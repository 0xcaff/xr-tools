use anyhow::bail;
use nalgebra::{Dyn, Isometry3, Matrix, Quaternion, Translation3, UnitQuaternion, VecStorage};
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
/// device factory configuration for XREAL One devices. the imu camera and the slam camera refer to
/// the same physical camera module.
pub struct Config {
    #[serde(rename = "FSN")]
    pub fsn: String,

    #[serde(rename = "IMU", deserialize_with = "deserialize_imu_device")]
    pub imu: ImuDevice,

    #[serde(rename = "RGB_camera", deserialize_with = "deserialize_rgb_camera")]
    pub rgb_camera: Option<CameraIntrinsicsRadial>,

    #[serde(rename = "SLAM_camera", deserialize_with = "deserialize_slam_camera")]
    pub slam_camera: Option<SlamCamera>,

    #[serde(deserialize_with = "deserialize_displays_config")]
    pub display: DisplaysConfig,

    pub display_distortion: DisplaysDistortion,

    #[serde(with = "last_modified_time_format")]
    pub last_modified_time: time::PrimitiveDateTime,
}

time::serde::format_description!(
    last_modified_time_format,
    PrimitiveDateTime,
    "[year]-[month]-[day] [hour]:[minute]:[second]"
);

impl Config {
    pub fn parse(data: &[u8]) -> Result<Self, anyhow::Error> {
        #[derive(Deserialize)]
        struct ConfigConstructor {
            glasses_version: usize,

            #[serde(flatten)]
            rest: Config,
        }

        let constructor = serde_json::from_slice::<ConfigConstructor>(data)?;

        if constructor.glasses_version != 8 {
            bail!("unexpected glasses version {}", constructor.glasses_version);
        }

        Ok(constructor.rest)
    }
}

pub struct DisplaysConfig {
    pub resolution: [f64; 2],
    pub left: DisplayConfig,
    pub right: DisplayConfig,
}

pub struct DisplayConfig {
    // todo: what frame is this in?
    /// K: 3x3 display/camera intrinsic matrix in row-major order:
    /// [ fx  0  cx ]
    /// [  0 fy  cy ]
    /// [  0  0   1 ]
    pub k: [f64; 9],

    /// Transform of the display in the IMU frame.
    pub transform: Isometry3<f64>,
}

fn deserialize_displays_config<'de, D>(deserializer: D) -> Result<DisplaysConfig, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct DisplaysConfigConstructor {
        k_left_display: [f64; 9],
        k_right_display: [f64; 9],
        num_of_displays: usize,
        resolution: [f64; 2],
        target_p_left_display: [f64; 3],
        target_p_right_display: [f64; 3],
        target_q_left_display: [f64; 4],
        target_q_right_display: [f64; 4],
        target_type: String,
    }

    let constructor = DisplaysConfigConstructor::deserialize(deserializer)?;
    if constructor.target_type != "IMU" {
        return Err(serde::de::Error::custom(format!(
            "unexpected target type {}",
            constructor.target_type
        )));
    }

    if constructor.num_of_displays != 2 {
        return Err(serde::de::Error::custom(format!(
            "unexpected number of displays {}",
            constructor.num_of_displays
        )));
    }

    Ok(DisplaysConfig {
        resolution: constructor.resolution,
        left: DisplayConfig {
            k: constructor.k_left_display,
            transform: Isometry3::from_parts(
                Translation3::from(constructor.target_p_left_display),
                UnitQuaternion::from_quaternion(Quaternion::from(
                    constructor.target_q_left_display,
                )),
            ),
        },
        right: DisplayConfig {
            k: constructor.k_right_display,
            transform: Isometry3::from_parts(
                Translation3::from(constructor.target_p_right_display),
                UnitQuaternion::from_quaternion(Quaternion::from(
                    constructor.target_q_right_display,
                )),
            ),
        },
    })
}

fn deserialize_rgb_camera<'de, D>(
    deserializer: D,
) -> Result<Option<CameraIntrinsicsRadial>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Constructor {
        num_of_cameras: usize,

        device_1: Option<CameraIntrinsicsRadialConstructor>,
    }

    #[derive(Deserialize)]
    struct CameraIntrinsicsRadialConstructor {
        #[serde(flatten, deserialize_with = "deserialize_camera_intrinsics_radial")]
        device: CameraIntrinsicsRadial,
    }

    let constructor = Constructor::deserialize(deserializer)?;

    let Some(camera) = constructor.device_1 else {
        return Ok(None);
    };

    if constructor.num_of_cameras != 1 {
        return Err(serde::de::Error::custom(format!(
            "unexpected number of cameras {}",
            constructor.num_of_cameras
        )));
    }

    Ok(Some(camera.device))
}

fn deserialize_camera_transform<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Isometry3<f64>, D::Error> {
    #[derive(Deserialize)]
    struct Constructor {
        #[serde(rename = "imu_p_cam")]
        camera_position: [f64; 3],

        #[serde(rename = "imu_q_cam")]
        camera_rotation: [f64; 4],
    }

    let constructor = Constructor::deserialize(deserializer)?;

    Ok(Isometry3::from_parts(
        Translation3::from(constructor.camera_position),
        UnitQuaternion::from_quaternion(Quaternion::from(constructor.camera_rotation)),
    ))
}

#[derive(Deserialize)]
pub struct SlamCamera {
    /// Transform of the camera in the IMU frame.
    #[serde(flatten, deserialize_with = "deserialize_camera_transform")]
    pub camera_transform: Isometry3<f64>,

    #[serde(flatten, deserialize_with = "deserialize_camera_intrinsics_radial")]
    pub intrinsics: CameraIntrinsicsRadial,
}

fn deserialize_slam_camera<'de, D>(deserializer: D) -> Result<Option<SlamCamera>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Constructor {
        num_of_cameras: usize,
        device_1: Option<SlamCamera>,
    }

    let constructor = Constructor::deserialize(deserializer)?;

    let Some(camera) = constructor.device_1 else {
        return Ok(None);
    };

    if constructor.num_of_cameras != 1 {
        return Err(serde::de::Error::custom(format!(
            "unexpected number of cameras {}",
            constructor.num_of_cameras
        )));
    }

    Ok(Some(camera))
}

#[derive(Deserialize)]
pub struct CameraIntrinsicsRadial {
    pub cc: [f64; 2],
    pub fc: [f64; 2],
    pub kc: DistortionCoefficients,
    pub resolution: [f64; 2],
    pub rolling_shutter_time: f64,
}

fn deserialize_camera_intrinsics_radial<'de, D>(
    deserializer: D,
) -> Result<CameraIntrinsicsRadial, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Constructor {
        camera_model: String,

        #[serde(flatten)]
        rest: CameraIntrinsicsRadial,
    }

    let constructor = Constructor::deserialize(deserializer)?;
    if constructor.camera_model != "radial" {
        return Err(serde::de::Error::custom(format!(
            "unexpected camera model {}",
            constructor.camera_model
        )));
    }

    Ok(constructor.rest)
}

#[derive(Deserialize)]
#[serde(from = "[f64; 5]")]
pub struct DistortionCoefficients {
    pub k1: f64,
    pub k2: f64,
    pub p1: f64,
    pub p2: f64,
    pub k3: f64,
}

impl From<[f64; 5]> for DistortionCoefficients {
    fn from([k1, k2, p1, p2, k3]: [f64; 5]) -> Self {
        Self { k1, k2, p1, p2, k3 }
    }
}

fn deserialize_imu_device<'de, D>(deserializer: D) -> Result<ImuDevice, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Constructor {
        num_of_imus: usize,
        device_1: ImuDeviceWrapped,
    }

    #[derive(Deserialize)]
    struct ImuDeviceWrapped {
        #[serde(flatten)]
        device: ImuDevice,

        accel_q_gyro: [f64; 4],
        gyro_g_sensitivity: [f64; 9],
        mag_bias: [f64; 3],
        scale_accel: [f64; 3],
        scale_gyro: [f64; 3],
        scale_mag: [f64; 3],
        skew_accel: [f64; 3],
        skew_gyro: [f64; 3],
        skew_mag: [f64; 3],
    }

    let constructor = Constructor::deserialize(deserializer)?;

    if constructor.num_of_imus != 1 {
        return Err(serde::de::Error::custom(format!(
            "unexpected number of imus {}",
            constructor.num_of_imus
        )));
    }

    if constructor.device_1.accel_q_gyro != [0.0, 0.0, 0.0, 1.0] {
        return Err(serde::de::Error::custom(format!(
            "unexpected accel_q_gyro {:?}",
            constructor.device_1.accel_q_gyro
        )));
    }

    if constructor.device_1.gyro_g_sensitivity != [0.0; 9] {
        return Err(serde::de::Error::custom(format!(
            "unexpected gyro_g_sensitivity {:?}",
            constructor.device_1.gyro_g_sensitivity
        )));
    }

    if constructor.device_1.mag_bias != [0.0; 3] {
        return Err(serde::de::Error::custom(format!(
            "unexpected mag_bias {:?}",
            constructor.device_1.mag_bias
        )));
    }

    if constructor.device_1.scale_accel != [1.0; 3] {
        return Err(serde::de::Error::custom(format!(
            "unexpected scale_accel {:?}",
            constructor.device_1.scale_accel
        )));
    }

    if constructor.device_1.scale_gyro != [1.0; 3] {
        return Err(serde::de::Error::custom(format!(
            "unexpected scale_accel {:?}",
            constructor.device_1.scale_gyro
        )));
    }

    if constructor.device_1.scale_mag != [1.0; 3] {
        return Err(serde::de::Error::custom(format!(
            "unexpected scale_mag {:?}",
            constructor.device_1.scale_mag
        )));
    }

    if constructor.device_1.skew_accel != [0.0; 3] {
        return Err(serde::de::Error::custom(format!(
            "unexpected skew_accel {:?}",
            constructor.device_1.skew_accel
        )));
    }

    if constructor.device_1.skew_gyro != [0.0; 3] {
        return Err(serde::de::Error::custom(format!(
            "unexpected skew_gyro {:?}",
            constructor.device_1.skew_gyro
        )));
    }

    if constructor.device_1.skew_mag != [0.0; 3] {
        return Err(serde::de::Error::custom(format!(
            "unexpected skew_mag {:?}",
            constructor.device_1.skew_mag
        )));
    }

    Ok(constructor.device_1.device)
}

fn deserialize_magnetometer_transform<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Isometry3<f64>, D::Error> {
    #[derive(Deserialize)]
    struct Constructor {
        #[serde(rename = "gyro_p_mag")]
        position: [f64; 3],

        #[serde(rename = "gyro_q_mag")]
        rotation: [f64; 4],
    }

    let constructor = Constructor::deserialize(deserializer)?;

    Ok(Isometry3::from_parts(
        Translation3::from(constructor.position),
        UnitQuaternion::from_quaternion(Quaternion::from(constructor.rotation)),
    ))
}

#[derive(Deserialize)]
pub struct ImuDevice {
    pub accel_bias: [f64; 3],
    pub bias_temperature: f64,

    // todo: use this bias or the interpolated one?
    pub gyro_bias: [f64; 3],
    pub gyro_bias_temp_data: GyroBiasValues,

    /// Transform of the magnetometer in the gyro frame.
    #[serde(flatten, deserialize_with = "deserialize_magnetometer_transform")]
    pub magnetometer_transform: Isometry3<f64>,
    pub imu_intrinsics: ImuIntrinsics,
    pub imu_noises: [f64; 4],
}

pub struct ImuIntrinsics {
    pub accl: SensorIntrinsics,
    pub gyro: SensorIntrinsics,
    pub static_detection_window_size: usize,
    pub temperature_mean: f64,
}

#[derive(Deserialize)]
pub struct GyroBiasValues(pub Vec<GyroBias>);

impl GyroBiasValues {
    pub fn interpolate(&self, temperature: f64) -> [f64; 3] {
        let idx = self.0.partition_point(|it| temperature < it.temp);

        if idx == 0 {
            self.0[0].bias
        } else if idx == self.0.len() {
            self.0[self.0.len() - 1].bias
        } else {
            let previous = &self.0[idx - 1];
            let next = &self.0[idx];

            let t = (temperature - previous.temp) / (next.temp - previous.temp);

            [
                previous.bias[0] * (1.0 - t) + next.bias[0] * t,
                previous.bias[1] * (1.0 - t) + next.bias[1] * t,
                previous.bias[2] * (1.0 - t) + next.bias[2] * t,
            ]
        }
    }
}

impl<'de> Deserialize<'de> for ImuIntrinsics {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Constructor {
            accel_pkpk: [f64; 3],
            accel_std: [f64; 3],
            accl_bias: [f64; 3],
            accl_calib_mat: [f64; 9],

            gyro_pkpk: [f64; 3],
            gyro_std: [f64; 3],
            gyro_bias: [f64; 3],
            gyro_calib_mat: [f64; 9],

            static_detection_window_size: usize,
            temperature_mean: f64,
        }

        let it = Constructor::deserialize(deserializer)?;

        Ok(ImuIntrinsics {
            accl: SensorIntrinsics {
                pkpk: it.accel_pkpk,
                std: it.accel_std,
                bias: it.accl_bias,
                calibration_matrix: it.accl_calib_mat,
            },
            gyro: SensorIntrinsics {
                pkpk: it.gyro_pkpk,
                std: it.gyro_std,
                bias: it.gyro_bias,
                calibration_matrix: it.gyro_calib_mat,
            },
            static_detection_window_size: it.static_detection_window_size,
            temperature_mean: it.temperature_mean,
        })
    }
}

pub struct SensorIntrinsics {
    pub pkpk: [f64; 3],
    pub std: [f64; 3],
    pub bias: [f64; 3],
    pub calibration_matrix: [f64; 9],
}

#[derive(Deserialize)]
pub struct GyroBias {
    pub bias: [f64; 3],
    pub temp: f64,
}

#[derive(Deserialize)]
pub struct DisplaysDistortion {
    pub left_display: DisplayDistortion,
    pub right_display: DisplayDistortion,
}

pub struct Point {
    pub u: f64,
    pub v: f64,
    pub x: f64,
    pub y: f64,
}

#[derive(Deserialize)]
#[serde(try_from = "DisplayDistortionConstructor")]
pub struct DisplayDistortion(pub Matrix<Point, Dyn, Dyn, VecStorage<Point, Dyn, Dyn>>);

// todo: helper to apply distortion
// https://github.com/0xcaff/xr-tools/blob/eea0baecf84915ecf026a92e2252afa3816a87e3/apply_distortion.py
// https://github.com/0xcaff/xr-tools/blob/eea0baecf84915ecf026a92e2252afa3816a87e3/test_apriltag.py#L113C26-L114

#[derive(Deserialize)]
struct DisplayDistortionConstructor {
    data: Vec<f64>,
    num_col: usize,
    num_row: usize,
    #[serde(rename = "type")]
    typ: usize,
}

impl TryFrom<DisplayDistortionConstructor> for DisplayDistortion {
    type Error = anyhow::Error;

    fn try_from(internal: DisplayDistortionConstructor) -> Result<Self, anyhow::Error> {
        if internal.typ != 1 {
            bail!("unexpected type {}", internal.typ);
        }

        let (chunks, tail) = internal.data.as_chunks::<4>();
        if tail.len() != 0 {
            bail!("unexpected tail {:?}", tail);
        }

        if chunks.len() != internal.num_row * internal.num_col {
            bail!(
                "unexpected chunk size {} != {}",
                chunks.len(),
                internal.num_row * internal.num_col
            );
        }

        let points = chunks
            .into_iter()
            .map(|[u, v, x, y]| Point {
                u: *u,
                v: *v,
                x: *x,
                y: *y,
            })
            .collect::<Vec<_>>();

        Ok(DisplayDistortion(Matrix::<
            Point,
            Dyn,
            Dyn,
            VecStorage<Point, Dyn, Dyn>,
        >::from_vec_storage(
            VecStorage::new(Dyn(internal.num_row), Dyn(internal.num_col), points),
        )))
    }
}
