# Dependencies
```shell
sudo apt install libgtk-4-dev build-essential
sudo apt install libvte-2.91-gtk4-dev
sudo apt install libadwaita-1-dev
```

# Compilation
```shell
cargo run --release
```

# Build flatpak
```shell
# Generated cargo sources for flatpak-builder using 'flatpak-cargo-generator'
python3 ./flatpak/flatpak-cargo-generator.py ./Cargo.lock -o ./flatpak/cargo-sources.json
# Install required flatpak SDK and extensions
flatpak install --user org.gnome.Sdk//47 org.gnome.Platform//47  org.freedesktop.Sdk.Extension.rust-stable//24.08 org.freedesktop.Sdk.Extension.llvm18//24.08
# Build and install ivyTerm flatpak
flatpak-builder --install repo flatpak/com.tomiyou.ivyTerm.json --user -y
```
