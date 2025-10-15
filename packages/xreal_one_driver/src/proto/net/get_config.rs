use crate::proto::net::props::{GetPropertyRequest, PropertyResponse};
use crate::proto::net::NetworkTransaction;
use anyhow::bail;
use nalgebra::{Dyn, Matrix, VecStorage};
use serde::{Deserialize, Deserializer};

pub struct GetConfig;

impl NetworkTransaction<'static> for GetConfig {
    const MAGIC: [u8; 2] = [0x27, 0x1f];
    type RequestArgs = GetPropertyRequest;
    type Response = PropertyResponse<String>;
}

#[derive(Deserialize)]
pub struct Config {
    #[serde(rename = "FSN")]
    pub fsn: String,

    // todo: IMU
    // todo: RGB_camera
    // todo: SLAM_camera
    // todo: display
    pub display_distortion: DisplaysDistortion,
    pub glasses_version: usize,
    pub last_modified_time: time::PrimitiveDateTime,
}

#[derive(Deserialize)]
pub struct DisplaysDistortion {
    left_display: DisplayDistortion,
    right_display: DisplayDistortion,
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
