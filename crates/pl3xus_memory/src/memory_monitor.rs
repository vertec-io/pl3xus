use bevy::prelude::*;
use std::time::{Duration, Instant};

/// Resource to track memory usage over time
#[derive(Resource)]
pub struct MemoryMonitor {
    pub last_check: Instant,
    pub check_interval: Duration,
    pub memory_samples: Vec<(f64, usize)>, // (time_elapsed, memory_usage_bytes)
    pub start_time: Instant,
}

impl Default for MemoryMonitor {
    fn default() -> Self {
        Self {
            last_check: Instant::now(),
            check_interval: Duration::from_secs(5),
            memory_samples: Vec::new(),
            start_time: Instant::now(),
        }
    }
}

/// System to initialize the memory monitor
pub fn setup_memory_monitor(mut commands: Commands) {
    commands.insert_resource(MemoryMonitor::default());
}

/// System to monitor memory usage
pub fn monitor_system_memory(
    _time: Res<Time>,
    mut monitor: ResMut<MemoryMonitor>,
    network: Option<Res<pl3xus::Network<pl3xus_websockets::WebSocketProvider>>>,
) {
    if monitor.last_check.elapsed() < monitor.check_interval {
        return;
    }

    monitor.last_check = Instant::now();
    let elapsed = monitor.start_time.elapsed().as_secs_f64();

    // Get current memory usage
    let memory_usage = get_current_memory_usage();
    monitor.memory_samples.push((elapsed, memory_usage));

    // Print memory stats with more details
    println!(
        "Time elapsed: {:.2}s, Memory usage: {} bytes",
        elapsed, memory_usage
    );

    // Print network details if available
    if let Some(net) = network {
        // println!("  - Connection tasks: {}", net.connection_tasks.len());
        println!("  - Established connections: {}", net.has_connections());
        // println!("  - Message map entries: {}", net.recv_message_map.len());
    }

    // Check for memory leaks
    if monitor.memory_samples.len() >= 10 {
        let trend = analyze_memory_trend(&monitor.memory_samples);
        if trend > 0.05 {
            println!(
                "WARNING: Memory usage is steadily increasing. Possible memory leak detected!"
            );
            println!("Memory growth rate: {:.2}% per sample", trend * 100.0);
        }
    }
}

/// Get the current memory usage in bytes
fn get_current_memory_usage() -> usize {
    #[cfg(target_os = "windows")]
    {
        use std::process;
        let pid = process::id();
        get_windows_memory_info(pid).unwrap_or_default()
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Fallback for non-Windows platforms
        0
    }
}

#[cfg(target_os = "windows")]
fn get_windows_memory_info(pid: u32) -> Result<usize, String> {
    use std::process::Command;

    let output = Command::new("powershell")
        .args([
            "-Command",
            &format!(
                "Get-Process -Id {} | Select-Object -ExpandProperty WorkingSet",
                pid
            ),
        ])
        .output()
        .map_err(|e| format!("Failed to execute powershell command: {}", e))?;

    if !output.status.success() {
        return Err(format!("Command failed with status: {}", output.status));
    }

    let output_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    output_str
        .parse::<usize>()
        .map_err(|e| format!("Failed to parse memory usage: {}", e))
}

/// Analyze memory trend to detect leaks
/// Returns a value between 0 and 1 indicating the growth rate
fn analyze_memory_trend(samples: &[(f64, usize)]) -> f64 {
    if samples.len() < 5 {
        return 0.0;
    }

    // Calculate the average growth rate
    let mut growth_rates = Vec::new();
    for i in 1..samples.len() {
        let prev = samples[i - 1].1 as f64;
        let curr = samples[i].1 as f64;

        if prev > 0.0 {
            let rate = (curr - prev) / prev;
            growth_rates.push(rate);
        }
    }

    // Return the average growth rate
    if growth_rates.is_empty() {
        0.0
    } else {
        growth_rates.iter().sum::<f64>() / growth_rates.len() as f64
    }
}

/// Register the memory monitor plugin
pub fn register_memory_monitor_plugin(app: &mut App) {
    app.add_systems(Startup, setup_memory_monitor)
        .add_systems(Update, monitor_system_memory);
}
