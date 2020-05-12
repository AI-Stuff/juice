use crate::ffi::*;
use crate::{API, Error};
use super::Context;
use super::PointerMode;
use lazy_static::lazy_static;
use log::debug;
use std::collections::HashSet;
use std::convert::AsRef;
use std::convert::TryFrom;
use std::ptr;
use std::ptr::NonNull;
use std::sync::{Mutex,Arc};

impl API {
    /// Create a new cuBLAS context, allocating resources on the host and the GPU.
    ///
    /// The returned Context must be provided to future cuBLAS calls.
    /// Creating contexts all the time can lead to performance problems.
    /// Generally one Context per GPU device and configuration is recommended.
    pub fn create() -> Result<Context, Error> {
        let handle = unsafe { API::ffi_create() }?;
        Ok(Context::from_c(handle))
    }

    /// Destroys the cuBLAS context, freeing its resources.
    ///
    /// Should generally not be called directly.
    /// Automatically called when dropping a Context.
    ///
    /// # Safety
    /// Instructs CUDA to remove the cuBLAS handle, causing any further instructions to fail.
    /// This should be called at the end of using cuBLAS and should ideally be handled by drop
    /// exclusively, and never called by the user.
    pub unsafe fn destroy(context: &mut Context) -> Result<(), Error> {
        API::ffi_destroy(*context.id_c())
    }

    /// Retrieve the pointer mode for a given cuBLAS context.
    pub fn get_pointer_mode(context: &Context) -> Result<PointerMode, Error> {
        Ok(PointerMode::from_c(
            unsafe { API::ffi_get_pointer_mode(*context.id_c()) }?,
        ))
    }

    /// Set the pointer mode for a given cuBLAS context.
    pub fn set_pointer_mode(context: &mut Context, pointer_mode: PointerMode) -> Result<(), Error> {
        Ok(unsafe {
            API::ffi_set_pointer_mode(*context.id_c(), pointer_mode.as_c())
        }?)
    }

    unsafe fn ffi_create() -> Result<cublasHandle_t, Error> {
        let mut handle: cublasHandle_t = ptr::null_mut();
        match cublasCreate_v2(&mut handle) {
            cublasStatus_t::CUBLAS_STATUS_SUCCESS => {
                Tracker::<cublasContext>::track(handle);
                Ok(handle)
            },
            cublasStatus_t::CUBLAS_STATUS_NOT_INITIALIZED => Err(Error::NotInitialized),
            cublasStatus_t::CUBLAS_STATUS_ARCH_MISMATCH => Err(Error::ArchMismatch),
            cublasStatus_t::CUBLAS_STATUS_ALLOC_FAILED => Err(Error::AllocFailed),
            _ => Err(Error::Unknown(
                "Unable to create the cuBLAS context/resources.",
            )),
        }
    }

    unsafe fn ffi_destroy(handle: cublasHandle_t) -> Result<(), Error> {
        Tracker::<cublasContext>::untrack(handle);
        match cublasDestroy_v2(handle) {
            cublasStatus_t::CUBLAS_STATUS_SUCCESS => Ok(()),
            cublasStatus_t::CUBLAS_STATUS_NOT_INITIALIZED => Err(Error::NotInitialized),
            _ => Err(Error::Unknown(
                "Unable to destroy the CUDA cuDNN context/resources.",
            )),
        }
    }

    unsafe fn ffi_get_pointer_mode(handle: cublasHandle_t) -> Result<cublasPointerMode_t, Error> {
        Tracker::<cublasContext>::exists(handle);
        let pointer_mode = &mut [cublasPointerMode_t::CUBLAS_POINTER_MODE_HOST];
        match cublasGetPointerMode_v2(handle, pointer_mode.as_mut_ptr()) {
            cublasStatus_t::CUBLAS_STATUS_SUCCESS => Ok(pointer_mode[0]),
            cublasStatus_t::CUBLAS_STATUS_NOT_INITIALIZED => Err(Error::NotInitialized),
            _ => Err(Error::Unknown("Unable to get cuBLAS pointer mode.")),
        }
    }

    unsafe fn ffi_set_pointer_mode(
        handle: cublasHandle_t,
        pointer_mode: cublasPointerMode_t,
    ) -> Result<(), Error> {
        Tracker::<cublasContext>::exists(handle);
        match cublasSetPointerMode_v2(handle, pointer_mode) {
            cublasStatus_t::CUBLAS_STATUS_SUCCESS => Ok(()),
            cublasStatus_t::CUBLAS_STATUS_NOT_INITIALIZED => Err(Error::NotInitialized),
            _ => Err(Error::Unknown("Unable to get cuBLAS pointer mode.")),
        }
    }

    // TODO: cublasGetVersion_v2
    // TODO: cublasSetStream_v2
    // TODO: cublasGetStream_v2
    // TODO: cublasGetAtomicsMode
    // TODO: cublasSetAtomicsMode
    // TODO: cublasSetVector
    // TODO: cublasGetVector
    // TODO: cublasSetMatrix
    // TODO: cublasGetMatrix
    // TODO: cublasSetVectorAsync
    // TODO: cublasGetVectorAsync
    // TODO: cublasSetMatrixAsync
    // TODO: cublasGetMatrixAsync
}

#[cfg(test)]
mod test {
    use crate::ffi::cublasPointerMode_t;
    use crate::API;
    use crate::Context;

    #[test]
    #[serial_test::serial]
    fn manual_context_creation() {
        crate::chore::test_setup();

        unsafe {
            let handle = API::ffi_create().unwrap();
            API::ffi_destroy(handle).unwrap();
        }
    }

    #[test]
    #[serial_test::serial]
    fn default_pointer_mode_is_host() {
        crate::chore::test_setup();

        unsafe {
            let context = Context::new().unwrap();
            let mode = API::ffi_get_pointer_mode(*context.id_c()).unwrap();
            assert_eq!(cublasPointerMode_t::CUBLAS_POINTER_MODE_HOST, mode);
        }
        crate::chore::test_teardown();
    }

    #[test]
    #[serial_test::serial]
    fn can_set_pointer_mode() {
        crate::chore::test_setup();

        unsafe {
            let context = Context::new().unwrap();
            API::ffi_set_pointer_mode(
                *context.id_c(),
                cublasPointerMode_t::CUBLAS_POINTER_MODE_DEVICE,
            ).unwrap();
            let mode = API::ffi_get_pointer_mode(*context.id_c()).unwrap();
            assert_eq!(cublasPointerMode_t::CUBLAS_POINTER_MODE_DEVICE, mode);
            API::ffi_set_pointer_mode(
                *context.id_c(),
                cublasPointerMode_t::CUBLAS_POINTER_MODE_HOST,
            ).unwrap();
            let mode2 = API::ffi_get_pointer_mode(*context.id_c()).unwrap();
            assert_eq!(cublasPointerMode_t::CUBLAS_POINTER_MODE_HOST, mode2);
        }
        crate::chore::test_teardown();
    }
}