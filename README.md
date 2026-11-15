# LibrePods en español (GNOME)

Versión modificada de [LibrePods](https://github.com/librepods-org/librepods) para usar AirPods en Linux con GNOME. Se basa en la reescritura en Rust (rama `linux/rust`, commit `672e65a`) e integra un menú sencillo en el panel superior, una tarjeta animada de batería y una opción para emparejar AirPods nuevos.

Probado con AirPods 4 con cancelación de ruido (A3056) en Ubuntu con GNOME 50.

## Qué agrega esta versión

- **Menú en el panel (arriba a la derecha), en español:** nombre y estado, batería de cada AirPod y del estuche, Conectar/Desconectar, modo de ruido (Cancelación, Transparencia, Adaptativo), "Bajar volumen al hablar", Ver batería, Emparejar AirPods nuevos, Más ajustes y Salir.
- **Tarjeta animada de batería:** aparece al conectar los AirPods o al hacer clic en el ícono. Sale justo debajo del panel, donde se abre el menú de WiFi/Bluetooth. Los anillos se llenan con animación y se cierra sola a los ~6 s.
- **Emparejar fácil:** busca AirPods en modo de emparejamiento, los empareja, los marca como confiables y los conecta, avisando con notificaciones.
- **Corrección:** en la primera conexión se perdían la información del dispositivo y las llaves LE (IRK y llave de cifrado), necesarias para reconocer los AirPods al abrir el estuche.
- **Estado del menú:** ahora se marca como desconectado cuando los AirPods se desconectan.

Lo que ya traía LibrePods se conserva: conexión automática al abrir el estuche, pausa al quitarse un AirPod, ventana completa de ajustes, etc.

## Archivos modificados

| Archivo | Cambio |
|---|---|
| `src/ui/popup.rs` | nuevo: tarjeta animada (iced canvas) |
| `src/bluetooth/pairing.rs` | nuevo: emparejar AirPods nuevos |
| `src/ui/tray.rs` | menú reescrito en español |
| `src/ui/window.rs` | abre y anima la tarjeta, posición bajo el panel |
| `src/bluetooth/aacp.rs` | corrección del guardado de información y llaves |
| `src/main.rs`, `src/devices/airpods.rs`, `src/ui/messages.rs` | MAC en el menú, estado al desconectar, XWayland |
| `Cargo.toml` | activa la función `canvas` de iced |

## Requisitos

```bash
sudo apt install -y libdbus-1-dev libpulse-dev pkg-config libxkbcommon-dev libwayland-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # Rust
```

En GNOME se necesita la extensión de AppIndicator (en Ubuntu ya viene: `ubuntu-appindicators`).

## Compilar e instalar

```bash
cd linux-rust
cargo build --release
cd ..
./instalar.sh
```

`instalar.sh`:
1. copia el programa a `~/.local/bin` y lo agrega al arranque automático (minimizado);
2. activa `Experimental = true` en `/etc/bluetooth/main.conf` (pide sudo, deja respaldo en `main.conf.bak`). Sin esto no funciona la conexión automática al abrir el estuche;
3. lo deja corriendo.

## Notas

- La app se abre por XWayland porque en Wayland GNOME no deja elegir la posición de las ventanas. Con `xwayland-native-scaling` activado se ve nítida. Para usar Wayland nativo: `LIBREPODS_WAYLAND=1 librepods` (la tarjeta saldría al centro).
- El cambio automático entre dispositivos de Apple (iCloud) no es posible en Linux.

## Licencia y créditos

Código original: [LibrePods](https://github.com/librepods-org/librepods) por Kavish Devar y colaboradores, bajo **GNU AGPL-3.0**. Esta versión modificada se distribuye bajo la misma licencia (ver `LICENSE`).
