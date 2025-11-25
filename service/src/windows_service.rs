use std::ffi::OsString;
use std::time::Duration;
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

const SERVICE_NAME: &str = "FrameworkControlService";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

pub fn run_service() -> windows_service::Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

define_windows_service!(ffi_service_main, service_main);

fn service_main(_arguments: Vec<OsString>) {
    if let Err(e) = run_service_main() {
        eprintln!("Service error: {}", e);
    }
}

fn run_service_main() -> windows_service::Result<()> {
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            ServiceControl::Stop => {
                // Signal the service to stop
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    // Tell Windows we're starting
    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    // Run the actual service logic
    run_fan_curve_service();

    // Tell Windows we're stopping
    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    Ok(())
}

fn run_fan_curve_service() {
    use tokio::runtime::Runtime;

    let runtime = Runtime::new().expect("Failed to create runtime");

    runtime.block_on(async {
        // Initialize state
        let state = crate::AppState::initialize().await;

        // Load fan curve from config
        let config = state.config.read().await;
        let fan_curve = vec![
            (40.0, 20.0),
            (50.0, 30.0),
            (60.0, 40.0),
            (70.0, 60.0),
            (80.0, 80.0),
            (90.0, 100.0),
        ];
        drop(config);

        tracing::info!("Framework Control Service started - fan curve active");

        // Main service loop
        loop {
            if let Some(ft) = state.framework_tool.read().await.as_ref() {
                if let Ok(thermal) = ft.read_thermal().await {
                    let max_temp = thermal
                        .sensors
                        .iter()
                        .map(|s| s.temp_c)
                        .fold(f32::NEG_INFINITY, f32::max);

                    // Interpolate fan speed from curve
                    let mut duty = 50.0;
                    for i in 0..fan_curve.len() {
                        if i == 0 && max_temp <= fan_curve[i].0 {
                            duty = fan_curve[i].1;
                            break;
                        }
                        if i == fan_curve.len() - 1 && max_temp >= fan_curve[i].0 {
                            duty = fan_curve[i].1;
                            break;
                        }
                        if i < fan_curve.len() - 1
                            && max_temp >= fan_curve[i].0
                            && max_temp <= fan_curve[i + 1].0
                        {
                            let t1 = fan_curve[i].0;
                            let t2 = fan_curve[i + 1].0;
                            let d1 = fan_curve[i].1;
                            let d2 = fan_curve[i + 1].1;
                            let ratio = (max_temp - t1) / (t2 - t1);
                            duty = d1 + (d2 - d1) * ratio;
                            break;
                        }
                    }

                    // Apply fan speed
                    let _ = ft.set_fan_duty(duty as u32, None).await;
                    tracing::debug!("Fan curve: {}°C -> {}%", max_temp, duty as u32);
                }
            }

            // Wait 5 seconds before next update
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });
}
