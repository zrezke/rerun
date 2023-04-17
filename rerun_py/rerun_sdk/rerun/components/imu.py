from rerun.components import REGISTERED_COMPONENT_NAMES, ComponentTypeFactory
from rerun.components.point import Point3DArray
from rerun.components.quaternion import QuaternionArray
import pyarrow as pa
import numpy.typing as npt
import numpy as np

__all__ = ["ImuType", "Imu"]


class Imu(pa.ExtensionArray):  # type: ignore[misc]
    def create(
        accel: npt.NDArray[np.float32], gyro: npt.NDArray[np.float32], orientation: npt.NDArray[np.float32]
    ) -> "Imu":
        """Build Imu data from acceleration and gyroscope data"""
        assert accel.shape[0] == 3
        assert gyro.shape[0] == 3
        accel_point = Point3DArray.from_numpy(accel.reshape(1, 3))
        gyro_point = Point3DArray.from_numpy(gyro.reshape(1, 3))
        quat = QuaternionArray.from_numpy(np.array(orientation, dtype=np.float32).reshape(1, 4))
        return pa.StructArray.from_arrays(  # type: ignore[no-any-return]
            fields=ImuType.storage_type,
            arrays=[accel_point, gyro_point, quat],
        )


ImuType = ComponentTypeFactory("ImuType", Imu, REGISTERED_COMPONENT_NAMES["rerun.imu"])
pa.register_extension_type(ImuType())
