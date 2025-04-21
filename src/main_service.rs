use std::{
    ffi::OsString,
    sync::mpsc,
    time::Duration,
    fs::{File, OpenOptions},
    io::Write,
};
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};


