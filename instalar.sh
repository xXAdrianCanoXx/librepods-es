#!/bin/bash
# Instala LibrePods (versión modificada en español) y lo deja arrancando solo.
set -e
BIN="$HOME/librepods-es/linux-rust/target/release/librepods"
[ -x "$BIN" ] || { echo "Falta compilar: cd ~/librepods-es/linux-rust && cargo build --release"; exit 1; }

mkdir -p ~/.local/bin ~/.config/autostart ~/.local/share/applications ~/.local/share/icons/hicolor/256x256/apps
pkill -x librepods 2>/dev/null || true
cp "$BIN" ~/.local/bin/librepods
cp ~/librepods-es/linux-rust/assets/icon.png ~/.local/share/icons/hicolor/256x256/apps/librepods.png

cat > ~/.local/share/applications/librepods.desktop <<EOF
[Desktop Entry]
Type=Application
Name=LibrePods
Comment=Batería y modos de los AirPods
Exec=$HOME/.local/bin/librepods
Icon=librepods
Categories=Audio;Utility;
EOF

cat > ~/.config/autostart/librepods.desktop <<EOF
[Desktop Entry]
Type=Application
Name=LibrePods
Exec=$HOME/.local/bin/librepods --start-minimized
Icon=librepods
X-GNOME-Autostart-enabled=true
EOF
echo "✓ Instalado y con arranque automático"

# Conexión automática al abrir el estuche: BlueZ necesita las funciones "Experimental"
if grep -q '^Experimental *= *true' /etc/bluetooth/main.conf; then
    echo "✓ BlueZ Experimental ya estaba activo"
else
    echo "Activando BlueZ Experimental (pide contraseña)…"
    sudo cp /etc/bluetooth/main.conf /etc/bluetooth/main.conf.bak
    sudo sed -i 's/^#\?Experimental *= *false/Experimental = true/' /etc/bluetooth/main.conf
    grep -q '^Experimental *= *true' /etc/bluetooth/main.conf || \
        sudo sed -i '/^\[General\]/a Experimental = true' /etc/bluetooth/main.conf
    sudo systemctl restart bluetooth
    echo "✓ Bluetooth reiniciado (los AirPods se reconectan en unos segundos)"
    sleep 4
fi

setsid -f ~/.local/bin/librepods --start-minimized >/dev/null 2>&1
echo "✓ LibrePods corriendo — busca el ícono arriba a la derecha"
