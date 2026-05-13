# Typeless 打包指南

## Debian / Ubuntu (.deb)

```bash
cargo install cargo-deb
cargo deb -p typeless-cli
# 输出 → target/debian/typeless-cli_0.1.0_amd64.deb
sudo dpkg -i target/debian/typeless-cli_*.deb
```

元数据见 `crates/typeless-cli/Cargo.toml` 的 `[package.metadata.deb]`。

## Fedora / RHEL (.rpm)

```bash
# 准备 source tarball
git archive --format=tar.gz --prefix=typeless-0.1.0/ -o ~/rpmbuild/SOURCES/typeless-0.1.0.tar.gz HEAD
cp packaging/rpm/typeless.spec ~/rpmbuild/SPECS/
rpmbuild -ba ~/rpmbuild/SPECS/typeless.spec
```

## Flatpak

```bash
flatpak install --user flathub org.freedesktop.Platform//23.08 org.freedesktop.Sdk//23.08 \
                                org.freedesktop.Sdk.Extension.rust-stable//23.08
flatpak-builder --user --install --force-clean build-dir \
    packaging/flatpak/io.github.mingensiie.Typeless.yaml
flatpak run io.github.mingensiie.Typeless
```

## systemd --user

见 `packaging/systemd/README.md`。
