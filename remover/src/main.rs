use std::process::Command;
use std::thread;
use std::time::Duration;

const SERVICE_NAME: &str = "ActivityMonitor";
const TASK_NAME: &str = "ActivityMonitorUserAgent";

fn main() {
    println!("============================================================");
    println!("  ActivityMonitor Enterprise Agent - Removedor v4.0.0");
    println!("============================================================");
    println!();

    if !is_admin() {
        println!("[*] Solicitando privilegios de administrador...");
        elevate_and_rerun();
        return;
    }

    println!("[1/6] Eliminando servicio (evita reinicio automatico)...");
    delete_service();

    println!("[2/6] Eliminando tarea programada...");
    delete_scheduled_task();

    println!("[3/6] Deteniendo servicio...");
    stop_service();

    println!("[4/6] Deteniendo procesos restantes...");
    kill_processes();

    println!("[5/6] Eliminando archivos...");
    remove_files();

    println!("[6/6] Limpiando registro...");
    clean_registry();

    println!();
    println!("[+] Agente eliminado correctamente.");
    println!();

    println!("[*] Auto-eliminando...");
    self_delete();
}

fn is_admin() -> bool {
    Command::new("net")
        .args(&["session"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn elevate_and_rerun() {
    let exe = std::env::current_exe().expect("No se pudo obtener la ruta del ejecutable");
    let _ = Command::new("powershell")
        .args(&[
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-Command",
            &format!("Start-Process '{}' -Verb RunAs -Wait", exe.display()),
        ])
        .status();
}

fn stop_service() {
    let _ = Command::new("net").args(&["stop", SERVICE_NAME]).output();
    thread::sleep(Duration::from_secs(3));

    let output = Command::new("sc")
        .args(&["query", SERVICE_NAME])
        .output();
    if let Ok(o) = output {
        let stdout = String::from_utf8_lossy(&o.stdout);
        if !stdout.contains("STOPPED") {
            println!("  [*] Forzando detencion...");
            kill_other_agent_processes();
            thread::sleep(Duration::from_secs(2));
        }
    }
    println!("  [+] Servicio detenido");
}

fn delete_service() {
    let output = Command::new("sc")
        .args(&["delete", SERVICE_NAME])
        .output();
    match output {
        Ok(o) if o.status.success() => println!("  [+] Servicio eliminado"),
        _ => println!("  [!] Servicio no encontrado o ya eliminado"),
    }
}

/// Kills every `activity-monitor-agent.exe` process EXCEPT the current remover.
/// We cannot use `taskkill /IM` because it would kill this remover itself.
fn kill_other_agent_processes() {
    let current_pid = std::process::id();

    let output = Command::new("tasklist")
        .args(&["/FI", "IMAGENAME eq activity-monitor-agent.exe", "/FO", "CSV", "/NH"])
        .output();

    if let Ok(o) = output {
        let stdout = String::from_utf8_lossy(&o.stdout);
        for line in stdout.lines() {
            let fields: Vec<&str> = line.split(',').collect();
            if fields.len() < 2 {
                continue;
            }
            let pid_str = fields[1].trim().trim_matches('"');
            if let Ok(pid) = pid_str.parse::<u32>() {
                if pid != current_pid {
                    let _ = Command::new("taskkill")
                        .args(&["/F", "/PID", &pid.to_string()])
                        .output();
                }
            }
        }
    }
    thread::sleep(Duration::from_secs(1));
}

fn kill_processes() {
    kill_other_agent_processes();
    println!("  [+] Procesos detenidos");
}

fn delete_scheduled_task() {
    let output = Command::new("schtasks")
        .args(&["/Delete", "/TN", TASK_NAME, "/F"])
        .output();
    match output {
        Ok(o) if o.status.success() => println!("  [+] Tarea programada eliminada"),
        _ => println!("  [!] Tarea programada no encontrada o ya eliminada"),
    }
}

fn remove_files() {
    let paths = [
        r"C:\ProgramData\ActivityMonitor",
        r"C:\ProgramData\ActivityMonitor\Bin",
        r"C:\ProgramData\ActivityMonitor\Config",
        r"C:\ProgramData\ActivityMonitor\Data",
        r"C:\ProgramData\ActivityMonitor\Logs",
    ];

    for path in &paths {
        let _ = Command::new("rmdir")
            .args(&["/s", "/q", path])
            .output();
    }
    println!("  [+] Archivos eliminados");
}

fn clean_registry() {
    let keys = [
        r"HKLM\SOFTWARE\ActivityMonitor\Agent",
        r"HKLM\SOFTWARE\ActivityMonitor",
    ];
    for key in &keys {
        let _ = Command::new("reg")
            .args(&["delete", key, "/f"])
            .output();
    }
    println!("  [+] Registro limpiado");
}

fn self_delete() {
    let exe = std::env::current_exe().expect("No se pudo obtener la ruta del ejecutable");
    let bat_path = std::env::temp_dir().join("am_self_remove.bat");

    // After the remover exits, delete the exe, its parent folders and the registry.
    let script = format!(
        "@echo off\r\n\
         timeout /t 3 /nobreak >nul\r\n\
         del /F /Q \"{exe}\" >nul 2>&1\r\n\
         rmdir /s /q \"C:\\ProgramData\\ActivityMonitor\" >nul 2>&1\r\n\
         reg delete \"HKLM\\SOFTWARE\\ActivityMonitor\" /f >nul 2>&1\r\n\
         del /F /Q \"{bat}\" >nul 2>&1\r\n\
         cmd /c del /f /q \"%~f0\" >nul 2>&1\r\n",
        exe = exe.display(),
        bat = bat_path.display(),
    );

    std::fs::write(&bat_path, script).expect("No se pudo crear script de auto-eliminacion");

    let _ = Command::new("cmd.exe")
        .args(&["/c", &bat_path.to_string_lossy()])
        .spawn();
}
