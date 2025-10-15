use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State};
use std::path::PathBuf;
use tokio::time::{timeout, Duration};

// ==================== ESTRUCTURAS DE DATOS ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ClusteringParams {
    input_file: String,
    output_dir: String,
    min_atoms: i32,
    max_iterations: i32,
    n_clusters_per_level: Option<i32>,
    silhouette_threshold: f64,
    davies_bouldin_threshold: f64,
    dispersion_threshold: Option<f64>,
    quantile: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClusterInfo {
    name: String,
    atoms: i32,
    level: i32,
    reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClusteringResult {
    success: bool,
    message: String,
    clusters_found: i32,
    total_atoms: i32,
    clusters_info: Vec<ClusterInfo>,
}

#[derive(Clone, Serialize)]
struct ProgressPayload {
    message: String,
    progress: Option<f32>,
}

#[derive(Clone, Serialize)]
struct DependencyStatus {
    python_version: String,
    dependencies_ok: bool,
    missing_dependencies: Vec<String>,
}

// Estado global de la aplicación
#[derive(Clone)]
struct AppState {
    is_processing: Arc<Mutex<bool>>,
    should_cancel: Arc<Mutex<bool>>,
}

// ==================== UTILIDADES ====================

/// Detecta el comando de Python según el sistema operativo
fn get_python_command() -> String {
    if cfg!(target_os = "windows") {
        "python".to_string()
    } else {
        "python3".to_string()
    }
}

/// Resuelve el path del script Python desde los recursos empaquetados
fn resolve_python_script(
    app_handle: &tauri::AppHandle,
    script_name: &str,
) -> Result<PathBuf, String> {
    app_handle
        .path()
        .resolve(script_name, tauri::path::BaseDirectory::Resource)
        .map_err(|e| format!("No se pudo encontrar el script: {} - Error: {}", script_name, e))
}

// ==================== VERIFICACIÓN DE DEPENDENCIAS ====================

#[tauri::command]
async fn check_python_dependencies() -> Result<DependencyStatus, String> {
    let python_cmd = get_python_command();

    // Verificar que Python existe
    let version_output = Command::new(&python_cmd)
        .arg("--version")
        .output()
        .map_err(|_| {
            format!(
                "Python no está instalado o no está en PATH.\n\
                 Por favor instala Python 3.8+ y asegúrate de que esté en PATH.\n\
                 Comando buscado: {}",
                python_cmd
            )
        })?;

    let python_version = String::from_utf8_lossy(&version_output.stdout)
        .trim()
        .to_string();

    // Script para verificar dependencias
    let check_script = r#"
import sys
missing = []

try:
    import numpy
except ImportError:
    missing.append("numpy")

try:
    import sklearn
except ImportError:
    missing.append("scikit-learn")

try:
    from ovito.io import import_file
except ImportError:
    missing.append("ovito")

try:
    from scipy.spatial import ConvexHull
except ImportError:
    missing.append("scipy")

if missing:
    print("MISSING:" + ",".join(missing))
    sys.exit(1)
else:
    print("OK")
    sys.exit(0)
"#;

    let deps_output = Command::new(&python_cmd)
        .arg("-c")
        .arg(check_script)
        .output()
        .map_err(|e| format!("Error verificando dependencias: {}", e))?;

    let output_str = String::from_utf8_lossy(&deps_output.stdout);

    if deps_output.status.success() {
        Ok(DependencyStatus {
            python_version,
            dependencies_ok: true,
            missing_dependencies: vec![],
        })
    } else {
        let missing: Vec<String> = if output_str.starts_with("MISSING:") {
            output_str
                .trim_start_matches("MISSING:")
                .split(',')
                .map(|s| s.trim().to_string())
                .collect()
        } else {
            vec!["Error desconocido al verificar dependencias".to_string()]
        };

        Ok(DependencyStatus {
            python_version,
            dependencies_ok: false,
            missing_dependencies: missing,
        })
    }
}

// ==================== CLUSTERING CON PROGRESO EN TIEMPO REAL ====================

#[tauri::command]
async fn run_clustering(
    params: ClusteringParams,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
    window: tauri::Window,
) -> Result<ClusteringResult, String> {
    // Verificar si ya hay un proceso ejecutándose
    {
        let mut is_processing = state.is_processing.lock().unwrap();
        if *is_processing {
            return Err("Ya hay un proceso de clustering en ejecución".to_string());
        }
        *is_processing = true;
    }

    // Resetear flag de cancelación
    {
        let mut should_cancel = state.should_cancel.lock().unwrap();
        *should_cancel = false;
    }

    // Validar que el archivo existe
    if !std::path::Path::new(&params.input_file).exists() {
        let mut is_processing = state.is_processing.lock().unwrap();
        *is_processing = false;
        return Err(format!("El archivo no existe: {}", params.input_file));
    }

    // Emitir evento de inicio
    window
        .emit(
            "clustering-progress",
            ProgressPayload {
                message: "🚀 Iniciando clustering...".to_string(),
                progress: Some(0.0),
            },
        )
        .ok();

    // Ejecutar con timeout de 30 minutos
    let state_clone = state.inner().clone();
    let result = timeout(
        Duration::from_secs(1800), // 30 minutos
        execute_python_clustering(params, app_handle, window, state_clone),
    )
    .await;

    // Liberar estado de procesamiento
    {
        let mut is_processing = state.is_processing.lock().unwrap();
        *is_processing = false;
    }

    match result {
        Ok(Ok(clustering_result)) => Ok(clustering_result),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("⏱️ Timeout: El clustering tardó más de 30 minutos".to_string()),
    }
}

async fn execute_python_clustering(
    params: ClusteringParams,
    app_handle: tauri::AppHandle,
    window: tauri::Window,
    state: AppState,
) -> Result<ClusteringResult, String> {
    // Resolver path del script
    let script_path = resolve_python_script(&app_handle, "clustering_interface.py")?;

    // Serializar parámetros
    let params_json = serde_json::to_string(&params)
        .map_err(|e| format!("Error serializando parámetros: {}", e))?;

    let python_cmd = get_python_command();

    // Lanzar proceso Python
    let mut child = Command::new(&python_cmd)
        .arg(&script_path)
        .arg(&params_json)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            format!(
                "Error ejecutando Python:\n{}\n\nComando: {} {:?}",
                e, python_cmd, script_path
            )
        })?;

    let stdout = child
        .stdout
        .take()
        .ok_or("No se pudo capturar stdout")?;

    let stderr = child
        .stderr
        .take()
        .ok_or("No se pudo capturar stderr")?;

    // Leer stdout en tiempo real
    let window_clone = window.clone();
    let state_clone = Arc::new(state);
    let state_for_reader = state_clone.clone();

    let stdout_handle = tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut json_output = String::new();
        let mut found_json = false;

        for line in reader.lines() {
            // Verificar si se solicitó cancelación
            {
                let should_cancel = state_for_reader.should_cancel.lock().unwrap();
                if *should_cancel {
                    return Err("❌ Clustering cancelado por el usuario".to_string());
                }
            }

            match line {
                Ok(line) => {
                    // Si la línea es JSON (empieza con {), capturarlo
                    if line.trim().starts_with('{') {
                        json_output = line.clone();
                        found_json = true;
                    } else {
                        // Emitir línea de progreso al frontend
                        window_clone
                            .emit(
                                "clustering-progress",
                                ProgressPayload {
                                    message: line,
                                    progress: None,
                                },
                            )
                            .ok();
                    }
                }
                Err(e) => {
                    return Err(format!("Error leyendo stdout: {}", e));
                }
            }
        }

        if found_json {
            Ok(json_output)
        } else {
            Err("No se recibió resultado JSON del script Python".to_string())
        }
    });

    // Leer stderr (errores)
    let stderr_handle = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut errors = Vec::new();

        for line in reader.lines() {
            if let Ok(line) = line {
                errors.push(line);
            }
        }

        errors.join("\n")
    });

    // Esperar a que termine el proceso
    let status = child
        .wait()
        .map_err(|e| format!("Error esperando proceso Python: {}", e))?;

    // Obtener resultados de lectura
    let json_result = stdout_handle
        .await
        .map_err(|e| format!("Error en task de stdout: {}", e))??;

    let stderr_output = stderr_handle
        .await
        .map_err(|e| format!("Error en task de stderr: {}", e))?;

    if !status.success() {
        return Err(format!(
            "❌ El script Python falló con código {}:\n\n{}",
            status.code().unwrap_or(-1),
            stderr_output
        ));
    }

    // Parsear resultado JSON
    let result: ClusteringResult = serde_json::from_str(&json_result).map_err(|e| {
        format!(
            "Error parseando resultado JSON:\n{}\n\nJSON recibido:\n{}",
            e, json_result
        )
    })?;

    // Emitir evento de finalización
    window
        .emit(
            "clustering-progress",
            ProgressPayload {
                message: "✅ Clustering completado".to_string(),
                progress: Some(100.0),
            },
        )
        .ok();

    Ok(result)
}

// ==================== CANCELACIÓN ====================

#[tauri::command]
async fn cancel_clustering(state: State<'_, AppState>) -> Result<(), String> {
    let mut should_cancel = state.should_cancel.lock().unwrap();
    *should_cancel = true;
    Ok(())
}

#[tauri::command]
fn get_processing_status(state: State<'_, AppState>) -> bool {
    let is_processing = state.is_processing.lock().unwrap();
    *is_processing
}

// ==================== PERSISTENCIA DE CONFIGURACIÓN ====================

#[tauri::command]
fn save_config(params: ClusteringParams, app_handle: tauri::AppHandle) -> Result<(), String> {
    let config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("No se pudo obtener directorio de configuración: {}", e))?;

    // Crear directorio si no existe
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Error creando directorio de config: {}", e))?;

    let config_path = config_dir.join("last_config.json");

    let json = serde_json::to_string_pretty(&params)
        .map_err(|e| format!("Error serializando configuración: {}", e))?;

    std::fs::write(config_path, json)
        .map_err(|e| format!("Error guardando configuración: {}", e))?;

    Ok(())
}

#[tauri::command]
fn load_config(app_handle: tauri::AppHandle) -> Result<ClusteringParams, String> {
    let config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("No se pudo obtener directorio de configuración: {}", e))?;

    let config_path = config_dir.join("last_config.json");

    if !config_path.exists() {
        return Err("No hay configuración guardada".to_string());
    }

    let json = std::fs::read_to_string(config_path)
        .map_err(|e| format!("Error leyendo configuración: {}", e))?;

    serde_json::from_str(&json)
        .map_err(|e| format!("Error parseando configuración: {}", e))
}

// ==================== UTILIDADES ADICIONALES ====================

#[tauri::command]
fn open_output_folder(output_dir: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&output_dir)
            .spawn()
            .map_err(|e| format!("Error abriendo carpeta: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&output_dir)
            .spawn()
            .map_err(|e| format!("Error abriendo carpeta: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&output_dir)
            .spawn()
            .map_err(|e| format!("Error abriendo carpeta: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
fn get_system_info() -> Result<String, String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let python_cmd = get_python_command();

    Ok(format!(
        "Sistema Operativo: {}\nArquitectura: {}\nComando Python: {}",
        os, arch, python_cmd
    ))
}
// ==================== LECTURA DE ARCHIVO DUMP ====================

#[tauri::command]
fn leer_dump() -> Result<String, String> {
    use std::fs;

    let path = std::path::Path::new("test/inputs/dump.track_clustering");

    if !path.exists() {
        return Err(format!(
            "El archivo no existe en la ruta relativa: {:?}",
            path
        ));
    }

    match fs::read_to_string(path) {
        Ok(data) => Ok(data),
        Err(e) => Err(format!("Error al leer el archivo: {}", e)),
    }
}

// ==================== MAIN ====================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            is_processing: Arc::new(Mutex::new(false)),
            should_cancel: Arc::new(Mutex::new(false)),
        })
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            run_clustering,
            cancel_clustering,
            get_processing_status,
            check_python_dependencies,
            save_config,
            load_config,
            open_output_folder,
            get_system_info,
            leer_dump
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}