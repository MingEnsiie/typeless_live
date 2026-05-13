#!/usr/bin/env bash
# 下载 Whisper GGML 模型到 typeless 数据目录。
set -euo pipefail
MODEL="${1:-base}"
DEST="${XDG_DATA_HOME:-$HOME/.local/share}/typeless/models"
mkdir -p "$DEST"
URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-${MODEL}.bin"
echo "Downloading ggml-${MODEL}.bin → $DEST"
curl -L --progress-bar "$URL" -o "$DEST/ggml-${MODEL}.bin"
echo "✓ Done. Set asr.model = ggml-${MODEL}.bin"
