// Emparejar AirPods nuevos sin entrar a Configuración.
// Busca dispositivos en modo de emparejamiento, empareja, marca como confiable y conecta.
use bluer::agent::Agent;
use bluer::{AdapterEvent, Session};
use futures::StreamExt;
use log::{info, warn};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(60);

pub fn notify(title: &str, body: &str) {
    let _ = std::process::Command::new("notify-send")
        .args(["-a", "LibrePods", "-i", "audio-headphones", "-t", "6000", title, body])
        .spawn();
}

fn parece_airpods(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("airpods") || n.contains("beats")
}

/// Corre en segundo plano, informando con notificaciones.
pub fn start_pairing() {
    std::thread::spawn(|| {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                warn!("No se pudo crear runtime para emparejar: {}", e);
                return;
            }
        };
        rt.block_on(async {
            notify(
                "Emparejar AirPods",
                "Abre el estuche junto a la laptop y mantén presionado el botón del estuche hasta que la luz parpadee en blanco.",
            );
            match pair_new().await {
                Ok(name) => notify("✓ Listo", &format!("{} emparejados y conectados.", name)),
                Err(e) => notify("No se pudo emparejar", &e),
            }
        });
    });
}

async fn pair_new() -> Result<String, String> {
    let session = Session::new().await.map_err(|e| e.to_string())?;
    let adapter = session.default_adapter().await.map_err(|e| e.to_string())?;
    adapter.set_powered(true).await.map_err(|e| e.to_string())?;
    let _ = adapter.set_pairable(true).await;

    // Agente que acepta el emparejamiento "Just Works" de los AirPods sin preguntar.
    let _agent = session
        .register_agent(Agent {
            request_default: false,
            request_confirmation: Some(Box::new(|_req| Box::pin(async { Ok(()) }))),
            request_authorization: Some(Box::new(|_req| Box::pin(async { Ok(()) }))),
            ..Default::default()
        })
        .await
        .map_err(|e| e.to_string())?;

    let events = adapter
        .discover_devices()
        .await
        .map_err(|e| format!("No se pudo buscar dispositivos: {}", e))?;
    tokio::pin!(events);
    let deadline = tokio::time::sleep(TIMEOUT);
    tokio::pin!(deadline);

    let device = loop {
        tokio::select! {
            _ = &mut deadline => {
                return Err("No encontré AirPods en modo de emparejamiento (60 s). Inténtalo otra vez manteniendo el botón del estuche.".into());
            }
            ev = events.next() => {
                let Some(AdapterEvent::DeviceAdded(addr)) = ev else { continue };
                let Ok(dev) = adapter.device(addr) else { continue };
                if dev.is_paired().await.unwrap_or(false) {
                    continue;
                }
                let name = dev.name().await.ok().flatten().unwrap_or_default();
                if parece_airpods(&name) {
                    info!("Encontrados en modo emparejamiento: {} ({})", name, addr);
                    break dev;
                }
            }
        }
    };
    drop(events); // detiene la búsqueda

    let name = device
        .name()
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "AirPods".into());
    notify("Emparejando…", &format!("Conectando con {}", name));

    // Hasta 3 intentos: a veces el primero falla por estar aún buscando.
    let mut last_err = String::new();
    for _ in 0..3 {
        match device.pair().await {
            Ok(()) => {
                last_err.clear();
                break;
            }
            Err(e) => {
                last_err = e.to_string();
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
    if !last_err.is_empty() && !device.is_paired().await.unwrap_or(false) {
        return Err(format!("Falló el emparejamiento: {}", last_err));
    }
    let _ = device.set_trusted(true).await;
    if let Err(e) = device.connect().await {
        warn!("Emparejados pero no conectaron: {}", e);
        let _ = tokio::process::Command::new("bluetoothctl")
            .arg("connect")
            .arg(device.address().to_string())
            .output()
            .await;
    }
    Ok(name)
}
