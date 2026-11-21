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

## Первое подключение

В Vaults → Hosts нажмите "NEW HOST", укажите адрес, имя пользователя и пароль или SSH-ключ. Нажмите на сохранённый хост один раз, чтобы подключиться. В настольном терминале выделение мышью автоматически копирует текст и показывает "Copied!". Чтобы выделить текст через несколько экранов, держите левую кнопку и ведите курсор к верхнему или нижнему краю текста. Можно прокручивать колесом или трекпадом, не отпуская кнопку. Чтобы выбрать дальнюю границу без перетаскивания, щёлкните в начале диапазона, прокрутите историю и щёлкните по концу с `Shift`.

При подключении к SSH в настольном терминале кольцо движется по трём точкам: начало подключения, проверка ключа и вход. После входа полоса становится зелёной, затем открывается терминал.

В `htop`, `tmux` и других программах с включённым управлением мышью клики и прокрутка идут в программу. Удерживайте Shift, чтобы выделить текст или прокрутить историю терминала на компьютере. На Android касание ячейки передаёт клик, свайп передаёт прокрутку.

Когда терминал в фокусе, `Ctrl+C`, `Ctrl+V`, `Ctrl+Tab` и `F1`–`F12` передаются программе в сеансе. Для буфера обмена откройте меню вкладки правым кликом. На macOS в схеме "Mac hotkeys" доступны `Cmd+C` и `Cmd+V`, на Android вставка есть в меню клавиш терминала.

Если macOS не подключается к локальному SSH-хосту, проверьте доступ Termian в Системных настройках → Конфиденциальность и безопасность → Локальная сеть.

Нажмите имя Vault в боковой панели, чтобы открыть список и выбрать хранилище. На компьютере новый Vault создаётся через "New Vault". Существующие данные открываются в "Personal". Для всех Vault укажите один сервер, токен и пароль шифрования в Settings → Account. После обычной синхронизации список Vault и их данные доступны на других устройствах с теми же настройками.

Настройки приложения общие для всех Vault на устройстве. Если сменить тему, шрифт или вид списка в одном Vault, другой откроется с теми же настройками. Для хоста можно оставить собственную тему. Хосты, идентификаторы, ключи и журналы каждого Vault хранятся отдельно.

Чтобы переместить сохранённый хост, откройте "Host Details" → "General" → "Vault" и выберите другое хранилище. На Android после выбора нажмите "Save". Подтвердите перемещение: хост появится в корне выбранного Vault. Группу можно назначить там. Журналы и доверенные ключи сервера остаются в исходном Vault.

В настольном SFTP включите "Show Hidden Files" в меню действий. Имена скрытых файлов и папок серее, на папках есть значок перечёркнутого глаза. На Android нажмите на текстовый файл в SFTP, чтобы читать его страницами без сохранения копии.

На Android откройте Settings в левом меню. Разделы идут в том же порядке, что на компьютере. В Terminal → Terminal theme и в редакторе хоста тема выбирается из списка с превью.

На Android при запуске и после команды "Lock" подтвердите доступ отпечатком или паролем, PIN-кодом либо графическим ключом экрана блокировки. Если экран блокировки не настроен, откройте настройки безопасности Android с экрана входа. На Android 8 используется системный экран блокировки; встроенный запрос биометрии доступен с Android 9.

Язык интерфейса: Settings → Language. По умолчанию выбран English.

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
