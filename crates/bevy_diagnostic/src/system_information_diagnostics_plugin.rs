use crate::DiagnosticId;
use bevy_app::{App, Plugin};

/// Adds a System Information Diagnostic, specifically `cpu_usage` (in %) and `mem_usage` (in %)
///
/// Supported targets:
/// * linux,
/// * windows,
/// * android,
/// * macos
///
/// NOT supported when using the `bevy/dynamic` feature even when using previously mentioned targets
#[derive(Default)]
pub struct SystemInformationDiagnosticsPlugin;
impl Plugin for SystemInformationDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(internal::setup_system)
            .add_system(internal::diagnostic_system);
    }
}

impl SystemInformationDiagnosticsPlugin {
    pub const CPU_USAGE: DiagnosticId =
        DiagnosticId::from_u128(78494871623549551581510633532637320956);
    pub const MEM_USAGE: DiagnosticId =
        DiagnosticId::from_u128(42846254859293759601295317811892519825);
}

// NOTE: sysinfo fails to compile when using bevy dynamic or on iOS and does nothing on wasm
#[cfg(all(
    any(
        target_os = "linux",
        target_os = "windows",
        target_os = "android",
        target_os = "macos"
    ),
    not(feature = "bevy_dynamic_plugin")
))]
pub mod internal {
    use bevy_ecs::{prelude::ResMut, system::Local};
    use bevy_log::info;
    use sysinfo::{CpuExt, System, SystemExt};

    use crate::{Diagnostic, Diagnostics};

    const BYTES_TO_GIB: f64 = 1.0 / 1024.0 / 1024.0 / 1024.0;

    pub(crate) fn setup_system(mut diagnostics: ResMut<Diagnostics>) {
        diagnostics.add(
            Diagnostic::new(
                super::SystemInformationDiagnosticsPlugin::CPU_USAGE,
                "cpu_usage",
                20,
            )
            .with_suffix("%"),
        );
        diagnostics.add(
            Diagnostic::new(
                super::SystemInformationDiagnosticsPlugin::MEM_USAGE,
                "mem_usage",
                20,
            )
            .with_suffix("%"),
        );
    }

    pub(crate) fn diagnostic_system(
        mut diagnostics: ResMut<Diagnostics>,
        mut sysinfo: Local<Option<System>>,
    ) {
        if sysinfo.is_none() {
            *sysinfo = Some(System::new_all());
        }
        let Some(sys) = sysinfo.as_mut() else {
            return;
        };

        sys.refresh_cpu();
        sys.refresh_memory();
        let current_cpu_usage = {
            let mut usage = 0.0;
            let cpus = sys.cpus();
            for cpu in cpus {
                usage += cpu.cpu_usage(); // NOTE: this returns a value from 0.0 to 100.0
            }
            // average
            usage / cpus.len() as f32
        };
        // `memory()` fns return a value in bytes
        let total_mem = sys.total_memory() as f64 / BYTES_TO_GIB;
        let used_mem = sys.used_memory() as f64 / BYTES_TO_GIB;
        let current_used_mem = used_mem / total_mem * 100.0;

        diagnostics.add_measurement(super::SystemInformationDiagnosticsPlugin::CPU_USAGE, || {
            current_cpu_usage as f64
        });
        diagnostics.add_measurement(super::SystemInformationDiagnosticsPlugin::MEM_USAGE, || {
            current_used_mem
        });
    }

    #[derive(Debug)]
    // This is required because the Debug trait doesn't detect it's used when it's only used in a print :(
    #[allow(dead_code)]
    struct SystemInfo {
        os: String,
        kernel: String,
        cpu: String,
        core_count: String,
        memory: String,
    }

    pub(crate) fn log_system_info() {
        let mut sys = sysinfo::System::new();
        sys.refresh_cpu();
        sys.refresh_memory();

        let info = SystemInfo {
            os: sys
                .long_os_version()
                .unwrap_or_else(|| String::from("not available")),
            kernel: sys
                .kernel_version()
                .unwrap_or_else(|| String::from("not available")),
            cpu: sys.global_cpu_info().brand().trim().to_string(),
            core_count: sys
                .physical_core_count()
                .map(|x| x.to_string())
                .unwrap_or_else(|| String::from("not available")),
            // Convert from Bytes to GibiBytes since it's probably what people expect most of the time
            memory: format!("{:.1} GiB", sys.total_memory() as f64 * BYTES_TO_GIB),
        };

        info!("{:?}", info);
    }
}

#[cfg(not(all(
    any(
        target_os = "linux",
        target_os = "windows",
        target_os = "android",
        target_os = "macos"
    ),
    not(feature = "bevy_dynamic_plugin")
)))]
pub mod internal {
    pub(crate) fn setup_system() {
        bevy_log::warn!("This platform and/or configuration is not supported!");
    }

    pub(crate) fn diagnostic_system() {
        // no-op
    }

    pub(crate) fn log_system_info() {
        // no-op
    }
}
