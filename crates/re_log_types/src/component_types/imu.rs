use crate::Component;
use arrow2_convert::{ArrowDeserialize, ArrowField, ArrowSerialize};

use super::{Point3D, Quaternion};

#[derive(Clone, Debug, PartialEq, ArrowField, ArrowSerialize, ArrowDeserialize)]
pub struct ImuData {
    pub accel: Point3D,
    pub gyro: Point3D,
    pub orientation: Quaternion,
}

impl Component for ImuData {
    #[inline]
    fn name() -> crate::ComponentName {
        "rerun.imu".into()
    }
}
