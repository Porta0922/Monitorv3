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

    println!("[1/6] Deteniendo servicio...");
    stop_service();

    println!("[2/6] Eliminando servicio...");
    delete_service();

    println!("[3/6] Deteniendo procesos...");
    kill_processes();

    println!("[4/6] Eliminando tarea programada...");
    delete_scheduled_task();

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
            let _ = Command::new("taskkill")
                .args(&["/F", "/IM", "activity-monitor-agent.exe"])
                .output();
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

fn kill_processes() {
    let _ = Command::new("taskkill")
        .args(&["/F", "/IM", "activity-monitor-agent.exe"])
        .output();
    thread::sleep(Duration::from_secs(1));
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

    let script = format!(
        "@echo off\r\n\
         timeout /t 2 /nobreak >nul\r\n\
         del /F /Q \"{}\" >nul 2>&1\r\n\
         del /F /Q \"{}\" >nul 2>&1\r\n\
         cmd /c del /f /q \"%~f0\" >nul 2>&1\r\n",
        exe.display(),
        bat_path.display(),
    );

    std::fs::write(&bat_path, script).expect("No se pudo crear script de auto-eliminacion");

    let _ = Command::new("cmd.exe")
        .args(&["/c", &bat_path.to_string_lossy()])
        .spawn();
}
