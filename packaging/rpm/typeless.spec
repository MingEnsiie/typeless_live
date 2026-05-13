Name:           typeless
Version:        0.1.0
Release:        1%{?dist}
Summary:        AI voice input method (Wispr Flow-style)
License:        MIT
URL:            https://github.com/MingEnsiie/typeless_live
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  alsa-lib-devel
BuildRequires:  libxdo-devel
BuildRequires:  pkgconf

Requires:       alsa-lib
Requires:       libxdo

%description
Typeless 是一个 Wispr Flow 风格的 AI 语音输入法守护进程：
本地 Whisper 转写 + 云端 / 本地 LLM 后处理（去口癖、加标点、纠错），
支持全局热键、SQLite 历史、词典、隐私模式与多语言。

%prep
%setup -q

%build
cargo build --release --bin typeless-cli

%install
mkdir -p %{buildroot}%{_bindir}
install -m 0755 target/release/typeless-cli %{buildroot}%{_bindir}/typeless-cli
mkdir -p %{buildroot}%{_userunitdir}
install -m 0644 packaging/systemd/typeless.service %{buildroot}%{_userunitdir}/

%files
%license LICENSE
%doc README.md
%{_bindir}/typeless-cli
%{_userunitdir}/typeless.service

%changelog
* Wed May 13 2026 Typeless Contributors - 0.1.0-1
- Initial RPM packaging.
