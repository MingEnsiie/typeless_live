# 构建 Tauri 桌面应用

## Linux 系统依赖

```bash
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev libasound2-dev libxdo-dev pkg-config
```

## 启动开发

```bash
cd apps/desktop
npm install
npm run tauri dev
```

## 启用本地 Whisper

```bash
./scripts/download-models.sh base
cd apps/desktop && npm run tauri dev -- --features whisper
```
