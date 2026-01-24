# xr-tools

hardware for what appears like it could be a new compute platform is rapidly becoming available. unfortunately, mfgs
have non-standard closed interfaces not available to developers except on specific hardware on specific platforms.

this is a collection of low-level drivers for common devices. eventually, we hope for this to be a home for a common
application model around many xr devices.

let the building begin!

## device support

* [XREAL One and XREAL One Pro](./packages/xreal_one_driver)
  
  Software Updates, IMU (Gyro, Accelerometer, Magnetometer), Camera, Display Control (Brightness, Dimming, Mode), Key
  Presses and Factory Calibration Values (Intrinsics and Extrinsics for IMU, Camera and Display)

### upcoming

* Viture Luma Ultra
  
  Hope to have IMU and cameras working (including side cameras).

## enabling camera on macos

To enable Xreal One Pro camera on macOS, see [enable_camera_mac](./enable_camera_mac).

## special thanks
special thanks to the folks in the MakeReal XR discord for their research support. this was a team effort.
