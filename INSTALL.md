# Dev dependencies
```shell
# Ubuntu/Debian
sudo apt install libgtk-4-dev build-essential
sudo apt install libvte-2.91-gtk4-dev
sudo apt install libadwaita-1-dev
# Fedora
sudo yum install gtk4-devel
sudo yum install libadwaita-devel
sudo yum install vte291-gtk4-devel
```

# Compilation
```shell
cargo run --release
```

# Build release packages
```shell
# Generate .deb using https://github.com/kornelski/cargo-deb
cargo deb
# Generate .rpm using https://github.com/cat-in-136/cargo-generate-rpm
cargo build --release
strip -s target/release/ivyterm
cargo generate-rpm
```

# Build flatpak
```shell
# Generated cargo sources for flatpak-builder using 'flatpak-cargo-generator'
python3 ./flatpak/flatpak-cargo-generator.py ./Cargo.lock -o ./packaging/flatpak/cargo-sources.json
# Install required flatpak SDK and extensions
flatpak install --user org.gnome.Sdk//47 org.gnome.Platform//47  org.freedesktop.Sdk.Extension.rust-stable//24.08 org.freedesktop.Sdk.Extension.llvm18//24.08
# Build and install ivyTerm flatpak
flatpak-builder --install repo packaging/flatpak/com.tomiyou.ivyTerm.json --user -y
```
