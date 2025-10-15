use anyhow::bail;
use nalgebra::{Dyn, Matrix, VecStorage};
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
pub struct Config {
    #[serde(rename = "FSN")]
    pub fsn: String,

    #[serde(rename = "IMU", deserialize_with = "deserialize_imu_device")]
    pub imu: ImuDevice,

    #[serde(rename = "RGB_camera", deserialize_with = "deserialize_rgb_camera")]
    pub rgb_camera: CameraIntrinsicsRadial,

    #[serde(rename = "SLAM_camera", deserialize_with = "deserialize_slam_camera")]
    pub slam_camera: SlamCamera,

    #[serde(deserialize_with = "deserialize_displays_config")]
    pub display: DisplaysConfig,

    pub display_distortion: DisplaysDistortion,
    pub last_modified_time: time::PrimitiveDateTime,
}

impl Config {
    pub fn parse(data: &[u8]) -> Result<Self, anyhow::Error> {
        #[derive(Deserialize)]
        struct ConfigConstructor {
            glasses_version: usize,

            #[serde(flatten)]
            rest: Config,
        }

        let constructor = serde_json::from_slice::<ConfigConstructor>(data)?;

        if constructor.glasses_version != 1 {
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
    pub k: [f64; 9],
    pub target_p: [f64; 3],
    pub target_q: [f64; 4],
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
            target_p: constructor.target_p_left_display,
            target_q: constructor.target_q_left_display,
        },
        right: DisplayConfig {
            k: constructor.k_right_display,
            target_p: constructor.target_p_right_display,
            target_q: constructor.target_q_right_display,
        },
    })
}

fn deserialize_rgb_camera<'de, D>(deserializer: D) -> Result<CameraIntrinsicsRadial, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Constructor {
        num_of_cameras: usize,

        #[serde(deserialize_with = "deserialize_camera_intrinsics_radial")]
        device_1: CameraIntrinsicsRadial,
    }

    let camera = Constructor::deserialize(deserializer)?;
    if camera.num_of_cameras != 1 {
        return Err(serde::de::Error::custom(format!(
            "unexpected number of cameras {}",
            camera.num_of_cameras
        )));
    }

    Ok(camera.device_1)
}

#[derive(Deserialize)]
pub struct SlamCamera {
    pub imu_p_cam: [f64; 3],
    pub imu_q_cam: [f64; 4],

    #[serde(flatten, deserialize_with = "deserialize_camera_intrinsics_radial")]
    pub intrinsics: CameraIntrinsicsRadial,
}

fn deserialize_slam_camera<'de, D>(deserializer: D) -> Result<SlamCamera, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Constructor {
        num_of_cameras: usize,
        device_1: SlamCamera,
    }

    let constructor = Constructor::deserialize(deserializer)?;
    if constructor.num_of_cameras != 1 {
        return Err(serde::de::Error::custom(format!(
            "unexpected number of cameras {}",
            constructor.num_of_cameras
        )));
    }

    Ok(constructor.device_1)
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
        device_1: ImuDevice,
    }

    let constructor = Constructor::deserialize(deserializer)?;

    if constructor.num_of_imus != 1 {
        return Err(serde::de::Error::custom(format!(
            "unexpected number of imus {}",
            constructor.num_of_imus
        )));
    }

    Ok(constructor.device_1)
}

#[derive(Deserialize)]
pub struct ImuDevice {
    pub accel_bias: [f64; 3],
    pub accel_q_gyro: [f64; 4],
    pub bias_temperature: f64,
    pub gyro_bias: [f64; 3],
    pub gyro_bias_temp_data: Vec<GyroBias>,
    pub gyro_g_sensitivity: [f64; 9],
    pub gyro_p_mag: [f64; 3],
    pub gyro_q_mag: [f64; 4],

    #[serde(flatten)]
    pub remaining: serde_json::Value,

    pub imu_noises: [f64; 4],
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
