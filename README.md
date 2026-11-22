# Termian

SSH/SFTP-клиент на Rust для macOS, Windows и Android. Подключения и ключи хранятся в локальном Vault. Для синхронизации можно [запустить свой сервер](cloud-sync-server/README.md).

## Установка

В [GitHub Releases](https://github.com/hahatulk/termian/releases/latest) скачайте `.dmg` для macOS Apple Silicon, `-setup.exe` для Windows x64 или `.apk` для Android 8+. На панели задач Windows окна Termian используют значок из EXE.

Новые версии сначала публикуются как "Pre-release". В "Latest" они попадают после ручного перевода.

## Запуск из исходников

Нужен Rust stable. На macOS установите Xcode Command Line Tools и задайте `export MACOSX_DEPLOYMENT_TARGET=11.0`. На Windows установите MSVC и Windows SDK.

```bash
git clone https://github.com/hahatulk/termian.git
cd termian
cargo run --locked --bin termian-native
```

[Сборка Android](spec.md#92-android)

## Скриншоты

<table>
  <tr>
    <td align="center"><a href="https://github.com/user-attachments/assets/4d2edd91-a4ea-47c8-a944-ab96992bca8e"><img src="https://github.com/user-attachments/assets/4d2edd91-a4ea-47c8-a944-ab96992bca8e" alt="Терминал" width="400"></a><br>Терминал</td>
    <td align="center"><a href="https://github.com/user-attachments/assets/bf0001af-9abf-4fce-95f3-3d3bd9c47117"><img src="https://github.com/user-attachments/assets/bf0001af-9abf-4fce-95f3-3d3bd9c47117" alt="Vault и редактор хоста" width="400"></a><br>Vault и редактор хоста</td>
  </tr>
  <tr>
    <td align="center"><a href="https://github.com/user-attachments/assets/c30cb794-2966-485e-a2a1-67dbfa10b30c"><img src="https://github.com/user-attachments/assets/c30cb794-2966-485e-a2a1-67dbfa10b30c" alt="Настройки синхронизации" width="400"></a><br>Настройки синхронизации</td>
    <td align="center"><a href="https://github.com/user-attachments/assets/835bfa61-1e41-4c76-b017-0724b1a12ee9"><img src="https://github.com/user-attachments/assets/835bfa61-1e41-4c76-b017-0724b1a12ee9" alt="Панель сервера" width="300"></a><br>Панель сервера</td>
  </tr>
  <tr>
    <td align="center" colspan="2"><a href="https://github.com/user-attachments/assets/de8ee1d1-95ec-4f90-a13a-d6150da6936f"><img src="https://github.com/user-attachments/assets/de8ee1d1-95ec-4f90-a13a-d6150da6936f" alt="Каналы обновлений" width="400"></a><br>Каналы обновлений</td>
    <td></td>
  </tr>
</table>

[Автообновление](spec.md#46-автообновление) · [Спецификация](spec.md) · [Сообщить об ошибке](https://github.com/hahatulk/termian/issues)
