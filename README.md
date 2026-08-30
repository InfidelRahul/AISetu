# AISetu

One React Native UI. Four OS families:

| Platform | How it runs |
|----------|-------------|
| **Windows** | Electron (`npm run dist:win`) — full gateway + window |
| **macOS** | Electron (`npm run dist:mac`) |
| **Linux** | Electron (`npm run dist:linux`) or this web desktop shell |
| **Android** | Expo / React Native (`npm run android` or EAS). Talks to the desktop gateway. |
| **iOS** | Same app (`npm run ios`) if you have Xcode |

The phone is not a second AI model. It is the same app, pointed at the PC that hosts the local OpenAI API (`Settings → API host`). Emulator: `http://10.0.2.2:8787`. Device: `http://YOUR_LAN_IP:8787`.

```bash
./build.sh           # tests + web UI + portable app in release/
./build.sh all       # also Electron + Android export when tools exist
./build.sh win|mac|linux|android
```

Output: `release/aisetu-*-portable/` (run `./AISetu` or `AISetu.cmd`), `release/web/`, optional Electron installers.
