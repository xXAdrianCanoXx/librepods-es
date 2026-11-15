// use ksni::TrayMethods; // provides the spawn method

use ab_glyph::{Font, ScaleFont};
use ksni::{Icon, ToolTip};
use tokio::sync::mpsc::UnboundedSender;

use crate::bluetooth::aacp::{BatteryStatus, ControlCommandIdentifiers};
use crate::ui::messages::BluetoothUIMessage;
use crate::utils::get_app_settings_path;

#[derive(Debug)]
pub struct MyTray {
    pub conversation_detect_enabled: Option<bool>,
    pub battery_headphone: Option<u8>,
    pub battery_headphone_status: Option<BatteryStatus>,
    pub battery_l: Option<u8>,
    pub battery_l_status: Option<BatteryStatus>,
    pub battery_r: Option<u8>,
    pub battery_r_status: Option<BatteryStatus>,
    pub battery_c: Option<u8>,
    pub battery_c_status: Option<BatteryStatus>,
    pub connected: bool,
    pub listening_mode: Option<u8>,
    pub allow_off_option: Option<u8>,
    pub command_tx: Option<UnboundedSender<(ControlCommandIdentifiers, Vec<u8>)>>,
    pub ui_tx: Option<UnboundedSender<BluetoothUIMessage>>,
    /// MAC de los AirPods (últimos conectados o guardados)
    pub mac: Option<String>,
}

impl MyTray {
    fn device_name(&self) -> String {
        let Some(mac) = &self.mac else {
            return "AirPods".into();
        };
        std::fs::read_to_string(crate::utils::get_devices_path())
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get(mac)?.get("name")?.as_str().map(String::from))
            .filter(|n| !n.contains(':'))
            .unwrap_or_else(|| "AirPods".into())
    }

    fn battery_line(&self) -> String {
        let fmt = |label: &str, level: Option<u8>, status: Option<BatteryStatus>| -> Option<String> {
            if status == Some(BatteryStatus::Disconnected) {
                return None;
            }
            let level = level?;
            let rayo = if status == Some(BatteryStatus::Charging) { "⚡" } else { "" };
            Some(format!("{} {}%{}", label, level, rayo))
        };
        let parts: Vec<String> = if self.battery_headphone.is_some() {
            [fmt("🎧", self.battery_headphone, self.battery_headphone_status)]
                .into_iter().flatten().collect()
        } else {
            [
                fmt("Izq", self.battery_l, self.battery_l_status),
                fmt("Der", self.battery_r, self.battery_r_status),
                fmt("Estuche", self.battery_c, self.battery_c_status),
            ]
            .into_iter().flatten().collect()
        };
        if parts.is_empty() { "Batería: —".into() } else { parts.join("  ·  ") }
    }
}

fn bluetoothctl(cmd: &'static str, mac: String) {
    std::thread::spawn(move || {
        let ok = std::process::Command::new("bluetoothctl")
            .args([cmd, &mac])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !ok && cmd == "connect" {
            crate::bluetooth::pairing::notify(
                "No se pudieron conectar",
                "Abre el estuche cerca de la laptop e inténtalo de nuevo.",
            );
        }
    });
}

impl ksni::Tray for MyTray {
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").into()
    }
    fn title(&self) -> String {
        self.device_name()
    }
    fn icon_pixmap(&self) -> Vec<Icon> {
        let text = {
            let mut levels: Vec<u8> = Vec::new();
            if let Some(h) = self.battery_headphone {
                if self.battery_headphone_status != Some(BatteryStatus::Disconnected) {
                    levels.push(h);
                }
            } else {
                if let Some(l) = self.battery_l
                    && self.battery_l_status != Some(BatteryStatus::Disconnected)
                {
                    levels.push(l);
                }
                if let Some(r) = self.battery_r
                    && self.battery_r_status != Some(BatteryStatus::Disconnected)
                {
                    levels.push(r);
                }
                // if let Some(c) = self.battery_c {
                //     if self.battery_c_status != Some(BatteryStatus::Disconnected) {
                //         levels.push(c);
                //     }
                // }
            }
            let min_battery = levels.iter().min().copied();
            if let Some(b) = min_battery {
                format!("{}", b)
            } else {
                "?".to_string()
            }
        };
        let any_bud_charging = matches!(self.battery_l_status, Some(BatteryStatus::Charging))
            || matches!(self.battery_r_status, Some(BatteryStatus::Charging));
        let app_settings_path = get_app_settings_path();
        let settings = std::fs::read_to_string(&app_settings_path)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
        let text_mode = settings
            .clone()
            .and_then(|v| v.get("tray_text_mode").cloned())
            .and_then(|ttm| serde_json::from_value(ttm).ok())
            .unwrap_or(false);
        let icon = generate_icon(&text, text_mode, any_bud_charging);
        vec![icon]
    }
    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            icon_name: "".to_string(),
            icon_pixmap: vec![],
            title: self.device_name(),
            description: if self.connected { self.battery_line() } else { "Desconectados".into() },
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        // Clic izquierdo en el ícono: mostrar la tarjeta de batería
        if let Some(tx) = &self.ui_tx {
            let _ = tx.send(BluetoothUIMessage::ShowBatteryPopup);
        }
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        let mut items: Vec<ksni::MenuItem<Self>> = Vec::new();

        // Encabezado: nombre + estado
        items.push(
            StandardItem {
                label: format!(
                    "{}  —  {}",
                    self.device_name(),
                    if self.connected { "Conectados" } else { "Desconectados" }
                ),
                icon_name: "audio-headphones".into(),
                enabled: false,
                ..Default::default()
            }
            .into(),
        );
        if self.connected {
            items.push(
                StandardItem {
                    label: self.battery_line(),
                    enabled: false,
                    ..Default::default()
                }
                .into(),
            );
        }
        items.push(MenuItem::Separator);

        // Conectar / Desconectar
        if let Some(mac) = self.mac.clone() {
            let connected = self.connected;
            items.push(
                StandardItem {
                    label: if connected { "Desconectar".into() } else { "Conectar".into() },
                    icon_name: if connected {
                        "bluetooth-disabled-symbolic".into()
                    } else {
                        "bluetooth-active-symbolic".into()
                    },
                    activate: Box::new(move |_this: &mut Self| {
                        bluetoothctl(if connected { "disconnect" } else { "connect" }, mac.clone());
                    }),
                    ..Default::default()
                }
                .into(),
            );
        }

        // Modo de ruido (solo si están conectados)
        if self.connected && self.listening_mode.is_some() {
            let allow_off = self.allow_off_option == Some(0x01);
            let mut options = vec![
                ("Cancelación de ruido", 0x02u8),
                ("Transparencia", 0x03),
                ("Adaptativo", 0x04),
            ];
            if allow_off {
                options.insert(0, ("Apagado", 0x01));
            }
            let selected = self
                .listening_mode
                .and_then(|mode| options.iter().position(|&(_, val)| val == mode))
                .unwrap_or(0);
            let values: Vec<u8> = options.iter().map(|&(_, v)| v).collect();
            items.push(MenuItem::Separator);
            items.push(
                RadioGroup {
                    selected,
                    select: Box::new(move |this: &mut Self, current| {
                        if let Some(tx) = &this.command_tx {
                            let value = values.get(current).copied().unwrap_or(0x02);
                            let _ = tx.send((ControlCommandIdentifiers::ListeningMode, vec![value]));
                            this.listening_mode = Some(value);
                        }
                    }),
                    options: options
                        .into_iter()
                        .map(|(label, _)| RadioItem {
                            label: label.into(),
                            ..Default::default()
                        })
                        .collect(),
                }
                .into(),
            );
            items.push(
                CheckmarkItem {
                    label: "Bajar volumen al hablar".into(),
                    checked: self.conversation_detect_enabled.unwrap_or(false),
                    enabled: self.conversation_detect_enabled.is_some(),
                    activate: Box::new(|this: &mut Self| {
                        if let Some(tx) = &this.command_tx
                            && let Some(is_enabled) = this.conversation_detect_enabled
                        {
                            let new_state = !is_enabled;
                            let value = if !new_state { 0x02 } else { 0x01 };
                            let _ = tx.send((
                                ControlCommandIdentifiers::ConversationDetectConfig,
                                vec![value],
                            ));
                            this.conversation_detect_enabled = Some(new_state);
                        }
                    }),
                    ..Default::default()
                }
                .into(),
            );
        }

        items.push(MenuItem::Separator);
        if self.connected {
            items.push(
                StandardItem {
                    label: "Ver batería".into(),
                    icon_name: "battery-good-symbolic".into(),
                    activate: Box::new(|this: &mut Self| {
                        if let Some(tx) = &this.ui_tx {
                            let _ = tx.send(BluetoothUIMessage::ShowBatteryPopup);
                        }
                    }),
                    ..Default::default()
                }
                .into(),
            );
        }
        items.push(
            StandardItem {
                label: "Emparejar AirPods nuevos…".into(),
                icon_name: "list-add-symbolic".into(),
                activate: Box::new(|_| crate::bluetooth::pairing::start_pairing()),
                ..Default::default()
            }
            .into(),
        );
        items.push(
            StandardItem {
                label: "Más ajustes…".into(),
                icon_name: "preferences-system-symbolic".into(),
                activate: Box::new(|this: &mut Self| {
                    if let Some(tx) = &this.ui_tx {
                        let _ = tx.send(BluetoothUIMessage::OpenWindow);
                    }
                }),
                ..Default::default()
            }
            .into(),
        );
        items.push(
            StandardItem {
                label: "Salir".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|_| std::process::exit(0)),
                ..Default::default()
            }
            .into(),
        );
        items
    }
}

fn generate_icon(text: &str, text_mode: bool, charging: bool) -> Icon {
    use ab_glyph::{FontRef, PxScale};
    use image::{ImageBuffer, Rgba};
    use imageproc::drawing::draw_text_mut;

    let width = 64;
    let height = 64;

    let mut img = ImageBuffer::from_fn(width, height, |_, _| Rgba([0u8, 0u8, 0u8, 0u8]));

    let font_data = include_bytes!("../../assets/font/DejaVuSans.ttf");
    let font = match FontRef::try_from_slice(font_data) {
        Ok(f) => f,
        Err(_) => {
            return Icon {
                width: width as i32,
                height: height as i32,
                data: vec![0u8; (width * height * 4) as usize],
            };
        }
    };
    if !text_mode {
        let percentage = text.parse::<f32>().unwrap_or(0.0) / 100.0;

        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;
        let inner_radius = 22.0;
        let outer_radius = 28.0;

        // ring background
        for y in 0..height {
            for x in 0..width {
                let dx = x as f32 - center_x;
                let dy = y as f32 - center_y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist > inner_radius && dist <= outer_radius {
                    img.put_pixel(x, y, Rgba([128u8, 128u8, 128u8, 255u8]));
                }
            }
        }

        // ring
        for y in 0..height {
            for x in 0..width {
                let dx = x as f32 - center_x;
                let dy = y as f32 - center_y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist > inner_radius && dist <= outer_radius {
                    let angle = dy.atan2(dx);
                    let angle_from_top =
                        (angle + std::f32::consts::PI / 2.0).rem_euclid(2.0 * std::f32::consts::PI);
                    if angle_from_top <= percentage * 2.0 * std::f32::consts::PI {
                        img.put_pixel(x, y, Rgba([0u8, 255u8, 0u8, 255u8]));
                    }
                }
            }
        }
        if charging {
            let emoji = "⚡";
            let scale = PxScale::from(48.0);
            let color = Rgba([0u8, 255u8, 0u8, 255u8]);
            let scaled_font = font.as_scaled(scale);
            let mut emoji_width = 0.0;
            for c in emoji.chars() {
                let glyph_id = font.glyph_id(c);
                emoji_width += scaled_font.h_advance(glyph_id);
            }
            let x = ((width as f32 - emoji_width) / 2.0).max(0.0) as i32;
            let y = ((height as f32 - scale.y) / 2.0).max(0.0) as i32;
            draw_text_mut(&mut img, color, x, y, scale, &font, emoji);
        }
    } else {
        // battery text
        let scale = PxScale::from(48.0);
        let color = if charging {
            Rgba([0u8, 255u8, 0u8, 255u8])
        } else {
            Rgba([255u8, 255u8, 255u8, 255u8])
        };

        let scaled_font = font.as_scaled(scale);
        let mut text_width = 0.0;
        for c in text.chars() {
            let glyph_id = font.glyph_id(c);
            text_width += scaled_font.h_advance(glyph_id);
        }
        let x = ((width as f32 - text_width) / 2.0).max(0.0) as i32;
        let y = ((height as f32 - scale.y) / 2.0).max(0.0) as i32;

        draw_text_mut(&mut img, color, x, y, scale, &font, text);
    }

    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for pixel in img.pixels() {
        data.push(pixel[3]);
        data.push(pixel[0]);
        data.push(pixel[1]);
        data.push(pixel[2]);
    }

    Icon {
        width: width as i32,
        height: height as i32,
        data,
    }
}
